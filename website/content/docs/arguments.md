---
title: Arguments and values
description: Pass values to messages with Args and Value - text, exact decimal numbers, dates and pre-resolved values - and what each does when a function reads it.
group: Translating
order: 110
---

`Args` is the set of named values a message formats. Names are the variable names without the `$`, and each value is a `Value`, which converts from the types you already have.

```rust
use dxn_i18n::{Args, Value};
use dxn_intl::{DateTime, Number};

let args = Args::new()
    .with("name", "Ana")                         // text
    .with("count", 3)                            // any integer type
    .with("ratio", 0.256)                        // a finite float
    .with("price", "19.99".parse::<Number>()?)   // an exact decimal
    .with("due", DateTime::date(2026, 10, 27)?); // a wall time

assert_eq!(args.get("name").and_then(Value::as_str), Some("Ana"));
assert_eq!(args.get("count"), Some(&Value::from(3)));
assert!(args.get("missing").is_none());
```

## Building arguments

| Method | Does |
|---|---|
| `Args::new()` | No arguments. |
| `with(name, value)` | Returns the arguments with `name` set; chains. |
| `set(name, value)` | Sets `name` in place, replacing an earlier value. |
| `get(name)` | The `Option<&Value>`. |
| `iter()` | `(name, value)` pairs, in no particular order. |
| `is_empty()` | Whether there are none. |

`Args` also collects from an iterator of pairs with one value type:

```rust
use dxn_i18n::Args;

let limits: Args = [("min", 1), ("max", 10)].into_iter().collect();
assert!(!limits.is_empty());
```

Names compare in Unicode NFC, as variable names in messages do, so a decomposed accent in a name finds the composed one. An argument a message does not use is ignored. An argument a message uses and you did not pass is an `UnresolvedVariable` error and the placeholder shows `{$name}` ([Errors and fallbacks](./errors.md)).

## Value

`Value` is an enum with four cases:

| Case | Made from | A function reads it as |
|---|---|---|
| `Value::String(String)` | `&str`, `String`, `&String` | Text. `:number` reads text that is a number literal (`"42"`, `"1e3"`) as a number and `:date` reads ISO 8601 text as a date. |
| `Value::Number(Number)` | Every integer type (`i8` to `i128`, `u8` to `u128`, `isize`, `usize`), `f32`, `f64`, `Number` | An exact decimal. |
| `Value::DateTime(DateTime)` | `dxn_intl::DateTime` | A wall time on the ISO calendar. |
| `Value::Resolved(Arc<dyn ResolvedValue>)` | A custom function's result, or one you build | Whatever it says it is ([Custom functions](./custom-functions.md)). |

`Value::as_str()` returns the text of a `String` and `None` otherwise.

### Numbers

Integers keep every digit. A finite float becomes the shortest decimal that reads back as the same float (`0.1` is `0.1`). A float that is `NaN` or infinite becomes text (`"NaN"`, `"inf"`), and a number function then reports it as a bad operand. For amounts of money, build a `Number` from text so no float is involved: `"19.99".parse::<Number>()`.

A number argument is shown in the user's locale even with no function: `{$count}` with `1200` gives `1,200` in English. Text is not: the string `"1200"` in `{$count}` stays `1200`, so pass numbers as numbers, not as pre-formatted text.

### Dates

Pass a `DateTime` or ISO 8601 text, and use `:date`, `:time` or `:datetime` for options. A `DateTime` has no zone: convert an instant to the user's local time first ([Dates and times](./dates-and-times.md#there-is-no-time-zone)).

### Booleans and enums

There is no `From<bool>`. Pass a fixed string and select on it:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$state :string} .match $state on {{Notifications are on.}} * {{Notifications are off.}}",
)?;
let enabled = true;
let state = if enabled { "on" } else { "off" };
let text = message.format(&parse_locale("en")?, &Args::new().with("state", state));
assert_eq!(text, "Notifications are on.");
```

Pass the string your code chose (`"on"`), never one derived from user input, so translators know every value a selector can take.

### Already resolved values

`Value::Resolved` holds the result of a function. You rarely build one yourself; a custom function returns them, and the formatter passes them between composed functions. If you do need to pass a ready-made value, wrap an `Arc<dyn ResolvedValue>`: `TextValue` is the plain-text one.

```rust
use std::sync::Arc;
use dxn_i18n::{Args, Message, ResolvedValue, TextValue, Value};
use dxn_intl::parse_locale;

let ready: Arc<dyn ResolvedValue> = Arc::new(TextValue("ready".to_string()));
let args = Args::new().with("state", Value::Resolved(ready));
let message = Message::parse("State: {$state}")?;
assert_eq!(message.format(&parse_locale("en")?, &args), "State: ready");
```

Two `Value::Resolved` compare equal only when they are the same `Arc`.

Next: [Markup and parts](./markup-and-parts.md) for structured output, or [Errors and fallbacks](./errors.md).
