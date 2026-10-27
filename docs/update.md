# Software update

`guise::update` is a complete self-update feature: it checks a release feed for a
newer version, installs it **in place**, prompts the user, and restarts into it.

```rust
use guise::update::{check_now, start, Updater};

let updater = Updater::github("Acme", env!("CARGO_PKG_VERSION"), "acme/acme")
    .codesign_requirement("anchor apple generic and certificate leaf[subject.OU] = TEAMID")
    .on_notify(|title, body| post_os_notification(title, body))
    .restart_guard(|cx| can_restart_without_losing_work(cx))
    .before_restart(|cx| save_session(cx));

start(updater.clone(), cx);   // at launch, then hourly
check_now(updater, cx);       // wire to a "Check for Updates…" menu item
```

That is the whole integration. `start` opens the prompt when a newer release is
ready — once per version, so a check doesn't reopen a window the user already
dismissed. `check_now` always answers: the prompt if there's an update, and a
short notice saying why not if there isn't.

The module has two halves, and they are usable apart:

- **The engine** — `UpdateConfig`, plus the `Release` / `InstallKind` /
  `UpdateStage` / `Relaunch` vocabulary. No gpui, entirely testable, and blocking,
  so it belongs on the background executor.
- **The UI** — `UpdatePrompt` and `UpdateNotice`, ordinary guise entities. Put
  them in a window you own, or let `open` / `check_now` put them in their own.

## The prompt

`UpdatePrompt` is a **stateful entity** with exactly one action in flight. Once an
install starts it reports every stage it moves through and stays put until it
either restarts the app or fails with the reason on screen — never a button whose
only feedback is a notification you might not see.

```rust
let prompt = cx.new(|cx| UpdatePrompt::new(updater, release, cx));

cx.subscribe(&prompt, |_this, _prompt, event: &UpdatePromptEvent, _cx| {
    match event {
        UpdatePromptEvent::Started        => {}
        UpdatePromptEvent::Stage(stage)   => println!("{}", stage.label()),
        UpdatePromptEvent::Installed(_)   => {}
        UpdatePromptEvent::Failed(why)    => eprintln!("{why}"),
        UpdatePromptEvent::Dismissed      => {}
    }
})
.detach();

Stack::new().child(prompt.clone())   // or update::open(updater, release, cx)
```

Its action button never promises what it can't do. When the install can be
rewritten in place and the release has published the matching asset, the button
reads **Update & Restart**; otherwise it reads **Open Download** and opens the
release page. A failure turns it into **Try Again**.

| Method | Notes |
| --- | --- |
| `new(updater, release, cx)` | Resolves installability once, so the button's promise can't drift. |
| `auto_restart(bool)` | Default `true`. Off means you handle `Installed` yourself. |
| `window_root(bool)` | Draws a titlebar drag strip; set by `update::open`, off when embedding. |
| `accept(cx)` | Take the action the button offers. |
| `dismiss(cx)` | Emit `Dismissed`. Inert while installing. |
| `set_stage(stage, cx)` / `set_failed(why, cx)` / `reset(cx)` | Drive the display yourself. |
| `busy()` / `stage()` / `error()` / `release()` | Read the current state. |

The prompt **never closes a window** — it emits `Dismissed` and whoever owns the
window decides. `update::open` subscribes and closes the window it created.

`update::is_installing(cx)` reports whether an install is running anywhere in the
process; worth checking before offering a menu item that would open a second
prompt.

## The notice

`UpdateNotice` answers a manual check that had nothing to install. It's a panel,
not a desktop notification, because a notification is silently dropped when the
user has denied the app permission to post one — and a "Check for Updates…" that
appears to do nothing at all is worse than the answer being unwelcome.

```rust
let notice = cx.new(|cx| UpdateNotice::new(updater, UpdateOutcome::UpToDate, cx));
```

`UpdateOutcome` has three cases, and each names both what happened and why:

| Outcome | Reads as |
| --- | --- |
| `UpToDate` | "You're up to date" · "Acme 1.31.0 is the latest version." |
| `Pending(version)` | "Acme 1.32.0 is on the way" · "It is still building for this platform." |
| `Failed(why)` | "Couldn't check for updates" · the reason |

`Pending` is the case worth keeping: a release goes live before CI finishes
uploading to it, and "still building" is the truth where "up to date" is a lie.

