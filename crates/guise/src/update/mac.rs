//! macOS: prepare the release `.dmg` or `.zip` and rsync the new bundle's
//! contents onto the installed `.app` — never replace the bundle directory
//! itself.
//!
//! The "in place" is the point: the installed bundle keeps its path *and* its
//! directory inode, so LaunchServices' registration stays valid and the running
//! executable is only ever replaced-by-rename (its open inode lives on, which
//! macOS is fine with — it is in-place *modification* of a running binary that
//! gets a process killed). Swapping the whole bundle out from under
//! LaunchServices is what makes a relaunch fall back to running the bare Mach-O
//! inside Terminal.app: `open` on a path whose registration is stale resolves to
//! the inner executable, unbundled and broken.

use super::UpdateConfig;

#[cfg(target_os = "macos")]
pub(crate) fn install(
  config: &UpdateConfig,
  release: &crate::update::Release,
  app: &std::path::Path,
  on_stage: &dyn Fn(crate::update::UpdateStage),
) -> Result<crate::update::Relaunch, String> {
  use crate::update::{InstallKind, Relaunch, UpdateStage};
  use std::process::Command;

  /// Detach the mount on every exit path, success or error.
  struct Unmount(std::path::PathBuf);
  impl Drop for Unmount {
    fn drop(&mut self) {
      let _ = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(&self.0)
        .status();
    }
  }

  // An unverifiable payload is not installable. See
  // `UpdateConfig::codesign_requirement`: without a requirement to pin the
  // signing identity to, `--verify` would only prove the bundle matches its own
  // seal, so *any* validly signed app would pass — which is not the question
  // being asked of something about to be executed as the user.
  let requirement = config
    .requirement
    .as_deref()
    .ok_or("no codesign requirement is configured, so this update can't be verified")?;

  let asset = release
    .asset_for(&InstallKind::MacApp(app.to_path_buf()))
    .ok_or("this release hasn't published a macOS build yet")?;
  let dir = std::env::temp_dir().join(format!("{}-update-{}", config.slug, release.version));
  if dir.exists() {
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
  }
  std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let is_zip = asset.name.to_ascii_lowercase().ends_with(".zip");
  let archive = dir.join(if is_zip { "update.zip" } else { "update.dmg" });
  let total = asset.size;
  on_stage(UpdateStage::Downloading { done: 0, total });
  super::fetch::file(&asset.url, &archive, total, &config.user_agent, &|done| {
    on_stage(UpdateStage::Downloading { done, total })
  })?;

  // Belt and braces alongside the codesign check below: a digest catches a
  // swapped asset before the image is even mounted, and `hdiutil` parsing an
  // attacker-chosen file is a larger surface than a hash of it.
  on_stage(UpdateStage::Verifying);
  config.verify_checksum(release, asset, &archive)?;

  on_stage(UpdateStage::Preparing);
  let (prepared, unmount) = if is_zip {
    let extracted = dir.join("extracted");
    std::fs::create_dir_all(&extracted).map_err(|e| e.to_string())?;
    let expanded = Command::new("ditto")
      .args(["-x", "-k", "--sequesterRsrc", "--rsrc"])
      .arg(&archive)
      .arg(&extracted)
      .status()
      .map_err(|e| format!("ditto: {e}"))?;
    if !expanded.success() {
      return Err("could not expand the update archive".to_string());
    }
    (extracted, None)
  } else {
    let mount = dir.join("mnt");
    std::fs::create_dir_all(&mount).map_err(|e| e.to_string())?;
    let attach = Command::new("hdiutil")
      .args(["attach", "-nobrowse", "-quiet", "-mountpoint"])
      .arg(&mount)
      .arg(&archive)
      .status()
      .map_err(|e| format!("hdiutil attach: {e}"))?;
    if !attach.success() {
      return Err("could not mount the update image".to_string());
    }
    (mount.clone(), Some(Unmount(mount)))
  };

  // Verify the payload in the prepared archive, before a byte of it reaches the
  // installed bundle. Verifying only afterwards means a bad update is already
  // committed by the time you find out — there is no rollback, and the next
  // launch runs it.
  on_stage(UpdateStage::Verifying);
  let src = app_in(&prepared)?;
  let trusted = Command::new("codesign")
    .args(["--verify", "--deep"])
    .arg(format!("-R={requirement}"))
    .arg(&src)
    .status()
    .map_err(|e| format!("codesign: {e}"))?;
  if !trusted.success() {
    return Err(format!(
      "the update isn't signed by {} — refusing to install it",
      config.slug
    ));
  }

  // rsync the prepared bundle's *contents* (trailing slash) onto the installed
  // bundle; --delete drops files the new version no longer ships.
  // --delay-updates stages every changed file inside the bundle (per-directory
  // `.~tmp~` folders) and promotes it by rename only at the end, so a sync that
  // dies partway leaves the old files intact instead of a mixed bundle with a
  // broken signature. `Icon?` is the dmg's custom-icon file (`Icon\r`), not
  // part of the app.
  on_stage(UpdateStage::Installing);
  let mut contents = std::ffi::OsString::from(src);
  contents.push("/");
  let synced = Command::new("rsync")
    .args(["-a", "--delete", "--delay-updates", "--exclude", "Icon?"])
    .arg(&contents)
    .arg(app)
    .status()
    .map_err(|e| format!("rsync: {e}"))?;
  if !synced.success() {
    scrub_staging(app);
    return Err("could not copy the update into place".to_string());
  }

  // Re-verify what actually landed. The pre-flight check above established the
  // payload was genuine; this catches the sync itself having mangled it, which
  // Gatekeeper would otherwise turn into a dead next launch.
  let verified = Command::new("codesign")
    .args(["--verify", "--deep"])
    .arg(format!("-R={requirement}"))
    .arg(app)
    .status()
    .map_err(|e| format!("codesign: {e}"))?;
  if !verified.success() {
    return Err("the updated app failed signature verification".to_string());
  }

  drop(unmount);
  let _ = std::fs::remove_dir_all(&dir);
  Ok(Relaunch::Current)
}

