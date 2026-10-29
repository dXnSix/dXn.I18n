# dXn.I18n

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**Documentation:** https://dxnsix.github.io/dXn.I18n/

**Translations for Rust apps with [Unicode MessageFormat 2](https://www.unicode.org/reports/tr35/tr35-messageFormat.html)
messages**, independent of any UI framework and building for `wasm32-unknown-unknown`. Plurals,
numbers and dates are formatted in Rust by [dXn.Intl](https://github.com/dXnSix/dXn.Intl) on
ICU4X's CLDR data, so every platform shows the same text.

- **MessageFormat 2**: the full syntax and data model, declarations, matchers with any number of
  selectors, markup, fallbacks and the default bidi isolation strategy. The Unicode MessageFormat
  Working Group's conformance suite runs in CI.
- **Functions**: `:string`, `:number`, `:integer`, `:offset`, `:percent`, `:currency`, `:date`,
  `:time` and `:datetime`, with plural (cardinal and ordinal) and exact selection for every
  language CLDR has; apps add their own functions.
- **Bundles and a translator**: JSON resources with nested ids, a lookup chain from the user's
  locale through its parents and fallbacks to the default, and values formatted in the user's
  region and preferences (`-u-hc-h23`, `-u-nu-…`).

## Use

```toml
[dependencies]
dxn_i18n = { git = "https://github.com/dXnSix/dXn.I18n", tag = "v0.1.0" }
dxn_intl = { git = "https://github.com/dXnSix/dXn.Intl", tag = "v0.1.0" }
```

```rust
use dxn_i18n::{Args, Bundle, Translator};
use dxn_intl::parse_locale;

let mut en = Bundle::new(parse_locale("en")?);
en.add_json(r#"{
  "cart": {
    "summary": ".input {$count :number}\n.match $count\n0 {{Your cart is empty.}}\none {{One item, {$total :currency currency=EUR}.}}\n* {{{$count} items, {$total :currency currency=EUR}.}}"
  }
}"#)?;
let mut hr = Bundle::new(parse_locale("hr")?);
hr.add_json(r#"{
  "cart": {
    "summary": ".input {$count :number}\n.match $count\n0 {{Košarica je prazna.}}\none {{{$count} artikl, {$total :currency currency=EUR}.}}\nfew {{{$count} artikla, {$total :currency currency=EUR}.}}\n* {{{$count} artikala, {$total :currency currency=EUR}.}}"
  }
}"#)?;

let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(en);
translator.add_bundle(hr);

let args = Args::new().with("count", 3).with("total", 42.5);
assert_eq!(translator.format("cart.summary", &args), "3 items, €42.50.");
translator.set_locale(parse_locale("hr-HR")?);
assert_eq!(translator.format("cart.summary", &args), "3 artikla, 42,50\u{a0}€.");
# Ok::<(), Box<dyn std::error::Error>>(())
```

A message that fails still formats: what cannot be resolved shows as its fallback (`{$name}`), and
`Translator::try_format` or `MessageFormatter::format` report the errors. `format_to_parts` gives
text, values and markup separately, for UIs that render `{#b}bold{/b}` or links.

Arguments are text, exact decimal numbers (`dxn_intl::Number`, from integers, floats or text) or
wall times (`dxn_intl::DateTime`); dates in messages are ISO 8601 text (`2026-10-27T15:45`).

## What is left out

`:number`'s significant-digit, notation and compact options are accepted and ignored; currencies
always show their own fraction digits and grouping. Time zones are the caller's: a date is a wall
time.

## Development

See [AGENTS.md](AGENTS.md) for validation and [docs/releasing.md](docs/releasing.md) for releases.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option. The conformance tests in `crates/dxn_i18n/tests/conformance/` are the Unicode
MessageFormat Working Group's, under the [Unicode License](crates/dxn_i18n/tests/conformance/LICENSE).
