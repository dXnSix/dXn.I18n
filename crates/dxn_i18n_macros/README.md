# dxn_i18n_macros

Compile-time checked translations for [dxn_i18n](https://github.com/dXnSix/dXn.I18n): a message
id that is not in the crate's messages, an argument a message does not take, or one it needs and
the call does not pass, fails the build instead of showing a fallback at run time.

```toml
[dependencies]
dxn_i18n = { git = "https://github.com/dXnSix/dXn.I18n", tag = "vX.Y.Z" }
dxn_i18n_macros = { git = "https://github.com/dXnSix/dXn.I18n", tag = "vX.Y.Z" }

[package.metadata.dxn_i18n]
messages = "locales"              # the directory, relative to Cargo.toml (the default)
default-locale = "en"             # the locale every id must be in (the default)
default-namespace = "translation" # a namespace whose ids are not prefixed (none by default)
```

```rust,ignore
use dxn_i18n_macros::t;

let text = t!(translator, "cart.items", count = 3);
```

```text
error: no message `cart.itmes` in the default locale `en` (`locales`); did you mean `cart.items`?
error: message `cart.items` has no argument `$cuont`; it takes `$count`
error: message `greeting` needs the argument `$gender`, which its `hr` translation reads
```

- `t!` takes what `dxn_i18n::t!` takes, with the id as a string literal, and expands to it.
- The directory holds `<locale>.json` and `<locale>/<namespace>.json`, the JSON
  `Bundle::add_json` reads; a namespace's ids are `<namespace>.<id>`, except the default
  namespace's.
- An id must be in the default locale. Every argument a translation of it reads is needed, and
  one that none reads is an error. A message that does not parse is an error too.
- A change to a message file rebuilds the crate; a new file is read the next time the crate
  builds.
- `check!(id, (args…), messages = "…" => expression)` is the check alone, for a framework's own
  translation macro; its settings are defaults the crate's metadata overrides.
