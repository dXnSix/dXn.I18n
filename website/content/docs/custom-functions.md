---
title: Custom functions
description: Register your own MessageFormat 2 functions in a FunctionRegistry - the Function and ResolvedValue traits, reading options, selecting variants, composing with built-ins and reporting problems.
group: Guides
order: 130
---

When the built-ins do not cover a format your app needs (money in cents, a file size, a relative label), register a function. A message calls it like any other: `{$bytes :my:size}`. The function receives the operand and options and returns a value that knows how to format itself, and optionally how to select a variant.

```rust
use std::sync::Arc;
use dxn_i18n::{
    Args, FunctionContext, FunctionError, FunctionRegistry, Message, MessageFormatter, Options,
    ResolvedValue, TextValue, Value,
};
use dxn_intl::parse_locale;

fn shout(
    _context: &mut FunctionContext<'_>,
    operand: Option<&Value>,
    options: &Options,
) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
    let text = operand
        .and_then(Value::as_str)
        .ok_or_else(|| FunctionError::BadOperand("`:my:shout` needs text".to_string()))?;
    let mark = options.text("mark").unwrap_or_default();
    Ok(Arc::new(TextValue(format!("{}{mark}", text.to_uppercase()))))
}

let mut functions = FunctionRegistry::default(); // the built-ins, plus ours
functions.register("my:shout", shout);

let formatter = MessageFormatter::new(parse_locale("en")?).with_functions(Arc::new(functions));
let message = Message::parse("{$who :my:shout mark=|!|}")?;
let formatted = formatter.format(&message, &Args::new().with("who", "hey"));
assert_eq!(formatted.text, "HEY!");
assert!(formatted.errors.is_empty());
```

