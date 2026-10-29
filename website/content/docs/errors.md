---
title: Errors and fallbacks
description: Formatting never fails - a placeholder that cannot be resolved shows a fallback like {$name} - and how to read the FormatError list with MessageFormatter, Translator::try_format and TranslateError.
group: Translating
order: 120
---

A translation problem should not take down a screen. `Translator::format`, `Message::format` and the formatters' `text` always produce a string: what cannot be resolved shows as a **fallback** that names it, and the error is recorded next to the text.

```rust
use dxn_i18n::{Args, FormatError, Message, MessageFormatter};
use dxn_intl::parse_locale;

let message = Message::parse("Hello {$name}, you have {$n :number} messages.")?;
let formatter = MessageFormatter::new(parse_locale("en")?);
let formatted = formatter.format(&message, &Args::new().with("n", "many"));

assert_eq!(formatted.text, "Hello {$name}, you have {$n} messages.");
assert_eq!(formatted.errors[0], FormatError::UnresolvedVariable("name".to_string()));
assert!(matches!(&formatted.errors[1], FormatError::BadOperand { function, .. } if function == "number"));
```

`Formatted<T>` has the formatted `text` and the `errors`, in the order they happened. One placeholder can raise several: an unresolved `$x` given to `:number` reports the variable and then a bad operand.

Two stages can fail. **Parsing** (`Message::parse`, `Bundle::add_json`) catches syntax and data-model errors once, when messages load ([Message syntax](./message-syntax.md#parse-errors)). **Formatting** reports what depends on the values passed in, which is what this page covers.

## Fallback text

A fallback is the placeholder in its source form, in braces, so a translator or developer can see what went wrong:

| Placeholder | Failed because | Shows |
|---|---|---|
| `{$name}` | No such argument | `{$name}` |
| `{$n :number}` | `$n` is not a number | `{$n}` |
| `{\|2026-13-01\| :date}` | Not a date | `{\|2026-13-01\|}` |
| `{:clock}` | No function `clock` | `{:clock}` |
| `{$x :nope}` | No function `nope` | `{$x}` |
| `{$x}` after `.local $x = {$missing :number}` | The declaration failed | `{$x}` |

A literal operand is shown quoted with `|` (and escaped), a variable with `$`, and a function without an operand with `:name`. In [parts](./markup-and-parts.md) a fallback is `Part::Fallback("$name")`, without the braces.

A selector that fails does not hide the message: only `*` variants match it, so the fallback variant is shown and a `BadSelector` error is recorded ([Selecting variants](./matching.md#when-a-selector-cannot-select)).

## FormatError

| Variant | Raised when | Example |
|---|---|---|
| `UnresolvedVariable(name)` | No argument or declaration has the name. | `{$name}` with no `name` argument |
| `UnknownFunction(name)` | No registered function has the name. | `{$n :nope}` |
| `BadOperand { function, reason }` | The function cannot take its operand: missing, the wrong kind, or itself a fallback. | `{$n :number}` with `"many"` |
| `BadOption { function, option, reason }` | An option's value is not one the function takes. The function ignored the option and still formatted. | `{$n :number useGrouping=nope}` |
| `BadSelector(name)` | The selector could not select; only `*` matched. | `.match` on a `:currency` value |
| `Function { function, reason }` | The function failed another way. | `minimumFractionDigits=3 maximumFractionDigits=1` |

`FormatError` implements `Display`, `Error`, `Clone` and `PartialEq`. Branch on the variant, never on the message text. `function` is the name without the colon (`number`), or `#name` for markup; errors about a `u:` option carry it with the colon (`:number`).

`BadOption` is the one variant that keeps the value: the placeholder is formatted without that option. `BadSelector` selects the `*` variant, and the rest replace the placeholder with its fallback.

## Read the errors

| Method | Returns | Use it for |
|---|---|---|
| `Translator::format(id, &args)` | `String` | Screens. Never fails. |
| `Translator::try_format(id, &args)` | `Result<String, TranslateError>` | Logging, tests, tools. |
| `Translator::format_to_parts(id, &args)` | `Option<Formatted<Vec<Part>>>` | Rich output; `None` when missing. |
| `MessageFormatter::format(&message, &args)` | `Formatted<String>` | One message, errors included. |

`try_format` is `Ok(text)` only when nothing went wrong. Otherwise `TranslateError` tells you which kind of failure it was, and the text is still there:

```rust
use dxn_i18n::{Args, Bundle, FormatError, TranslateError, Translator};
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add("price", "Price: {$amount :currency currency=EUR}")?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(bundle);

match translator.try_format("price", &Args::new()) {
    Ok(text) => println!("{text}"),
    Err(TranslateError::Format { id, text, errors }) => {
        assert_eq!(id, "price");
        assert_eq!(text, "Price: {$amount}");
        assert_eq!(errors[0], FormatError::UnresolvedVariable("amount".to_string()));
    }
    Err(TranslateError::Missing(id)) => panic!("{id} should exist"),
}
```

| `TranslateError` | Meaning |
|---|---|
| `Missing(id)` | No bundle in the [lookup chain](./translator.md#the-lookup-chain) has the message. |
| `Format { id, text, errors }` | The message formatted, with fallbacks for these errors. |

### Log in debug builds, stay quiet in release

A helper that returns the best text and reports the rest keeps the call sites short:

```rust
use dxn_i18n::{Args, TranslateError, Translator};

fn tr(translator: &Translator, id: &str, args: &Args) -> String {
    match translator.try_format(id, args) {
        Ok(text) => text,
        Err(TranslateError::Format { id, text, errors }) => {
            if cfg!(debug_assertions) {
                eprintln!("translation `{id}`: {errors:?}");
            }
            text
        }
        // `format` shows the id (or nothing) for a missing message.
        Err(TranslateError::Missing(_)) => translator.format(id, args),
    }
}
```

Translators are not the only ones who cause failures: a developer who renames an argument (`count` to `items`) breaks every message that uses the old name. The fallback `{$count}` on screen makes that visible, and the [test recipe](./cookbook.md#check-translations-in-a-test) turns it into a failing test.

## What does not fail

- A message missing in the user's language is not an error: the lookup continues to fallback locales and the default.
- An argument a message does not use is ignored.
- A bundle message that fails to parse is left out of the bundle and reported at load time ([Bundles and JSON](./bundles.md#when-a-resource-is-wrong)), so it is simply "missing" afterwards.

Next: [Custom functions](./custom-functions.md), or [Testing](./testing.md).
