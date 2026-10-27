//! Self-update: check for a newer release, install it **in place**, and prompt
//! for it — the whole feature, from the release feed to the window that offers it.
//!
//! ```ignore
//! use guise::update::{start, Updater};
//!
//! let updater = Updater::github("Acme", env!("CARGO_PKG_VERSION"), "acme/acme")
//!     .codesign_requirement("anchor apple generic and certificate leaf[subject.OU] = TEAMID")
//!     .on_notify(|title, body| post_os_notification(title, body))
//!     .restart_guard(|cx| can_restart_without_losing_work(cx))
//!     .before_restart(|cx| save_session(cx));
//!
//! start(updater.clone(), cx);          // at launch, then hourly
//! check_now(updater, cx);              // the "Check for Updates…" menu item
//! ```
//!
//! Two halves, and they are usable apart:
//!
//! - **The engine** — [`UpdateConfig`] and the [`Release`] / [`InstallKind`] /
//!   [`UpdateStage`] / [`Relaunch`] vocabulary. No gpui, so it is all directly
//!   testable, and blocking, so it belongs on the background executor.
//! - **The UI** — [`UpdatePrompt`] and [`UpdateNotice`], ordinary guise entities.
//!   Render them in a window you own, or let [`open`] and [`check_now`] put them
//!   in their own.
//!
//! ## Installing in place is the load-bearing decision
//!
//! An install is never *replaced*, only *rewritten in place*:
//!
//! - **macOS**: the release `.dmg` is mounted and the new bundle's contents are
//!   `rsync --delete`d onto the installed `.app`. The bundle directory itself (its
//!   path *and* its inode) never changes, so LaunchServices' registration stays
//!   valid and the relaunch is [`Relaunch::Current`] — gpui's restart reopens the
//!   running bundle via `NSBundle`. A rename-swap (`.app` → `.app.old`, staged →
//!   `.app`) instead hands `open` a brand-new directory inode while deleting the
//!   running executable; when LaunchServices resolves the stale registration it
//!   can fall back to running the inner Mach-O as a plain executable — inside a
//!   terminal, unbundled and broken. Rewriting in place removes every ingredient
//!   of that failure.
//! - **Linux AppImage**: the new image is downloaded *next to* the running one (a
//!   rename across filesystems fails, and `/tmp` is often tmpfs) and renamed over
//!   it; the relaunch is [`Relaunch::Binary`] pointing at the image.
//! - Anything else ([`InstallKind::Unknown`]: a root-owned distro package, a dev
//!   build, Windows) can't be rewritten from inside the app — the prompt opens the
//!   release page instead, and says so on its button.
//!
//! A release is only ever offered once it has published the asset this machine
//! would install — see [`Release::ready_for`]. Release hosts publish the release
//! before CI uploads to it, so a newer tag can be visible for the length of a
//! notarization run with nothing on it we can use; prompting in that window yields
//! an Update button that can only fail.
//!
//! macOS installs additionally require an
//! [`codesign_requirement`](UpdateConfig::codesign_requirement) — without one
//! there is nothing to verify a downloaded bundle against, and the prompt falls
//! back to the download page rather than executing an unverified app as the user.

mod appimage;
mod checksum;
mod fetch;
mod install;
mod json;
mod mac;
mod notice;
mod prompt;
mod release;
mod semver;
mod service;
mod updater;

pub use install::{detect, InstallKind, Relaunch, UpdateStage};
pub use notice::{UpdateNotice, UpdateNoticeEvent, UpdateOutcome};
pub use prompt::{is_installing, UpdatePrompt, UpdatePromptEvent};
pub use release::{Release, ReleaseAsset, UpdateCheck, UpdateSource};
pub use semver::is_newer;
pub use service::{check_now, open, open_notice, start};
pub use updater::{UpdateConfig, Updater, POLL};