/// Remove the `.~tmp~` staging folders `rsync --delay-updates` leaves under
/// `dir` when a sync fails partway.
#[cfg(target_os = "macos")]
fn scrub_staging(dir: &std::path::Path) {
  let Ok(entries) = std::fs::read_dir(dir) else {
    return;
  };
  for entry in entries.flatten() {
    let path = entry.path();
    if !path.is_dir() {
      continue;
    }
    if path.file_name().is_some_and(|n| n == ".~tmp~") {
      let _ = std::fs::remove_dir_all(&path);
    } else {
      scrub_staging(&path);
    }
  }
}

/// The first `.app` bundle inside a mounted image or expanded archive.
#[cfg(any(target_os = "macos", test))]
fn app_in(dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
  std::fs::read_dir(dir)
    .ok()
    .and_then(|mut entries| {
      entries.find_map(|entry| {
        entry
          .ok()
          .map(|e| e.path())
          .filter(|p| p.extension().is_some_and(|x| x == "app"))
      })
    })
    .ok_or_else(|| "no .app in the update archive".to_string())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn install(
  _config: &UpdateConfig,
  _release: &crate::update::Release,
  _app: &std::path::Path,
  _on_stage: &dyn Fn(crate::update::UpdateStage),
) -> Result<crate::update::Relaunch, String> {
  Err("not macOS".to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A scratch dir that cleans up after itself.
  fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("guise-update-mac-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn finds_the_app_in_a_mounted_image() {
    let dir = scratch("appin");
    std::fs::create_dir_all(dir.join("Acme.app/Contents")).unwrap();
    std::fs::write(dir.join(".background"), b"").unwrap();
    assert_eq!(app_in(&dir).unwrap(), dir.join("Acme.app"));
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn empty_image_is_an_error() {
    let dir = scratch("empty");
    assert!(app_in(&dir).is_err());
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn scrub_removes_only_rsync_staging_dirs() {
    let dir = scratch("scrub");
    std::fs::create_dir_all(dir.join("Contents/MacOS/.~tmp~")).unwrap();
    std::fs::write(dir.join("Contents/MacOS/.~tmp~/acme"), b"half").unwrap();
    std::fs::create_dir_all(dir.join("Contents/.~tmp~")).unwrap();
    std::fs::write(dir.join("Contents/Info.plist"), b"keep").unwrap();

    scrub_staging(&dir);

    assert!(!dir.join("Contents/MacOS/.~tmp~").exists());
    assert!(!dir.join("Contents/.~tmp~").exists());
    assert_eq!(
      std::fs::read(dir.join("Contents/Info.plist")).unwrap(),
      b"keep"
    );
    let _ = std::fs::remove_dir_all(&dir);
  }

  /// Without a configured requirement there is nothing to verify the payload
  /// against, so the install must refuse rather than trust whatever mounts.
  #[cfg(target_os = "macos")]
  #[test]
  fn an_unconfigured_requirement_refuses_to_install() {
    let release = crate::update::Release {
      version: "9.9.9".to_string(),
      url: String::new(),
      assets: vec![crate::update::ReleaseAsset {
        name: "Acme.dmg".to_string(),
        url: "https://d/x".to_string(),
        size: 0,
      }],
    };
    let config = UpdateConfig::new("Acme", "1.0.0", crate::update::UpdateSource::github("a/b"));
    let err = install(
      &config,
      &release,
      std::path::Path::new("/Applications/Acme.app"),
      &|_| {},
    )
    .unwrap_err();
    assert!(err.contains("codesign requirement"), "{err}");
  }
}
