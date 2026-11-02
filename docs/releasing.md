# Releasing `dxn_i18n`

A release is a version tag `vX.Y.Z` on `main` with a GitHub Release of the same name. Nothing is
published to crates.io: consumers depend on the tag,

```toml
[dependencies]
dxn_i18n = { git = "https://github.com/dXnSix/dXn.I18n", tag = "vX.Y.Z" }
```

and `Cargo.lock` records the commit it points to. No registry release is made, and none can be
while `dxn_intl` comes by git tag only: CI checks the package's file list
(`cargo package --workspace --list`), not a registry build.

**release-plz** does the work (`release-plz.toml`, `.github/workflows/release-plz.yml`) in
git-only mode (`git_only = true`, `publish = false`): versions come from the `v*` tags, and on
every push to `main` it

- keeps a release pull request with the next version and its `CHANGELOG.md` section, and
- when `Cargo.toml` holds a version that has no tag yet, tags `vX.Y.Z` and creates the GitHub
  Release with that version's `CHANGELOG.md` section as its notes.

The first release, `v0.1.0`, needed no pull request: `Cargo.toml` said `0.1.0` and no `v*` tag
existed, so the `release` job tagged it on the first push that held the workflow.

## The 0.x policy

| Commit | Release |
| --- | --- |
| `fix:`, `perf:`, `feat:` | patch (`0.1.0` → `0.1.1`) |
| breaking: `!` after the type or scope, or a `BREAKING CHANGE:` footer | minor (`0.1.x` → `0.2.0`) |
| `docs:`, `test:`, `ci:`, `build:`, `refactor:`, `chore:` | none on their own; listed in the next release's changelog |

release-plz counts only the commits that change the crate's packaged files (`crates/dxn_i18n/` and
the `README.md` it ships). That is why `dxn_intl` is declared in `crates/dxn_i18n/Cargo.toml`, not
in the workspace table: a `fix(deps)` commit that moves its tag releases `dxn_i18n` on it, and
consumers read the tag to pair with there.

A breaking commit's body states the break and a `Migration:` footer tells consumers what to
change; the changelog prints it under the entry. `chore(release)` and `chore(deps)` commits are
left out of the changelog. `CHANGELOG.md` is written by release-plz and never edited by hand
between releases.

## Before merging a release pull request

- The proposed version is the expected one: a minor bump only when a commit since the last tag is
  breaking.
- It changes only `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`.
- Its CI is green.

## The `RELEASE_PLZ_TOKEN` secret

The release jobs use the fine-grained personal access token the `dXnSix` library repositories
share (Contents and Pull requests read and write), stored as the repository secret
`RELEASE_PLZ_TOKEN`; this repository must be on the token's access list. Events made with the
default `GITHUB_TOKEN` start no workflows, so with it the release pull request would get no CI.
When the token expires the Release-plz runs fail at checkout and releases stop until it is
replaced (see dXn.Reactive's `docs/releasing.md` for rotation).

## Tags never move

Release tags are never moved or deleted; a bad release is fixed forward with a new patch release.
