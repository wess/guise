//! The release check: fetch the latest published release, compare it against
//! the running version, and decide whether it is something this machine can
//! actually install.
//!
//! That last part is load-bearing. A GitHub release is created and published
//! *before* CI finishes building and uploading its assets, so for however long
//! a notarization run takes, `releases/latest` reports a version whose only
//! uploaded files may be for another platform. Offering that release produces an
//! Update button whose sole possible outcome is "no asset for this platform" —
//! which is exactly what a prompt should never do. [`Release::ready_for`] is
//! what holds it back until the artifact exists.

use super::install::InstallKind;
use super::json;
use super::{fetch, semver};

/// Where to look for releases.
///
/// Both forms expect the shape of GitHub's release API (`tag_name`, `html_url`,
/// and an `assets` array of `name` / `browser_download_url` / `size`), because
/// that is the format the overwhelming majority of desktop apps already publish.
/// Point [`UpdateSource::url`] at your own endpoint to serve the same JSON from
/// somewhere else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateSource {
  /// GitHub's `releases/latest` for an `owner/repo` slug.
  GitHub(String),
  /// A URL answering with the same JSON as GitHub's release API.
  Url(String),
}

impl UpdateSource {
  /// Releases published to a GitHub repo, given as `owner/repo`.
  pub fn github(repo: impl Into<String>) -> Self {
    UpdateSource::GitHub(repo.into())
  }

  /// A custom endpoint serving GitHub-shaped release JSON.
  pub fn url(url: impl Into<String>) -> Self {
    UpdateSource::Url(url.into())
  }

  /// The URL to fetch the latest release from.
  pub fn endpoint(&self) -> String {
    match self {
      UpdateSource::GitHub(repo) => {
        format!("https://api.github.com/repos/{repo}/releases/latest")
      }
      UpdateSource::Url(url) => url.clone(),
    }
  }
}

/// One uploaded release asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseAsset {
  /// The uploaded file name, e.g. `Acme-1.27.8-aarch64.AppImage`.
  pub name: String,
  /// Direct download URL.
  pub url: String,
  /// Byte size as the feed reports it. Drives the download progress bar and
  /// the truncation check after the download; 0 when the field is absent.
  pub size: u64,
}

/// A published release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
  /// Semver without the leading `v` (e.g. `1.25.0`).
  pub version: String,
  /// The release page URL — where "Release Notes" and the download fallback go.
  pub url: String,
  /// Every uploaded asset.
  pub assets: Vec<ReleaseAsset>,
}

/// The outcome of a check. [`UpdateCheck::Pending`] exists so a manual "Check
/// for Updates…" can say "still building" instead of the flat lie that you are
/// up to date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateCheck {
  /// Nothing newer is published.
  UpToDate,
  /// A newer release exists, but it hasn't uploaded anything this machine can
  /// use yet. Carries the version so the UI can name it.
  Pending(String),
  /// A newer release with the asset this install needs.
  Ready(Release),
}

/// Whether `name` is built for `arch`. Release artifacts spell architectures
/// inconsistently by design: `cargo-deb` writes Debian names (`arm64`, `amd64`)
/// while tarballs and AppImages carry the Rust/uname spelling.
fn matches_arch(name: &str, arch: &str) -> bool {
  let aliases: &[&str] = match arch {
    "aarch64" => &["aarch64", "arm64"],
    "x86_64" => &["x86_64", "amd64"],
    other => &[other],
  };
  aliases.iter().any(|alias| contains_token(name, alias))
}