A function is anything that implements `Function`, and plain functions (and closures) with this signature do. `register` takes the name without the colon, so `"my:shout"` is called as `:my:shout`. Use a namespace of your own (`my:`, your app's name) so a name never collides with a function MF2 adds later; the `u:` namespace is reserved.

To use the registry in a translator, `translator.set_functions(Arc::new(functions))`. `FunctionRegistry::empty()` starts with nothing, not even `:string`.

## The pieces

| Item | Role |
|---|---|
| `Function::call(context, operand, options)` | Runs for one placeholder. Returns a resolved value, or a `FunctionError`. |
| `operand: Option<&Value>` | The operand, if the placeholder has one. An earlier function's result arrives as `Value::Resolved`. |
| `options: &Options` | The options as values. `text(name)` reads one as text, `get(name)` as a `Value`, `is_literal(name)` tells a literal from a variable, `iter()` lists them. |
| `context: &mut FunctionContext` | `locale()` is the locale to format for (the message's, or `u:locale`). `report(error)` records a problem you worked around. `operand_is_fallback()` is true when the operand's variable could not be resolved. |
| `ResolvedValue::format()` | The value as text. An `Err` makes the placeholder show its fallback. |
| `ResolvedValue::select(keys)` | The keys it matches, as indexes into `keys`, best first. Optional. |
| `ResolvedValue::direction()` | Its writing direction, for [bidi isolation](./bidi.md). Optional. |

A resolved value must be `Send + Sync + 'static` (the trait requires `Any + Send + Sync`).

### Errors

| Return | Result in the message |
|---|---|
| `Err(FunctionError::BadOperand(reason))` | The placeholder shows its fallback; a `FormatError::BadOperand` is recorded. |
| `Err(FunctionError::bad_option("currency", "an ISO 4217 code"))` | The same, as a `BadOption`. |
| `Err(FunctionError::Failed(reason))` | The same, as a `FormatError::Function`. |
| `context.report(error)` and `Ok(value)` | The value formats; the error is recorded. Use it for an option you ignored. |

## A value that formats with the locale

Money held in cents is common. A function can use the locale it is given and `dxn_intl`'s formatters:

```rust
use std::sync::Arc;
use dxn_i18n::{Args, FunctionContext, FunctionError, FunctionRegistry, Message, MessageFormatter, Options, ResolvedValue, TextValue, Value};
use dxn_intl::{Number, NumberFormatter, NumberOptions, parse_locale};

fn cents(
    context: &mut FunctionContext<'_>,
    operand: Option<&Value>,
    options: &Options,
) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
    let Some(Value::Number(cents)) = operand else {
        return Err(FunctionError::BadOperand("`:my:cents` needs a number of cents".to_string()));
    };
    let currency = options
        .text("currency")
        .ok_or_else(|| FunctionError::bad_option("currency", "an ISO 4217 code"))?;
    // 1999 cents is exactly 19.99: shift the decimal point, no float.
    let amount: Number = format!("{cents}e-2")
        .parse()
        .map_err(|_| FunctionError::BadOperand("not a whole number of cents".to_string()))?;
    let text = NumberFormatter::new(context.locale(), NumberOptions::currency(currency))
        .and_then(|formatter| formatter.format(amount))
        .map_err(|error| FunctionError::Failed(error.to_string()))?;
    Ok(Arc::new(TextValue(text)))
}

let mut functions = FunctionRegistry::default();
functions.register("my:cents", cents);
let formatter = MessageFormatter::new(parse_locale("de-DE")?).with_functions(Arc::new(functions));

let message = Message::parse("Gesamt: {$total :my:cents currency=EUR}")?;
let formatted = formatter.format(&message, &Args::new().with("total", 1999));
assert_eq!(formatted.text, "Gesamt: 19,99\u{a0}€");
```

## A function that selects

A resolved value that implements `select` can drive a `.match`. The matcher passes the keys it uses; return the indexes of those your value matches, best first. Return `None` when the value cannot select at all; that reports `BadSelector` and only `*` matches.

```rust
use std::sync::Arc;
use dxn_i18n::{Args, FunctionContext, FunctionError, FunctionRegistry, Message, MessageFormatter, Options, ResolvedValue, Value};
use dxn_intl::parse_locale;

#[derive(Debug)]
struct Bucket(f64);

impl ResolvedValue for Bucket {
    fn format(&self) -> Result<String, FunctionError> {
        Ok(self.0.to_string())
    }

    fn select(&self, keys: &[&str]) -> Option<Vec<usize>> {
        let name = match self.0 {
            n if n < 10.0 => "small",
            n if n < 100.0 => "medium",
            _ => "large",
        };
        Some(keys.iter().position(|key| *key == name).into_iter().collect())
    }
}

fn bucket(
    _context: &mut FunctionContext<'_>,
    operand: Option<&Value>,
    _options: &Options,
) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
    match operand {
        Some(Value::Number(number)) => Ok(Arc::new(Bucket(number.to_f64()))),
        _ => Err(FunctionError::BadOperand("`:my:bucket` needs a number".to_string())),
    }
}

let mut functions = FunctionRegistry::default();
functions.register("my:bucket", bucket);
let formatter = MessageFormatter::new(parse_locale("en")?).with_functions(Arc::new(functions));

let message = Message::parse(
    ".input {$files :my:bucket}
     .match $files
     small  {{A few files.}}
     medium {{Some files.}}
     *      {{Many files.}}",
)?;
let say = |n: i32| formatter.format(&message, &Args::new().with("files", n)).text;
assert_eq!(say(3), "A few files.");
assert_eq!(say(42), "Some files.");
assert_eq!(say(500), "Many files.");
```

An empty `Some(vec![])` means "matches none of these keys": only `*` variants are candidates, and no error is recorded.

## Compose with the built-ins

An operand that another function resolved arrives as `Value::Resolved`. Downcast it to read what it holds: `downcast::<NumberValue>` gives the number as given (`operand()`), as shown after rounding (`shown()`) and its options; `downcast::<DateTimeValue>` the date. `TextValue` is plain text.

```rust
use dxn_i18n::{NumberValue, TextValue, Value, downcast};

/// The digits of a number operand, whether it is a number, text, or another function's result.
fn digits(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) => Some(text.clone()),
        Value::Resolved(resolved) => downcast::<NumberValue>(resolved)
            .map(|number| number.shown().to_string())
            .or_else(|| downcast::<TextValue>(resolved).map(|text| text.0.clone())),
        Value::DateTime(_) => None,
    }
}

assert_eq!(digits(&Value::from(7)), Some("7".to_string()));
assert_eq!(digits(&Value::from("x")), Some("x".to_string()));
```

With that, `.local $n = {$x :number minimumFractionDigits=2}` followed by `{$n :my:thing}` hands your function the number already rounded and padded.

:::tip Test a custom function like a built-in
The Working Group's own test functions (`:test:function`, `:test:select`, `:test:format`) are registered the same way in `tests/conformance.rs`. [Testing](./testing.md) shows a small harness for your functions.
:::

Next: [Cookbook](./cookbook.md), or [Testing](./testing.md).
