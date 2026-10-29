---
title: Bundles and JSON
description: Load one language's messages into a Bundle from JSON resources with nested ids, prefixes and merging, and handle messages that fail to parse without losing the rest.
group: Translating
order: 90
---

A `Bundle` is one language's messages, looked up by id. You build one per language, from JSON text your app reads (a file, an asset, an HTTP response, `include_str!`), and hand them to a [`Translator`](./translator.md). The library does no I/O; it takes the text.

```rust
use dxn_i18n::Bundle;
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add_json(r#"{
  "app": { "title": "Shop" },
  "cart": {
    "title": "Your cart",
    "empty": "Nothing here yet.",
    "total": "Total: {$amount :currency currency=EUR}"
  }
}"#)?;

assert!(bundle.contains("cart.title"));
assert_eq!(bundle.len(), 4);
assert_eq!(bundle.locale().to_string(), "en");
```

## The JSON format

A resource is one JSON object. Every string leaf is a message, and nested objects make dotted ids: `{"cart": {"title": ...}}` is the id `cart.title`. Objects can nest as deep as you need, and a key may itself contain dots (`"cart.title"` is the same id).

- A value that is not a string or an object (a number, a list, `true`, `null`) is `MessageError::NotText`.
- A string is parsed as an MF2 message. An empty string is a valid, empty message.
- The top level must be an object; `[...]` is `LoadError::Json`.
- If two entries make the same id, the later one replaces the earlier one. Avoid it.

A message is the whole string, so a complex message goes on one line. JSON strings cannot hold raw line breaks, and MF2 does not need them:

```json
{
  "cart": {
    "summary": ".input {$count :number} .match $count 0 {{Empty.}} one {{One item.}} * {{{$count} items.}}"
  }
}
```

You can also write `\n` escapes in the JSON string to lay a long message out in lines; inside a complex message they are ordinary whitespace. In Rust source, a raw string (`r#"..."#`) saves you from escaping the quotes.

## Adding messages

| Method | Does |
|---|---|
| `add(id, source)` | Parses one message and adds it under `id`, replacing the one with that id. Returns the `ParseError`. |
| `add_json(json)` | Adds every message of a resource. |
| `add_json_with_prefix(prefix, json)` | The same with every id under `prefix`: prefix `"checkout"` turns `title` into `checkout.title`. |
| `merge(other)` | Adds another bundle's messages, replacing equal ids. |

Prefixes are the way to keep a file per feature without repeating the feature name inside the file:

```rust
use dxn_i18n::Bundle;
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add_json_with_prefix("checkout", r#"{ "title": "Checkout", "pay": "Pay {$amount :currency currency=EUR}" }"#)?;
bundle.add_json_with_prefix("profile", r#"{ "title": "Your profile" }"#)?;
bundle.add("app.name", "Shop")?;

let mut ids: Vec<&str> = bundle.ids().collect();
ids.sort();
assert_eq!(ids, ["app.name", "checkout.pay", "checkout.title", "profile.title"]);
```

`ids()` returns ids in no particular order. `get(id)` returns the parsed `&Message`, which formats on its own:

```rust
use dxn_i18n::{Args, Bundle};
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add("hello", "Hello, {$name}!")?;
let message = bundle.get("hello").expect("added above");
assert_eq!(message.format(bundle.locale(), &Args::new().with("name", "Ana")), "Hello, Ana!");
```

Normally you let the translator do that, because it also finds the right bundle and formats values in the user's preferences.

## When a resource is wrong

`add_json` does not stop at the first bad message. It adds every message that parses and returns a `LoadError::Messages` listing the ones it left out, so one typo cannot hide a whole language:

```rust
use dxn_i18n::{Bundle, LoadError, MessageError};
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
let result = bundle.add_json(r#"{ "ok": "Fine", "broken": "{$x", "count": 3 }"#);

let Err(LoadError::Messages(mut failures)) = result else {
    panic!("expected the two bad messages to be listed");
};
failures.sort_by(|a, b| a.0.cmp(&b.0));
assert_eq!(failures[0].0, "broken");
assert!(matches!(failures[0].1, MessageError::Parse(_)));
assert_eq!(failures[1], ("count".to_string(), MessageError::NotText));
assert!(bundle.contains("ok")); // the valid message is in
assert!(!bundle.contains("broken"));

assert!(matches!(bundle.add_json("not json"), Err(LoadError::Json(_))));
```

| `LoadError` | Meaning |
|---|---|
| `Json(String)` | Not JSON, or not a JSON object. Nothing was added. |
| `Messages(Vec<(String, MessageError)>)` | These ids were left out; the rest were added. |

`LoadError` implements `Display` and `Error`, so `eprintln!("{error}")` or `?` into a `Box<dyn Error>` is enough to log it (the failures are in no promised order):

```text
messages left out: `broken` (syntax error at byte 3: expected `}`); `count` (not text);
```

Decide per environment what a left-out message means. An app can log the error and keep going: a message that failed to load is simply absent, so the [lookup](./translator.md#the-lookup-chain) continues to the next locale, and the user sees the default language's text for that one message instead of a crash. A test or a build step should fail on it ([Testing](./testing.md)).

## Reading bundles

| Method | Returns |
|---|---|
| `locale()` | The bundle's `&Locale`. |
| `get(id)` | `Option<&Message>`. |
| `contains(id)` | `bool`. |
| `ids()` | An iterator of ids, unordered. |
| `len()`, `is_empty()` | The message count. |

Next: [The Translator](./translator.md) combines bundles into a lookup chain.
