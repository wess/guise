//! [`UpdateConfig`] — the gpui-free engine — and [`Updater`], the app-level
//! object the components and the poller are built from.
//!
//! The split is not decoration: the check and the install are blocking work that
//! runs on gpui's background executor, which requires everything it captures to
//! be `Send`. [`Updater`] carries `Rc` callbacks (notifications and restart
//! hooks) and so can never cross that boundary; [`UpdateConfig`] is the plain-data
//! half that can, and it is what the background task actually gets a copy of.

use std::rc::Rc;
use std::time::Duration;

use gpui::{App, SharedString};

use super::{InstallKind, Relaunch, Release, UpdateCheck, UpdateSource, UpdateStage};

/// How often to re-check while running (a conservative hourly cadence).
pub const POLL: Duration = Duration::from_secs(60 * 60);

/// Everything the blocking check and install need, and nothing that can't cross
/// onto a background thread. Build one through [`Updater`] unless you are driving
/// the mechanics yourself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateConfig {
  /// Display name, used in the prompt's copy ("Acme 1.2.3 is available").
  pub(crate) app: String,
  /// Filesystem-safe name, used for staging paths under `$TMPDIR`.
  pub(crate) slug: String,
  /// The running version, which every check compares against.
  pub(crate) version: String,
  /// Where releases are published.
  pub(crate) source: UpdateSource,
  /// Sent as `User-Agent` — GitHub's API rejects requests without one.
  pub(crate) user_agent: String,
  /// The macOS `codesign` requirement an update must satisfy.
  pub(crate) requirement: Option<String>,
  /// Refuse to install when the release publishes no SHA-256 to check the
  /// download against.
  pub(crate) require_checksum: bool,
}

impl UpdateConfig {
  /// A config for `app` at `version`, checking `source`.
  pub fn new(app: impl Into<String>, version: impl Into<String>, source: UpdateSource) -> Self {
    let app = app.into();
    let slug = slug(&app);
    let user_agent = format!("{slug}-updater");
    UpdateConfig {
      app,
      slug,
      version: version.into(),
      source,
      user_agent,
      requirement: None,
      require_checksum: false,
    }
  }

  /// The `codesign` requirement a macOS update must satisfy before it is
  /// installed — **without** this, macOS installs refuse to run in place and
  /// the prompt falls back to opening the download page.
  ///
  /// Pass the requirement text only; the `-R=` that `codesign` needs is added
  /// here. Pin your team, not a certificate: team IDs survive certificate
  /// renewals, so
  ///
  /// ```ignore
  /// .codesign_requirement("anchor apple generic and certificate leaf[subject.OU] = XJDC46F35X")
  /// ```
  ///
  /// keeps working when the signing cert rolls. The practical consequence is
  /// that an ad-hoc signed build — what CI produces when the signing secrets
  /// are absent — cannot self-update, and shouldn't be able to.
  pub fn codesign_requirement(mut self, requirement: impl Into<String>) -> Self {
    self.requirement = Some(requirement.into());
    self
  }

  /// Refuse to install an update whose release publishes no SHA-256.
  ///
  /// A published digest is always checked when it exists. This turns a
  /// *missing* one from a silent pass into a refusal, which is the setting
  /// you want on Linux: the AppImage path has no signature to fall back on,
  /// so without a digest the only thing vouching for the file that is about
  /// to be renamed over the running binary is the feed that named it.
  ///
  /// Publish `<asset>.sha256` beside each artifact, or a `SHA256SUMS`
  /// listing, and turn this on.
  pub fn require_checksum(mut self, require: bool) -> Self {
    self.require_checksum = require;
    self
  }

  /// Whether a missing checksum blocks an install.
  pub fn requires_checksum(&self) -> bool {
    self.require_checksum
  }