## Configuring the updater

| Method | Default | Notes |
| --- | --- | --- |
| `Updater::github(app, version, "owner/repo")` | — | Reads GitHub's `releases/latest`. |
| `Updater::new(app, version, source)` | — | Any [`UpdateSource`](#release-sources). |
| `codesign_requirement(text)` | none | **Required for macOS installs** — see below. |
| `require_checksum(bool)` | `false` | Refuse to install when the release publishes no SHA-256 — **recommended on Linux**, see below. |
| `poll_every(Duration)` | 1 hour | How often `start` re-checks. |
| `window_title(into)` | `"Software Update"` | Title for the windows `open` creates. |
| `user_agent(into)` | `"<slug>-updater"` | GitHub rejects requests without one. |
| `slug(into)` | derived from the app name | Filesystem-safe name for staging paths. |
| `on_notify(f)` | none | `(title, body)` when an install starts, finishes unattended, or fails. |
| `restart_guard(f)` | allow | Return `false` to defer activation until the next normal launch. |
| `before_restart(f)` | none | Called just before the restart — persist your session here. |

guise never posts an OS notification itself: that needs the app's own bundle
identity and permission state. `restart_guard` is for unsaved work or any other
condition that makes an immediate restart unsafe. A veto leaves the installed
update ready for the next launch and does not run `before_restart`; an allowed
restart runs that hook because it does not go through the app's normal quit path.

## Release sources

Both sources expect the shape of GitHub's release API (`tag_name`, `html_url`,
and an `assets` array of `name` / `browser_download_url` / `size`), because that
is what desktop apps already publish:

```rust
UpdateSource::github("acme/acme")               // api.github.com/repos/…/releases/latest
UpdateSource::url("https://acme.dev/latest.json")
```

Feeds are parsed by a small reader inside guise, so this costs no dependency. A
malformed release tag is refused rather than sanitized — the version becomes a
path component of the staging directory, and a tag carrying `..` would otherwise
escape `$TMPDIR`.

## Installing in place is the load-bearing decision

An install is never *replaced*, only *rewritten in place*.

**macOS.** A release `.dmg` is mounted, or a `.zip` is expanded, and the new
bundle's contents are `rsync --delete`d onto the installed `.app`. If both
formats are published, Guise prefers the DMG. The bundle directory keeps its
path *and* its inode, so LaunchServices' registration stays valid and the
relaunch is `Relaunch::Current` — gpui reopens the running bundle via
`NSBundle`. A
rename-swap instead hands `open` a brand-new directory inode while deleting the
running executable; when LaunchServices resolves the stale registration it can
fall back to running the inner Mach-O as a plain executable, inside a terminal,
unbundled and broken. `--delay-updates` stages every changed file and promotes it
by rename at the end, so a sync that dies partway leaves the old bundle intact
rather than a mixed one with a broken signature.

**Linux AppImage.** The new image is downloaded *next to* the running one — a
rename across filesystems fails, and `/tmp` is often tmpfs — then renamed over
it. The relaunch is `Relaunch::Binary`, pointing at the image. The asset is
matched by architecture as a whole token, never as a substring: `ARCH` is `"x86"`
on 32-bit x86 and `"arm"` on 32-bit ARM, both substrings of the 64-bit asset
names, and a loose match would rename the wrong image over a working install.

**Anything else** (`InstallKind::Unknown`: a root-owned `.deb`/`.rpm`, a Windows
install, a dev build) can't be rewritten from inside the app, so the prompt opens
the release page instead — and says so on its button.

Downloads run through `curl`: https only (on redirects too), a size cap on
in-memory bodies, and a finished download whose length doesn't match what the
feed reported is treated as a failure. A truncated `.dmg` fails to mount, a
truncated `.zip` fails to expand, and a truncated AppImage would otherwise be
renamed over a working install.

## macOS installs need a codesign requirement

```rust
.codesign_requirement("anchor apple generic and certificate leaf[subject.OU] = XJDC46F35X")
```

The downloaded bundle is verified **on the mounted image, before a byte of it
reaches the installed app** — verifying afterwards means a bad update is already
committed, with no rollback and the next launch running it. It is verified again
after the sync, which catches the copy itself having mangled it.