/// Whether `name` contains `token` as a whole architecture field rather than as
/// a bare substring. [`std::env::consts::ARCH`] is `"x86"` on 32-bit x86 and
/// `"arm"` on 32-bit ARM, both substrings of the 64-bit asset names — so a loose
/// test would have an i686 install match the `x86_64` image and rename it over
/// itself, the very clobber arch matching exists to stop.
///
/// A trailing `_` does *not* end the token, because `_` continues one
/// (`x86_64`); a leading one does, because that is how `cargo-deb` delimits
/// fields (`acme_1.27.8_arm64.deb`).
fn contains_token(name: &str, token: &str) -> bool {
  let bytes = name.as_bytes();
  name.match_indices(token).any(|(i, _)| {
    let before = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
    let end = i + token.len();
    let after = end == bytes.len() || (!bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_');
    before && after
  })
}

/// The first asset whose name ends in `suffix`, optionally restricted to assets
/// built for `arch`. A checksum sidecar contains the artifact extension too,
/// but must never be selected as the artifact itself.
fn pick<'a>(
  assets: &'a [ReleaseAsset],
  suffix: &str,
  arch: Option<&str>,
) -> Option<&'a ReleaseAsset> {
  assets
    .iter()
    .find(|a| a.name.ends_with(suffix) && arch.is_none_or(|x| matches_arch(&a.name, x)))
}

impl Release {
  /// The download this install would fetch, if the release has published it.
  /// `None` for [`InstallKind::Unknown`], which has no in-place path at all.
  pub fn asset_for(&self, kind: &InstallKind) -> Option<&ReleaseAsset> {
    match kind {
      InstallKind::MacApp(_) => {
        pick(&self.assets, ".dmg", None).or_else(|| pick(&self.assets, ".zip", None))
      }
      InstallKind::AppImage(_) => pick(&self.assets, ".AppImage", Some(std::env::consts::ARCH)),
      InstallKind::Unknown => None,
    }
  }

  /// The asset publishing `asset`'s SHA-256, if the release ships one.
  ///
  /// Recognises the two conventions in the wild: a per-asset digest file
  /// (`Acme.AppImage.sha256`) and a listing covering the whole release
  /// (`SHA256SUMS`, `checksums.txt`). A per-asset file wins, because it is
  /// unambiguous about what it covers.
  pub fn checksum_for(&self, asset: &ReleaseAsset) -> Option<&ReleaseAsset> {
    let per_asset = [
      format!("{}.sha256", asset.name),
      format!("{}.sha256sum", asset.name),
      format!("{}.SHA256", asset.name),
    ];
    if let Some(found) = self.assets.iter().find(|a| per_asset.contains(&a.name)) {
      return Some(found);
    }
    self.assets.iter().find(|a| is_generic_checksum(&a.name))
  }

  /// Whether this release has finished publishing what this machine needs.
  ///
  /// For an in-place install that means the exact asset. For everything else
  /// the action is "open the download page", which needs no particular
  /// artifact — so the only thing worth waiting for is the release having
  /// *any* asset at all. Gating those on a per-OS asset instead would strand
  /// anyone whose platform you don't publish for (a source build on riscv64,
  /// say) on "still building" forever, never reaching the page fallback that
  /// [`InstallKind::Unknown`] exists to provide.
  pub fn ready_for(&self, kind: &InstallKind) -> bool {
    match kind {
      InstallKind::Unknown => !self.assets.is_empty(),
      _ => self.asset_for(kind).is_some(),
    }
  }
}

pub(crate) fn is_generic_checksum(name: &str) -> bool {
  matches!(
    name.to_ascii_lowercase().as_str(),
    "sha256sums" | "sha256sums.txt" | "checksums.txt" | "checksums.sha256"
  )
}

/// Parse a release feed body into a [`Release`].
fn parse(body: &[u8]) -> Result<Release, String> {
  let value = json::parse(body).map_err(|e| format!("parse release: {e}"))?;
  let tag = value
    .get("tag_name")
    .and_then(|v| v.as_str())
    .ok_or("release has no tag")?;
  let version = tag.trim_start_matches('v').to_string();
  // The version becomes a path component of the staging directory, and
  // `semver::parse` only reads the leading three fields — it would happily
  // accept `1.28.0-/../../..`, which `create_dir_all` then resolves out of
  // $TMPDIR. Nothing anyone ships tags that way, so refuse it rather than
  // sanitize it.
  if !version
    .split('.')
    .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
  {
    return Err(format!("refusing malformed release tag `{tag}`"));
  }
  let assets = value
    .get("assets")
    .and_then(|v| v.as_array())
    .map(|items| {
      items
        .iter()
        .filter_map(|item| {
          Some(ReleaseAsset {
            name: item.get("name")?.as_str()?.to_string(),
            url: item.get("browser_download_url")?.as_str()?.to_string(),
            size: item.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
          })
        })
        .collect()
    })
    .unwrap_or_default();
  let url = value
    .get("html_url")
    .and_then(|v| v.as_str())
    .unwrap_or_default()
    .to_string();
  Ok(Release {
    version,
    url,
    assets,
  })
}

