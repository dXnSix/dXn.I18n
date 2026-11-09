---
title: Macros
description: args! and t! build arguments and format messages at the call site, through the Translate trait a framework's service can implement; dxn_i18n_macros checks message ids and arguments when the crate builds.
group: Translating
order: 105
---

`args!` builds `Args` and `t!` formats a message, so a call site reads like `format!`:

```rust
use dxn_i18n::{Bundle, Translator, args, t};
use dxn_intl::parse_locale;

let mut en = Bundle::new(parse_locale("en")?);
en.add_json(r#"{
  "hello": "Hello, {$name}!",
  "items": ".input {$count :number}\n.match $count\none {{One item}}\n* {{{$count} items}}"
}"#)?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(en);

let name = "Ana";
assert_eq!(t!(translator, "hello", name), "Hello, Ana!");
assert_eq!(t!(translator, "items", count = 1200), "1,200 items");

let args = args!(name, count = 3, "user-id" = 42, r#type = "admin");
assert_eq!(args.iter().count(), 4);
```

## `args!`

| Form | Passes |
|---|---|
| `name = value` | `value` as `$name`, keeping its type: a number stays a number for `.match` and `:number`. |
| `name` | The variable `name` as `$name`, as `format!` does. |
| `"user-id" = value` | A variable name Rust cannot spell. |
| `r#type = value` | `$type`: a raw identifier names the variable without its `r#`. |

A later value for a name replaces an earlier one, as `Args::set` does, and names compare in Unicode NFC.

## `t!` and `Translate`

`t!(translator, id, args…)` is `Translate::translate(&translator, id, &args!(args…))`. `Translate` has one method, and a `Translator`, a reference to one and a `Box`, `Rc` or `Arc` of one implement it; so does a framework's service that holds a translator and decides the locale itself:

```rust
use std::cell::RefCell;
use dxn_i18n::{Args, Translate, Translator, t};

struct I18n {
    translator: RefCell<Translator>,
}

impl Translate for I18n {
    fn translate(&self, id: &str, args: &Args) -> String {
        self.translator.borrow().format(id, args)
    }
}

fn greet(i18n: &impl Translate) -> String {
    t!(i18n, "hello", name = "Iva")
}
```

`t!` never fails: a missing message and a placeholder that cannot be formatted show as `Translator::format` says ([Errors and fallbacks](./errors.md)). The id is any `&str`, so it can be made at run time.

## Checked at build time

`dxn_i18n_macros` has a `t!` that takes the same arguments and checks them when the crate builds, against the crate's messages:

```toml
[dependencies]
dxn_i18n_macros = { git = "https://github.com/dXnSix/dXn.I18n", tag = "v0.1.2" }

[package.metadata.dxn_i18n]
messages = "locales"              # the directory, relative to Cargo.toml (the default)
default-locale = "en"             # the locale every id must be in (the default)
default-namespace = "translation" # a namespace whose ids are not prefixed (none by default)
```

```rust
use dxn_i18n_macros::t;

let text = t!(translator, "cart.items", count = 3);
```

```text
error: no message `cart.itmes` in the default locale `en` (`locales`); did you mean `cart.items`?
error: message `cart.items` has no argument `$cuont`; it takes `$count`
error: message `greeting` needs the argument `$gender`, which its `hr` translation reads
```

- The directory holds `<locale>.json` and `<locale>/<namespace>.json` resources; a namespace's ids are `<namespace>.<id>`, except the default namespace's.
- An id is a string literal, and must be in the default locale.
- Every argument a translation of the message reads is needed, and one that none of them reads is an error. A message that does not parse is an error too.
- A change to a message file rebuilds the crate; a new file is read the next time the crate builds.

`check!(id, (args…), messages = "…" => expression)` is the check alone, for a framework's own translation macro: its `macro_rules!` passes the call's id and arguments, its defaults (`messages`, `default_locale`, `default_namespace`, which the crate's metadata overrides) and the expression to expand to.

Next: [Arguments and values](./arguments.md).
