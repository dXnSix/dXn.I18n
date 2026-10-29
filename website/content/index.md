---
title: dXn.I18n
description: Translations for Rust apps with Unicode MessageFormat 2 messages - plurals, numbers, currencies, dates, JSON bundles and locale fallback - independent of any UI framework and building for WebAssembly.
eyebrow: Translations for Rust
headline: MessageFormat 2 messages, formatted in Rust.
lede: >-
  dxn_i18n parses and formats Unicode MessageFormat 2 messages, loads them from JSON resources and
  finds the right bundle for the user's locale and its fallbacks. Plurals, numbers, money and dates
  come from dXn.Intl on ICU4X's CLDR data, so every platform, the browser included, shows the same
  text. It depends on no UI framework.
install: |-
  [dependencies]
  dxn_i18n = { git = "https://github.com/dXnSix/dXn.I18n", tag = "v0.1.0" }
  dxn_intl = { git = "https://github.com/dXnSix/dXn.Intl", tag = "v0.1.0" }
facts:
  - { label: Messages, value: "Unicode MessageFormat 2" }
  - { label: Conformance, value: "462 Working Group cases pass" }
  - { label: Locale data, value: "CLDR via ICU4X" }
  - { label: Rust, value: "1.97+ (edition 2024)" }
  - { label: Targets, value: "native and wasm32" }
  - { label: License, value: "MIT OR Apache-2.0" }
heroTitle: main.rs
heroLang: rust
heroCode: |
  use dxn_i18n::{Args, Bundle, Translator};
  use dxn_intl::parse_locale;

  fn main() -> Result<(), Box<dyn std::error::Error>> {
      let mut hr = Bundle::new(parse_locale("hr")?);
      hr.add_json(r#"{ "cart": { "summary":
        ".input {$count :number} .match $count 0 {{Košarica je prazna.}} one {{{$count} artikl.}} few {{{$count} artikla.}} * {{{$count} artikala.}}"
      } }"#)?;

      let mut translator = Translator::new(parse_locale("en")?);
      translator.add_bundle(hr);
      translator.set_locale(parse_locale("hr-HR")?);

      for count in [0, 1, 3, 5] {
          let args = Args::new().with("count", count);
          println!("{}", translator.format("cart.summary", &args));
      }
      Ok(())
  }
features:
  - title: The whole of MessageFormat 2
    text: Simple and complex messages, .input and .local declarations, matchers with any number of selectors, markup, fallbacks and bidi isolation, checked against the data model when a message is parsed.
    link: ./docs/message-syntax.md
  - title: Plurals for every CLDR language
    text: Cardinal and ordinal categories, exact numbers and text keys pick the variant, so Croatian gets its few form and Arabic its six. The category reads the digits actually shown.
    link: ./docs/matching.md
  - title: Numbers, money and dates in the user's style
    text: ":number, :integer, :percent, :currency, :offset, :date, :time and :datetime with their options, plus the user's -u-hc-h23 clock, -u-nu- digits and calendar preferences."
    link: ./docs/functions.md
  - title: JSON bundles and a lookup chain
    text: Nested JSON makes ids like cart.summary. A Translator walks from the user's locale through region parents and fallback locales to the default, and formats values in the matching language.
    link: ./docs/translator.md
  - title: Markup as parts
    text: "format_to_parts returns text, values, fallbacks and markup separately, so {#b}bold{/b} and links render in your UI without string surgery."
    link: ./docs/markup-and-parts.md
  - title: Failures still produce text
    text: A placeholder that cannot be resolved shows a fallback such as {$name}, and try_format returns the typed errors for logs and tests. A message that fails to load never hides the rest of its language.
    link: ./docs/errors.md
next:
  - { title: Get started, text: "Add the dependency and format your first plural message.", link: ./docs/getting-started.md }
  - { title: Message syntax, text: "Placeholders, declarations and the parse errors you can get.", link: ./docs/message-syntax.md }
  - { title: The Translator, text: "How a message is found and which locale formats it.", link: ./docs/translator.md }
  - { title: Cookbook, text: "Embedding translations, checking them in a test, sharing a translator.", link: ./docs/cookbook.md }
---

## Pick the plural form for the language