/// Fetch the latest published release and classify it against the running
/// version and this install. Blocking (spawns `curl`) — run it off the UI
/// thread; [`super::Updater`] does that for you.
pub(crate) fn check(
  source: &UpdateSource,
  user_agent: &str,
  current: &str,
  kind: &InstallKind,
) -> Result<UpdateCheck, String> {
  let release = parse(&fetch::bytes(&source.endpoint(), user_agent)?)?;
  if !semver::is_newer(&release.version, current) {
    return Ok(UpdateCheck::UpToDate);
  }
  if !release.ready_for(kind) {
    return Ok(UpdateCheck::Pending(release.version));
  }
  Ok(UpdateCheck::Ready(release))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  /// A trimmed GitHub `releases/latest` response, with both AppImage
  /// architectures so arch matching is exercised.
  const BODY: &str = r#"{
        "tag_name": "v1.26.0",
        "html_url": "https://github.com/acme/acme/releases/tag/v1.26.0",
        "assets": [
            {"name": "Acme.dmg", "browser_download_url": "https://d/Acme.dmg", "size": 87357960},
            {"name": "acme_1.26.0_amd64.deb", "browser_download_url": "https://d/deb", "size": 11977580},
            {"name": "Acme-1.26.0-x86_64.AppImage", "browser_download_url": "https://d/intel", "size": 4},
            {"name": "Acme-1.26.0-aarch64.AppImage", "browser_download_url": "https://d/arm", "size": 3}
        ]
    }"#;

  fn mac() -> InstallKind {
    InstallKind::MacApp(PathBuf::from("/Applications/Acme.app"))
  }

  fn appimage() -> InstallKind {
    InstallKind::AppImage(PathBuf::from("/opt/Acme.AppImage"))
  }

  #[test]
  fn release_json_parses() {
    let release = parse(BODY.as_bytes()).unwrap();
    assert_eq!(release.version, "1.26.0");
    assert_eq!(
      release.url,
      "https://github.com/acme/acme/releases/tag/v1.26.0"
    );
    assert_eq!(release.assets.len(), 4);
    assert_eq!(release.assets[0].size, 87_357_960);
  }

  #[test]
  fn tagless_body_is_an_error() {
    assert!(parse(br#"{"assets": []}"#).is_err());
    assert!(parse(b"not json").is_err());
  }

  #[test]
  fn missing_asset_fields_are_skipped() {
    let release = parse(br#"{"tag_name": "v9.9.9", "assets": [{"name": "x"}]}"#).unwrap();
    assert!(release.assets.is_empty());
    assert!(release.url.is_empty());
  }

  #[test]
  fn absent_size_field_defaults_to_zero() {
    let body = br#"{"tag_name": "v9.9.9", "assets":
            [{"name": "Acme.dmg", "browser_download_url": "https://d/x"}]}"#;
    assert_eq!(parse(body).unwrap().assets[0].size, 0);
  }

  #[test]
  fn mac_installs_take_the_universal_dmg() {
    let release = parse(BODY.as_bytes()).unwrap();
    assert_eq!(release.asset_for(&mac()).unwrap().url, "https://d/Acme.dmg");
  }

  #[test]
  fn mac_installs_fall_back_to_a_zip() {
    let release = Release {
      version: "1.26.0".to_string(),
      url: String::new(),
      assets: vec![ReleaseAsset {
        name: "Acme.zip".to_string(),
        url: "https://d/Acme.zip".to_string(),
        size: 1,
      }],
    };
    assert_eq!(release.asset_for(&mac()).unwrap().url, "https://d/Acme.zip");
    assert!(release.ready_for(&mac()));
  }

  #[test]
  fn mac_installs_prefer_a_dmg_over_a_zip() {
    let release = Release {
      version: "1.26.0".to_string(),
      url: String::new(),
      assets: vec![
        ReleaseAsset {
          name: "Acme.zip".to_string(),
          url: "https://d/Acme.zip".to_string(),
          size: 1,
        },
        ReleaseAsset {
          name: "Acme.dmg".to_string(),
          url: "https://d/Acme.dmg".to_string(),
          size: 1,
        },
      ],
    };
    assert_eq!(release.asset_for(&mac()).unwrap().url, "https://d/Acme.dmg");
  }

  #[test]
  fn checksum_sidecars_are_not_install_assets() {
    let release = Release {
      version: "1.26.0".to_string(),
      url: String::new(),
      assets: vec![ReleaseAsset {
        name: "Acme.zip.sha256".to_string(),
        url: "https://d/Acme.zip.sha256".to_string(),
        size: 1,
      }],
    };
    assert!(release.asset_for(&mac()).is_none());
  }

  #[test]
  fn appimage_picks_the_running_architecture() {
    // The fixture deliberately lists x86_64 *before* aarch64. Without arch
    // matching, `pick` returns the first ".AppImage" it sees, so on an
    // aarch64 host this assertion is what separates "matched my arch" from
    // "took whatever was listed first" — the bug being guarded is renaming
    // an image built for the other architecture over a working install.
    let release = parse(BODY.as_bytes()).unwrap();
    let want = if std::env::consts::ARCH == "aarch64" {
      "https://d/arm"
    } else {
      "https://d/intel"
    };
    assert_eq!(release.asset_for(&appimage()).unwrap().url, want);
  }

  #[test]
  fn debian_and_uname_arch_spellings_both_match() {
    assert!(matches_arch("acme_1.26.0_arm64.deb", "aarch64"));
    assert!(matches_arch("Acme-1.26.0-aarch64.AppImage", "aarch64"));
    assert!(matches_arch("acme_1.26.0_amd64.deb", "x86_64"));
    assert!(matches_arch("Acme-1.26.0-x86_64.AppImage", "x86_64"));
    assert!(!matches_arch("Acme-1.26.0-aarch64.AppImage", "x86_64"));
    assert!(!matches_arch("acme_1.26.0_amd64.deb", "aarch64"));
  }

  #[test]
  fn arch_tokens_do_not_match_as_bare_substrings() {
    // 32-bit `ARCH` values are substrings of the 64-bit asset names. Matching
    // loosely would let an i686 or armv7 install download a 64-bit image and
    // rename it over itself.
    assert!(!matches_arch("Acme-1.26.0-x86_64.AppImage", "x86"));
    assert!(!matches_arch("acme_1.26.0_arm64.deb", "arm"));
    assert!(!matches_arch("Acme-1.26.0-aarch64.AppImage", "arm"));
    // A genuine 32-bit artifact still matches its own name.
    assert!(matches_arch("Acme-1.26.0-x86.AppImage", "x86"));
    assert!(matches_arch("Acme-1.26.0-arm.AppImage", "arm"));
  }

  #[test]
  fn malformed_release_tags_are_refused() {
    // The version lands in the staging directory path, and version parsing
    // reads only the leading fields, so a tag carrying `..` would escape
    // $TMPDIR once `create_dir_all` resolved it.
    assert!(parse(br#"{"tag_name": "v1.28.0-/../../../../pwned", "assets": []}"#).is_err());
    assert!(parse(br#"{"tag_name": "v1.28.0/../x", "assets": []}"#).is_err());
    assert!(parse(br#"{"tag_name": "v1.28.0-beta1", "assets": []}"#).is_err());
    assert!(parse(br#"{"tag_name": "v1.28.0", "assets": []}"#).is_ok());
  }

  #[test]
  fn unknown_installs_have_no_in_place_asset() {
    let release = parse(BODY.as_bytes()).unwrap();
    assert!(release.asset_for(&InstallKind::Unknown).is_none());
  }

  #[test]
  fn a_release_with_our_asset_is_ready() {
    let release = parse(BODY.as_bytes()).unwrap();
    assert!(release.ready_for(&mac()));
    assert!(release.ready_for(&appimage()));
  }

  #[test]
  fn a_release_still_uploading_is_not_ready() {
    // The shape a real release takes when it goes live with only the Linux
    // artifacts while macOS notarization is still running. Offering this to
    // a Mac produces an Update button that can only ever fail.
    let body = br#"{"tag_name": "v1.27.8", "assets": [
            {"name": "Acme-1.27.8-aarch64.AppImage", "browser_download_url": "https://d/a", "size": 1},
            {"name": "acme_1.27.8_arm64.deb", "browser_download_url": "https://d/b", "size": 2}
        ]}"#;
    assert!(!parse(body).unwrap().ready_for(&mac()));
  }

  #[test]
  fn a_release_with_no_assets_at_all_is_not_ready() {
    let release = parse(br#"{"tag_name": "v9.9.9", "assets": []}"#).unwrap();
    assert!(!release.ready_for(&mac()));
    assert!(!release.ready_for(&appimage()));
    assert!(!release.ready_for(&InstallKind::Unknown));
  }

  #[test]
  fn installs_we_cannot_rewrite_are_never_stranded() {
    // `Unknown` only ever opens the download page, so it must not be gated
    // on an asset for this platform — a machine you publish nothing for
    // would sit on "still building" forever and never reach the page.
    let body = br#"{"tag_name": "v9.9.9", "assets": [
            {"name": "something-for-another-platform.tar.gz", "browser_download_url": "https://d/x", "size": 1}
        ]}"#;
    assert!(parse(body).unwrap().ready_for(&InstallKind::Unknown));
  }

  #[test]
  fn sources_resolve_to_their_endpoint() {
    assert_eq!(
      UpdateSource::github("acme/acme").endpoint(),
      "https://api.github.com/repos/acme/acme/releases/latest"
    );
    assert_eq!(
      UpdateSource::url("https://acme.dev/latest.json").endpoint(),
      "https://acme.dev/latest.json"
    );
  }

  #[test]
  fn a_per_asset_digest_beats_a_release_wide_listing() {
    let release = Release {
      version: "1.0.0".to_string(),
      url: String::new(),
      assets: vec![
        ReleaseAsset {
          name: "Acme-x86_64.AppImage".to_string(),
          url: "https://d/a".to_string(),
          size: 1,
        },
        ReleaseAsset {
          name: "SHA256SUMS".to_string(),
          url: "https://d/sums".to_string(),
          size: 1,
        },
        ReleaseAsset {
          name: "Acme-x86_64.AppImage.sha256".to_string(),
          url: "https://d/one".to_string(),
          size: 1,
        },
      ],
    };
    let asset = release.assets[0].clone();
    assert_eq!(
      release.checksum_for(&asset).map(|a| a.name.as_str()),
      Some("Acme-x86_64.AppImage.sha256")
    );
  }

  #[test]
  fn a_release_wide_listing_is_the_fallback() {
    let release = Release {
      version: "1.0.0".to_string(),
      url: String::new(),
      assets: vec![
        ReleaseAsset {
          name: "Acme.AppImage".to_string(),
          url: "https://d/a".to_string(),
          size: 1,
        },
        ReleaseAsset {
          name: "checksums.txt".to_string(),
          url: "https://d/sums".to_string(),
          size: 1,
        },
      ],
    };
    let asset = release.assets[0].clone();
    assert_eq!(
      release.checksum_for(&asset).map(|a| a.name.as_str()),
      Some("checksums.txt")
    );
  }

  #[test]
  fn a_release_with_no_digest_reports_none() {
    let release = Release {
      version: "1.0.0".to_string(),
      url: String::new(),
      assets: vec![ReleaseAsset {
        name: "Acme.AppImage".to_string(),
        url: "https://d/a".to_string(),
        size: 1,
      }],
    };
    let asset = release.assets[0].clone();
    assert!(release.checksum_for(&asset).is_none());
  }
}