Pass the requirement text only; the `-R=` that `codesign` needs is added for you.
Pin your *team*, not a certificate — team IDs survive certificate renewals.

Without a requirement there is nothing to verify against: a bare `--verify` only
proves a bundle matches its own seal, so any validly signed app at all would
pass, which is not the question to ask of something about to be executed as the
user. So with no requirement configured, `can_install` returns false for a macOS
install and the prompt falls back to the download page. The practical consequence
is that an ad-hoc signed build — what CI produces when the signing secrets are
absent — cannot self-update, and shouldn't be able to.

## Linux installs should require a checksum

macOS has `codesign` to answer "did the people who ship this app produce this
bundle". Linux has no equivalent: the AppImage path downloads a file, marks it
executable, and renames it over the running binary. A byte count is not an
integrity check, so without something more the only thing vouching for what
runs on the next launch is that the release feed named it.

Publish a digest beside each artifact and turn the check on:

```rust
Updater::github("Acme", env!("CARGO_PKG_VERSION"), "acme/acme")
    .require_checksum(true)
```

A published digest is **always** verified when one exists, on both platforms —
on macOS before the image is even mounted, since `hdiutil` parsing an
attacker-chosen file is a larger surface than hashing it. `require_checksum`
only decides what a *missing* digest means: a silent pass (the default, so a
project that never shipped checksums keeps working) or a refusal.

Two layouts are recognised, and a per-asset file wins because it is
unambiguous about what it covers:

```
Acme-x86_64.AppImage
Acme-x86_64.AppImage.sha256      # or .sha256sum / .SHA256
SHA256SUMS                       # or checksums.txt — a listing works too
```

A listing has to *name* the asset. Falling back to "the only digest in the
file" would happily verify the wrong one. A generic listing that covers other
artifacts but not this one counts as missing: lenient mode proceeds, while
`require_checksum(true)` refuses it. A malformed per-asset sidecar always fails.

The hash comes from whichever of `shasum`, `sha256sum`, or `openssl` is on the
machine, the same way the rest of this module shells out rather than growing a
dependency. Note what this does and doesn't buy you: it defeats a swapped or
corrupted asset, a poisoned mirror or CDN, and a truncated download that
happens to match the advertised size. It does not prove authorship — an
attacker who can rewrite the release can rewrite the digest beside it. On macOS
the codesign requirement is what proves that; on Linux, sign your releases out
of band if you need it.

## Driving the mechanics yourself

The engine is public. `UpdateConfig::check` and `UpdateConfig::install` are
**blocking** — run them on gpui's background executor, then feed the prompt with
`set_stage` / `set_failed`:

```rust
let config = updater.config().clone();
let found = cx.background_executor().spawn(async move { config.check() }).await;

match found {
    Ok(UpdateCheck::Ready(release)) => { /* offer it */ }
    Ok(UpdateCheck::Pending(version)) => { /* still building */ }
    Ok(UpdateCheck::UpToDate) => {}
    Err(why) => eprintln!("{why}"),
}
```

| Item | Notes |
| --- | --- |
| `detect() -> InstallKind` | How this copy was installed. Decides *how* to update, never *whether*. |
| `UpdateConfig::check()` | Fetch + classify against the running version. Blocking. |
| `UpdateConfig::install(release, kind, on_stage)` | Download and rewrite in place. Blocking; `on_stage` fires per download sample. |
| `UpdateConfig::can_install(release, kind)` | The question the action button asks. |
| `Release::asset_for(kind)` / `ready_for(kind)` | Which asset this machine wants, and whether it exists yet. |
| `is_newer(latest, current)` | Version comparison; anything unparseable is never newer. |

A release is only offered once it has published the asset this machine would
install. `ready_for` is deliberately looser for `Unknown`, which only ever opens a
page: gating that on a per-OS asset would strand anyone on a platform you don't
publish for on "still building" forever, never reaching the page fallback.

## Notes

- Two installs are never allowed to run at once. A second one would race two
  downloads over the same staging path, two `hdiutil attach` calls on the same
  mountpoint, and two rsyncs into the live bundle — each one's unmount tearing
  down the other's mount mid-copy.
- Closing the prompt's window mid-install withdraws consent to be restarted: the
  new version is left on disk for the next launch, and `on_notify` says so.
- The poller started by `start` runs at most once per process, so calling it from
  more than one place is safe.