  /// Check a downloaded file against the digest the release published for it.
  ///
  /// Absent a published digest this is a pass unless
  /// [`require_checksum`](Self::require_checksum) is set — a release that
  /// never shipped checksums shouldn't become uninstallable the day this
  /// lands.
  pub(crate) fn verify_checksum(
    &self,
    release: &Release,
    asset: &super::ReleaseAsset,
    file: &std::path::Path,
  ) -> Result<(), String> {
    let Some(published) = release.checksum_for(asset) else {
      if self.require_checksum {
        return Err(format!(
          "this release publishes no SHA-256 for {} — refusing to install it",
          asset.name
        ));
      }
      return Ok(());
    };
    let body = super::fetch::bytes(&published.url, &self.user_agent)
      .map_err(|e| format!("could not fetch the published checksum: {e}"))?;
    let body = String::from_utf8_lossy(&body);
    let Some(expected) = super::checksum::find(&body, &asset.name) else {
      // A release-wide listing may legitimately cover only some artifacts.
      // Treat that exactly like a missing checksum unless the app opted into
      // strict checks. A per-asset sidecar, on the other hand, made an explicit
      // promise about this download, so malformed contents always fail.
      if !self.require_checksum && super::release::is_generic_checksum(&published.name) {
        return Ok(());
      }
      return Err(format!(
        "{} does not record a SHA-256 for {}",
        published.name, asset.name
      ));
    };
    let actual = super::checksum::of_file(file)?;
    if !super::checksum::matches(&expected, &actual) {
      return Err(format!(
        "{} does not match its published SHA-256 — refusing to install it",
        asset.name
      ));
    }
    Ok(())
  }

  /// Override the `User-Agent` sent with every request.
  pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
    self.user_agent = user_agent.into();
    self
  }

  /// Override the filesystem-safe name used for staging paths.
  pub fn slug(mut self, slug: impl Into<String>) -> Self {
    self.slug = slug.into();
    self
  }

  /// The app's display name.
  pub fn app(&self) -> &str {
    &self.app
  }

  /// The running version.
  pub fn version(&self) -> &str {
    &self.version
  }

  /// Where releases are published.
  pub fn source(&self) -> &UpdateSource {
    &self.source
  }

  /// How this copy was installed — see [`super::detect`].
  pub fn install_kind(&self) -> InstallKind {
    super::detect()
  }

  /// Fetch the latest release and classify it. **Blocking** (it spawns `curl`)
  /// — run it on gpui's background executor, or let [`super::start`] and
  /// [`super::check_now`] do that for you.
  pub fn check(&self) -> Result<UpdateCheck, String> {
    super::release::check(
      &self.source,
      &self.user_agent,
      &self.version,
      &self.install_kind(),
    )
  }

  /// Download the release and rewrite this install in place, returning how to
  /// relaunch. **Blocking** — run it off the UI thread. `on_stage` is called
  /// from the calling thread as the install progresses, including once per
  /// download sample.
  pub fn install(
    &self,
    release: &Release,
    kind: &InstallKind,
    on_stage: &dyn Fn(UpdateStage),
  ) -> Result<Relaunch, String> {
    match kind {
      InstallKind::MacApp(app) => super::mac::install(self, release, app, on_stage),
      InstallKind::AppImage(path) => super::appimage::install(self, release, path, on_stage),
      InstallKind::Unknown => Err("this install can't be updated in place".to_string()),
    }
  }

  /// Whether this release can be installed in place — the question the prompt's
  /// action button asks before it promises anything.
  ///
  /// All three halves matter. The install has to be one we can rewrite; the
  /// release has to have published the asset to do it with (a release still
  /// uploading its artifacts hasn't); and a macOS install needs a
  /// [`codesign_requirement`](Self::codesign_requirement) to verify the payload
  /// against. Any of them missing and the honest button is "Open Download".
  pub fn can_install(&self, release: &Release, kind: &InstallKind) -> bool {
    if !kind.is_in_place() || release.asset_for(kind).is_none() {
      return false;
    }
    !matches!(kind, InstallKind::MacApp(_)) || self.requirement.is_some()
  }
}

/// The app-level updater: an [`UpdateConfig`] plus the parts only the UI side
/// needs — the poll cadence, the update window's title, and hooks for posting a
/// notification, deciding whether a restart is currently safe, and saving
/// state before the app restarts.
///
/// ```ignore
/// let updater = Updater::github("Acme", env!("CARGO_PKG_VERSION"), "acme/acme")
///     .codesign_requirement("anchor apple generic and certificate leaf[subject.OU] = XJDC46F35X")
///     .restart_guard(|cx| can_quit_without_losing_work(cx))
///     .before_restart(|cx| save_session(cx));
/// guise::update::start(updater, cx);
/// ```
#[derive(Clone)]
pub struct Updater {
  config: UpdateConfig,
  poll: Duration,
  title: SharedString,
  notify: Option<NotifyHook>,
  restart_guard: Option<RestartGuard>,
  before_restart: Option<RestartHook>,
}

/// The app's notification hook, called with `(title, body)`.
type NotifyHook = Rc<dyn Fn(&str, &str)>;

/// The app's final say over whether an installed update may restart it now.
type RestartGuard = Rc<dyn Fn(&mut App) -> bool>;

