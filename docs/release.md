# Releasing

One thing to ship: the **`guise-ui`** library. The version lives in the root
`[workspace.package]`, and the release comes off a tag.

[Tailor](https://github.com/wess/tailor) used to be released off the same tag as
a macOS app. It has its own repository and its own release now, on its own
cadence — which is most of why it moved.

## Cutting one

1. Write the CHANGELOG section. `## <version> — <date>`, and make it the notes
   you would want to read: `release.yml` lifts that section verbatim, so a list
   of commit subjects is not an option.
2. Bump `version` in the root `Cargo.toml`, then `cargo build --workspace` to
   regenerate `Cargo.lock` — it is committed, and CI builds `--locked`, so a
   bump without the lockfile fails.
3. Commit, tag `v<version>`, push both.

```sh
git tag -a v1.7.0 -m "Version 1.7.0 — …"
git push origin main v1.7.0
```

Pushing to `main` also deploys the site (`pages.yml`, on any change under
`site/` or `docs/`). Pushing the tag runs `release.yml`, which opens the GitHub
release with notes from the CHANGELOG.

## crates.io

Manual, on purpose:

```sh
cargo publish -p guise-ui
```

The library builds against plain crates.io `gpui`, with no patch section, so
this works from a clean checkout. It stays a human step because publishing
cannot be undone — a version can be yanked, never replaced.

`cargo package -p guise-ui --list` is worth a look before you do: it is the
proof that the crate carries the library and nothing else.

## After publishing

Tailor pins `guise-ui` from crates.io and keeps a checked-in record of what the
pinned version ships, so a release is only visible over there once someone bumps
the version and regenerates that file. Its tests fail until both happen, which
is the intended way for a new component to get catalogued rather than quietly
missed.
