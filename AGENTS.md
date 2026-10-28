# dXn.I18n contributor guidance

Translations for Rust apps with [Unicode MessageFormat 2](https://www.unicode.org/reports/tr35/tr35-messageFormat.html)
messages (the `dxn_i18n` crate in `crates/`): the MF2 parser and formatter, the default functions,
JSON message bundles and a translator with locale fallback. What it offers is in `README.md`.

## Boundaries

- No UI or app framework: not NSH, GPUI, RayX or any web framework. CI fails when one appears in
  the dependency tree. Frameworks wrap this crate (RayX's i18n service does); their glue, such as
  reading resources from an app's assets or following the system's languages, lives in their
  own repositories.
- It builds for `wasm32-unknown-unknown`: no threads, files or system locale. The caller passes
  JSON text and locales.
- Plurals, numbers and dates are `dxn_intl`'s (dXn.Intl, ICU4X), consumed by release tag. What
  this crate needs from them is added there, not worked around here.
- The syntax and the formatting follow the MessageFormat 2 specification. The Working Group's test
  suite runs as `tests/conformance.rs` over the copy in `tests/conformance/` (see its README to
  update it); it must stay green. A deliberate deviation is documented in `README.md` and skipped
  by name in the runner with the reason.

## Validation

Run before committing:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --workspace --target wasm32-unknown-unknown
cargo package --workspace --list
```

Tests are grouped by feature, one file per feature in `crates/dxn_i18n/tests/` (`syntax`,
`formatting`, `selection`, `functions`, `translator`, `conformance`; select one with
`cargo test -p dxn_i18n --test <feature>`). A new feature or behavior change gets a new group, or
tests in the group of its feature. Test only what you changed while iterating, and run the full
list above once before committing. Raw diagnostic output goes to the ignored `artifacts-temp/`,
never into commits.

## Consumers

Consumers depend on release tags only:
`dxn_i18n = { git = "https://github.com/dXnSix/dXn.I18n", tag = "vX.Y.Z" }`. Never build or
validate a consumer to test a change here; mimic its use in this repository's tests.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) with meaningful scopes
(`feat(syntax): …`, `fix(translator): …`). Never add Claude co-author or other AI attribution
lines.

## Releases

release-plz tags `vX.Y.Z` (Git tags only, no crates.io) and writes `CHANGELOG.md`, which is never
edited by hand. 0.x policy: `fix`, `perf` and `feat` → patch; a breaking change (`!` or a
`BREAKING CHANGE:` footer) → minor, with the break stated in the body and a `Migration:` footer.
A release is merging release-plz's pull request after checking its version and that it changes
only `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`. Push straight to `main` with
`git fetch origin && git rebase origin/main`, then `git push --force-with-lease origin main`; pull
requests exist only for releases. See `docs/releasing.md`.