/// The app's hook for the moment before the restart.
type RestartHook = Rc<dyn Fn(&mut App)>;

impl Updater {
  /// An updater for `app` at `version`, checking `source`.
  pub fn new(app: impl Into<String>, version: impl Into<String>, source: UpdateSource) -> Self {
    Updater::from_config(UpdateConfig::new(app, version, source))
  }

  /// An updater reading a GitHub repo's `releases/latest`, given `owner/repo`.
  pub fn github(
    app: impl Into<String>,
    version: impl Into<String>,
    repo: impl Into<String>,
  ) -> Self {
    Updater::new(app, version, UpdateSource::github(repo))
  }

  /// Wrap an existing [`UpdateConfig`].
  pub fn from_config(config: UpdateConfig) -> Self {
    Updater {
      config,
      poll: POLL,
      title: "Software Update".into(),
      notify: None,
      restart_guard: None,
      before_restart: None,
    }
  }

  /// See [`UpdateConfig::require_checksum`] — recommended for Linux installs,
  /// which have no signature to fall back on.
  pub fn require_checksum(mut self, require: bool) -> Self {
    self.config = self.config.require_checksum(require);
    self
  }

  /// See [`UpdateConfig::codesign_requirement`] — required for macOS installs.
  pub fn codesign_requirement(mut self, requirement: impl Into<String>) -> Self {
    self.config = self.config.codesign_requirement(requirement);
    self
  }

  /// Override the `User-Agent` sent with every request.
  pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
    self.config = self.config.user_agent(user_agent);
    self
  }

  /// Override the filesystem-safe name used for staging paths.
  pub fn slug(mut self, slug: impl Into<String>) -> Self {
    self.config = self.config.slug(slug);
    self
  }

  /// How often [`super::start`] re-checks while the app runs (default one hour).
  pub fn poll_every(mut self, every: Duration) -> Self {
    self.poll = every;
    self
  }

  /// Title for the windows [`super::check_now`] opens (default
  /// "Software Update", the platform convention).
  pub fn window_title(mut self, title: impl Into<SharedString>) -> Self {
    self.title = title.into();
    self
  }

  /// Called with `(title, body)` when the install starts, finishes without a
  /// window to restart into, or fails. guise never posts an OS notification
  /// itself — that needs an app's own bundle identity and permission state.
  pub fn on_notify(mut self, notify: impl Fn(&str, &str) + 'static) -> Self {
    self.notify = Some(Rc::new(notify));
    self
  }

  /// Called after the update is installed but before any pre-restart work.
  /// Return `false` when restarting would discard work. The prompt closes, the
  /// update stays installed, and it takes effect on the next normal launch.
  ///
  /// This is deliberately separate from [`before_restart`](Self::before_restart):
  /// the guard decides whether a restart may happen; the hook persists state
  /// only when it will.
  pub fn restart_guard(mut self, guard: impl Fn(&mut App) -> bool + 'static) -> Self {
    self.restart_guard = Some(Rc::new(guard));
    self
  }

  /// Called immediately before the app restarts into the new version — the
  /// place to persist a session, since the restart never goes through the
  /// normal quit path where an app would usually save.
  pub fn before_restart(mut self, hook: impl Fn(&mut App) + 'static) -> Self {
    self.before_restart = Some(Rc::new(hook));
    self
  }

  /// The `Send` half, for the background check and install.
  pub fn config(&self) -> &UpdateConfig {
    &self.config
  }

  /// The app's display name.
  pub fn app(&self) -> &str {
    self.config.app()
  }

  /// The running version.
  pub fn version(&self) -> &str {
    self.config.version()
  }

  /// The re-check cadence.
  pub fn poll(&self) -> Duration {
    self.poll
  }

  /// The update window's title.
  pub fn title(&self) -> &SharedString {
    &self.title
  }

  /// Post a notification through the app's hook, if it installed one.
  pub(crate) fn notify(&self, title: &str, body: &str) {
    if let Some(notify) = &self.notify {
      notify(title, body);
    }
  }

  /// Run the pre-restart hook, if the app installed one.
  pub(crate) fn run_before_restart(&self, cx: &mut App) {
    if let Some(hook) = &self.before_restart {
      hook(cx);
    }
  }

  /// Whether the app permits an immediate restart.
  pub(crate) fn can_restart(&self, cx: &mut App) -> bool {
    self.restart_guard.as_ref().is_none_or(|guard| guard(cx))
  }
}

