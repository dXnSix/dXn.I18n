---
title: Markup and parts
description: Put bold text and links in translations with MessageFormat 2 markup, and render them with format_to_parts, which returns text, values, fallbacks and markup separately.
group: Messages
order: 40
---

Translators decide where emphasis and links go inside a sentence, and the word order changes by language, so the markup cannot live in your code. MF2 markup marks the spots, and `format_to_parts` hands them to you so your UI renders them as bold text, a link or a widget.

## Markup in a message

```rust
use dxn_i18n::{Args, Bundle, Translator};
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add_json(r#"{ "terms": "Read the {#link href=$url}terms{/link} first." }"#)?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(bundle);

let args = Args::new().with("url", "https://example.com/terms");
// As plain text, markup is dropped.
assert_eq!(translator.format("terms", &args), "Read the terms first.");
```

| Markup | Kind | Meaning |
|---|---|---|
| `{#name}` | `MarkupKind::Open` | Starts a span: `{#b}`. |
| `{/name}` | `MarkupKind::Close` | Ends it: `{/b}`. |
| `{#name/}` | `MarkupKind::Standalone` | Stands alone: `{#img src=\|a.png\| /}`. |

Names are free: `b`, `link`, or namespaced like `html:img`. Options are `name=value` with a literal or a `$variable`, as on functions. The library does not check that opens and closes pair up or nest; that is the renderer's business. Attributes (`@name`) are accepted and not reported in parts.

## Format to parts

`Translator::format_to_parts(id, &args)` returns `Option<Formatted<Vec<Part>>>`: `None` when no bundle has the message, otherwise the parts and the same `errors` list that `MessageFormatter::format` gives. For a single message use `MessageFormatter::format_to_parts(&message, &args)`.

```rust
use dxn_i18n::{Args, Bundle, MarkupKind, Part, Translator, Value};
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add_json(r#"{ "terms": "Read the {#link href=$url}terms{/link} first." }"#)?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(bundle);

let args = Args::new().with("url", "https://example.com/terms");
let formatted = translator.format_to_parts("terms", &args).expect("the message exists");
assert!(formatted.errors.is_empty());

assert_eq!(formatted.text[0], Part::Text("Read the ".to_string()));
assert_eq!(
    formatted.text[1],
    Part::Markup {
        kind: MarkupKind::Open,
        name: "link".to_string(),
        options: vec![("href".to_string(), Value::from("https://example.com/terms"))],
    }
);
assert_eq!(formatted.text[2], Part::Text("terms".to_string()));
assert!(matches!(&formatted.text[3], Part::Markup { kind: MarkupKind::Close, .. }));
```

### The four kinds of part

| `Part` | Holds | Appears for |
|---|---|---|
| `Text(String)` | The pattern's literal text, escapes resolved. | Text between placeholders. |
| `Value { function, text }` | The formatted value, and the name of the function that made it (`Some("number")`), or `None` for a plain variable or literal. | `{$n :number}`, `{$name}`. |
| `Fallback(String)` | The fallback name without braces, as `$name`. | A placeholder that failed. |
| `Markup { kind, name, options }` | The markup, with its options resolved to values. | `{#b}`, `{/b}`, `{#img/}`. |

`Value.function` lets a UI style numbers or money differently from names. `Fallback` carries `$name`; `MessageFormatter::format` wraps it in braces (`{$name}`) when it flattens parts to text. Options with a `u:` prefix are consumed by the library and left out of `options`.

## Render parts yourself

The library does not decide what `link` means. Map the names you support, ignore the rest and escape the text. A tiny HTML renderer:

```rust
use dxn_i18n::{MarkupKind, Part};

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn to_html(parts: &[Part]) -> String {
    let mut html = String::new();
    for part in parts {
        match part {
            Part::Text(text) | Part::Value { text, .. } => html.push_str(&escape(text)),
            Part::Fallback(name) => html.push_str(&escape(&format!("{{{name}}}"))),
            Part::Markup { kind, name, options } => match (kind, name.as_str()) {
                (MarkupKind::Open, "b") => html.push_str("<strong>"),
                (MarkupKind::Close, "b") => html.push_str("</strong>"),
                (MarkupKind::Open, "link") => {
                    let href = options
                        .iter()
                        .find(|(option, _)| option == "href")
                        .and_then(|(_, value)| value.as_str())
                        .filter(|href| href.starts_with("https://"));
                    html.push_str(&format!("<a href=\"{}\">", escape(href.unwrap_or("#"))));
                }
                (MarkupKind::Close, "link") => html.push_str("</a>"),
                _ => {} // markup this UI does not know is dropped
            },
        }
    }
    html
}

let parts = [
    Part::Text("Read the ".to_string()),
    Part::Markup { kind: MarkupKind::Open, name: "link".to_string(), options: vec![("href".to_string(), "https://example.com".into())] },
    Part::Text("terms & conditions".to_string()),
    Part::Markup { kind: MarkupKind::Close, name: "link".to_string(), options: vec![] },
];
assert_eq!(
    to_html(&parts),
    "Read the <a href=\"https://example.com\">terms &amp; conditions</a>"
);
```

:::warning Markup is translator-controlled
A translation file decides which markup names and option values reach your renderer. Match names against a fixed list as above, and check URLs (here: `https://` only) before putting them in a link.
:::

For a GUI toolkit the shape is the same: keep a stack, push a style on `Open`, pop it on `Close`, and emit the text runs in between.

## Standalone markup and values

`{#img src=|logo.png| alt=$alt /}` becomes one `Part::Markup` of kind `Standalone` whose options are `src` and `alt`. A variable option keeps the argument's own type: a number argument arrives as `Value::Number`, text as `Value::String`. An option whose variable is unresolved is left out and `UnresolvedVariable` is reported.

Next: [Text direction](./bidi.md) for mixed-direction text, or [Errors and fallbacks](./errors.md).