A matcher lists variants by key. `0` is an exact number, `one` and `few` are the language's plural categories, and `*` is the fallback every matcher needs.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$count :number}
     .match $count
     0   {{Your cart is empty.}}
     one {{One item in your cart.}}
     *   {{{$count} items in your cart.}}",
)?;
let en = parse_locale("en")?;
assert_eq!(message.format(&en, &Args::new().with("count", 1200)), "1,200 items in your cart.");
```

See [Selecting variants](./docs/matching.md) for ordinals, text keys and several selectors.

## Format in the user's locale and preferences

The locale carries the user's region and Unicode extensions. A `de-CH` user sees Swiss separators in a `de` message, and `-u-hc-h23` turns every time into a 24-hour one.

```rust
use dxn_i18n::{Args, Bundle, Translator};
use dxn_intl::{DateTime, parse_locale};

let mut translator = Translator::new(parse_locale("en")?);
for (tag, json) in [("en", r#"{ "at": "At {$t :time}." }"#), ("de", r#"{ "price": "{$p :number}" }"#)] {
    let mut bundle = Bundle::new(parse_locale(tag)?);
    bundle.add_json(json)?;
    translator.add_bundle(bundle);
}

translator.set_locale(parse_locale("de-CH")?);
assert_eq!(translator.format("price", &Args::new().with("p", 1234.5)), "1'234.5");

translator.set_locale(parse_locale("en-US-u-hc-h23")?);
let at = Args::new().with("t", DateTime::new(2026, 10, 27, 15, 45, 0)?);
assert_eq!(translator.format("at", &at), "At 15:45.");
```

See [The Translator](./docs/translator.md) for the lookup chain and [Dates and times](./docs/dates-and-times.md).

## Render markup without string surgery

Translators put `{#link}` where the link belongs in their language. `format_to_parts` gives you the pieces, with markup options resolved.

```rust
use dxn_i18n::{Args, Bundle, Translator};
use dxn_intl::parse_locale;

let mut en = Bundle::new(parse_locale("en")?);
en.add("terms", "Read the {#link href=$url}terms{/link} first.")?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(en);

let args = Args::new().with("url", "https://example.com/terms");
let parts = translator.format_to_parts("terms", &args).expect("the message exists").text;
assert_eq!(parts.len(), 5);
// [Text("Read the "), Markup { kind: Open, name: "link", options: [("href", ..)] },
//  Text("terms"), Markup { kind: Close, name: "link", .. }, Text(" first.")]
```

See [Markup and parts](./docs/markup-and-parts.md) for a renderer.

## Errors that stay on screen

Formatting never panics and never returns nothing. What cannot be resolved shows as a fallback, and the errors come back next to the text.

```rust
use dxn_i18n::{Args, FormatError, Message, MessageFormatter};
use dxn_intl::parse_locale;

let message = Message::parse("Hello {$name}, you have {$n :number} messages.")?;
let formatter = MessageFormatter::new(parse_locale("en")?);
let formatted = formatter.format(&message, &Args::new().with("n", "many"));

assert_eq!(formatted.text, "Hello {$name}, you have {$n} messages.");
assert_eq!(formatted.errors[0], FormatError::UnresolvedVariable("name".to_string()));
```

See [Errors and fallbacks](./docs/errors.md).

## Add your own function

A function is a plain `fn` that returns a value able to format itself and, if you like, to select a variant. Register it under a namespace.

```rust
use std::sync::Arc;
use dxn_i18n::{
    Args, FunctionContext, FunctionError, FunctionRegistry, Message, MessageFormatter, Options,
    ResolvedValue, TextValue, Value,
};
use dxn_intl::parse_locale;

fn shout(
    _: &mut FunctionContext<'_>,
    operand: Option<&Value>,
    _: &Options,
) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
    let text = operand.and_then(Value::as_str).unwrap_or_default();
    Ok(Arc::new(TextValue(text.to_uppercase())))
}

let mut functions = FunctionRegistry::default();
functions.register("my:shout", shout);
let formatter = MessageFormatter::new(parse_locale("en")?).with_functions(Arc::new(functions));
let message = Message::parse("Welcome, {$name :my:shout}!")?;
assert_eq!(formatter.format(&message, &Args::new().with("name", "ana")).text, "Welcome, ANA!");
```

See [Custom functions](./docs/custom-functions.md).
