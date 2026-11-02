---
title: Releases
description: How dxn_i18n is versioned and released as git tags with release-plz, how to depend on and upgrade a tag, and the 0.x versioning policy.
group: Project
order: 170
---

`dxn_i18n` is released as git tags `vX.Y.Z`, each with a GitHub Release. Nothing goes to crates.io: consumers depend on the tag, and `Cargo.lock` records the commit it points to. A registry release is not possible while `dxn_intl` itself comes by git tag only, so CI checks the package's file list (`cargo package --workspace --list`) instead of a registry build.

## Depend on a release

```toml
[dependencies]
dxn_i18n = { git = "https://github.com/dXnSix/dXn.I18n", tag = "v0.1.1" }
dxn_intl = { git = "https://github.com/dXnSix/dXn.Intl", tag = "v0.1.1" }
```

To upgrade, change the tag to a newer one from the [Releases page](https://github.com/dXnSix/dXn.I18n/releases). Keep the `dxn_intl` tag equal to the one the new `dxn_i18n` release uses ([Get started](./getting-started.md#install)). Tags never move or disappear: a bad release is fixed forward with a new patch release.

## Versioning

The library is 0.x, and the policy follows Cargo's convention for 0.x versions:

| Commit | Release |
|---|---|
| `fix:`, `perf:`, `feat:` | patch (`0.1.0` to `0.1.1`) |
| Breaking: `!` after the type or scope, or a `BREAKING CHANGE:` footer | minor (`0.1.x` to `0.2.0`) |
| `docs:`, `test:`, `ci:`, `build:`, `refactor:`, `chore:` | none on their own; listed in the next release's changelog |

:::warning A minor version can break your code
Under 0.x, `0.1.x` to `0.2.0` is the breaking step. Each breaking commit states the break in its body and carries a `Migration:` footer, which the changelog prints under the entry. Read it before changing the tag.
:::

## How a release happens

[release-plz](https://release-plz.dev) runs on every push to `main`, in git-only mode (`publish = false`; versions come from the `v*` tags). It:

1. Keeps a release pull request open with the next version and its `CHANGELOG.md` section.
2. When `Cargo.toml` holds a version that has no tag yet, tags `vX.Y.Z` and creates the GitHub Release with that section as its notes.

The first release, `v0.1.0`, needed no pull request: `Cargo.toml` already said `0.1.0` and no tag existed, so the release job tagged it. Maintainers push straight to `main`; pull requests exist only for releases.

Before merging a release pull request, check that:

- the version is the expected one (a minor bump only when a commit since the last tag is breaking),
- it changes only `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`,
- its CI is green.

`CHANGELOG.md` is written by release-plz and is never edited by hand. The release jobs authenticate with the repository secret `RELEASE_PLZ_TOKEN`, a fine-grained token the `dXnSix` library repositories share. Events made with the default `GITHUB_TOKEN` start no workflows, so with it the release pull request would get no CI. When the token expires, the release runs fail at checkout and releases stop until it is replaced.

Next: back to [Get started](./getting-started.md), or the [Cookbook](./cookbook.md).