/// A filesystem-safe name derived from the app's display name: lowercase, with
/// every run of anything else collapsed to a single `-`. Staging paths are built
/// from this, so a name like "My App 2.0" must not arrive with spaces in it.
fn slug(app: &str) -> String {
  let mut out = String::with_capacity(app.len());
  for ch in app.chars() {
    if ch.is_ascii_alphanumeric() {
      out.push(ch.to_ascii_lowercase());
    } else if !out.ends_with('-') {
      out.push('-');
    }
  }
  let trimmed = out.trim_matches('-');
  if trimmed.is_empty() {
    "app".to_string()
  } else {
    trimmed.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::update::ReleaseAsset;
  use std::path::PathBuf;

  fn release(names: &[&str]) -> Release {
    Release {
      version: "9.9.9".to_string(),
      url: "https://acme.dev/releases/9.9.9".to_string(),
      assets: names
        .iter()
        .map(|name| ReleaseAsset {
          name: name.to_string(),
          url: format!("https://d/{name}"),
          size: 1,
        })
        .collect(),
    }
  }

  fn config() -> UpdateConfig {
    UpdateConfig::new("Acme", "1.0.0", UpdateSource::github("acme/acme"))
  }

  #[test]
  fn slugs_are_path_safe() {
    assert_eq!(slug("Acme"), "acme");
    assert_eq!(slug("My App 2.0"), "my-app-2-0");
    assert_eq!(slug("  Spaced  "), "spaced");
    assert_eq!(slug("../../etc"), "etc");
    assert_eq!(slug("🚀"), "app");
    assert_eq!(slug(""), "app");
  }

  #[test]
  fn the_user_agent_defaults_to_the_slug() {
    assert_eq!(config().user_agent, "acme-updater");
    assert_eq!(
      UpdateConfig::new("My App", "1", UpdateSource::github("a/b")).user_agent,
      "my-app-updater"
    );
  }

  /// A macOS install with no requirement configured has nothing to verify the
  /// payload against, so it must not be offered as an in-place install.
  #[test]
  fn macos_needs_a_codesign_requirement_to_be_installable() {
    let mac = InstallKind::MacApp(PathBuf::from("/Applications/Acme.app"));
    let dmg = release(&["Acme.dmg"]);
    assert!(!config().can_install(&dmg, &mac));
    assert!(config()
      .codesign_requirement("anchor apple generic")
      .can_install(&dmg, &mac));
  }

  /// The AppImage path verifies by architecture match and size, not codesign,
  /// so it needs no requirement.
  #[test]
  fn appimage_is_installable_without_a_requirement() {
    let image = InstallKind::AppImage(PathBuf::from("/opt/Acme.AppImage"));
    let asset = format!("Acme-9.9.9-{}.AppImage", std::env::consts::ARCH);
    assert!(config().can_install(&release(&[&asset]), &image));
  }

  #[test]
  fn a_release_without_our_asset_is_not_installable() {
    let mac = InstallKind::MacApp(PathBuf::from("/Applications/Acme.app"));
    let config = config().codesign_requirement("anchor apple generic");
    assert!(!config.can_install(&release(&["Acme.AppImage"]), &mac));
    assert!(!config.can_install(&release(&[]), &mac));
  }

  #[test]
  fn unknown_installs_are_never_installable_in_place() {
    let config = config().codesign_requirement("anchor apple generic");
    let every_asset = release(&["Acme.dmg", "Acme.AppImage"]);
    assert!(!config.can_install(&every_asset, &InstallKind::Unknown));
    assert!(config
      .install(&every_asset, &InstallKind::Unknown, &|_| {})
      .is_err());
  }

  /// The Linux install path has no signature to fall back on, so a release
  /// that publishes no digest must be refusable outright.
  #[test]
  fn require_checksum_refuses_a_release_with_no_digest() {
    let release = crate::update::Release {
      version: "9.9.9".to_string(),
      url: String::new(),
      assets: vec![crate::update::ReleaseAsset {
        name: "Acme.AppImage".to_string(),
        url: "https://d/a".to_string(),
        size: 1,
      }],
    };
    let asset = release.assets[0].clone();
    let path = std::path::Path::new("/nonexistent/Acme.AppImage");

    // Off by default: a project that never shipped checksums keeps working.
    let lenient = config();
    assert!(lenient.verify_checksum(&release, &asset, path).is_ok());

    let strict = config().require_checksum(true);
    assert!(strict.requires_checksum());
    let err = strict
      .verify_checksum(&release, &asset, path)
      .expect_err("a missing digest must block the install");
    assert!(err.contains("publishes no SHA-256"), "{err}");
  }
}
