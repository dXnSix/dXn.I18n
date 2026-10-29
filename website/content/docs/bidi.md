---
title: Text direction
description: Keep placeholders from reordering the text around them in mixed left-to-right and right-to-left text with MessageFormat 2's default bidi isolation strategy and the u:dir option.
group: Messages
order: 50
---

When a Hebrew name sits inside an English sentence, or a number inside an Arabic one, the Unicode bidirectional algorithm can move the punctuation and words next to it to the wrong side. Bidi isolation wraps each placeholder in invisible control characters so it cannot affect its neighbors.

It is **off by default**. Turn it on for the formatter or the translator:

```rust
use dxn_i18n::{Args, BidiIsolation, Message, MessageFormatter};
use dxn_intl::parse_locale;

let message = Message::parse("{$name} has {$points :number} points")?;
let args = Args::new().with("name", "שלום").with("points", 5);

let plain = MessageFormatter::new(parse_locale("en")?);
assert_eq!(plain.format(&message, &args).text, "שלום has 5 points");

let isolated = plain.with_bidi_isolation(BidiIsolation::Default);
assert_eq!(isolated.format(&message, &args).text, "\u{2068}שלום\u{2069} has 5 points");
```

On a translator, call `translator.set_bidi_isolation(BidiIsolation::Default)`. `Message::format` always uses `BidiIsolation::None`.

## What gets wrapped

With `BidiIsolation::Default`, a placeholder's text is wrapped according to the direction of its value and of the message. The message's direction is its locale's script direction (`ar` and `he` are right to left).

| Value | In a left-to-right message | In a right-to-left message |
|---|---|---|
| Text (arguments, literals, `:string`): direction unknown | `U+2068` FSI ... `U+2069` PDI | FSI ... PDI |
| Number or date: the direction of the locale it is formatted in | not wrapped | `U+2067` RLI ... PDI |
| Forced with `u:dir=ltr` or `rtl` | LRI or RLI ... PDI | LRI or RLI ... PDI |

So in an LTR message only text arguments are wrapped, and in an RTL message every placeholder is. A number formatted with `u:locale` of the other direction is wrapped in either kind of message. Fallbacks (`{$name}`) are never wrapped, and the isolates are part of the `text` of a `Part::Value` in [parts](./markup-and-parts.md).

```rust
use dxn_i18n::{Args, BidiIsolation, Message, MessageFormatter};
use dxn_intl::parse_locale;

let ar = MessageFormatter::new(parse_locale("ar")?).with_bidi_isolation(BidiIsolation::Default);
let message = Message::parse("{$n :number}")?;
let text = ar.format(&message, &Args::new().with("n", 5)).text;
assert_eq!(text, "\u{2067}5\u{2069}");
```

## Choosing the direction yourself

The `u:dir` option on a function sets the direction of its value: `ltr`, `rtl`, `auto` (the renderer decides, FSI) or `inherit` (the default). Use it where you know better, such as a phone number inside Arabic text, or a user-written name:

```rust
use dxn_i18n::{Args, BidiIsolation, Message, MessageFormatter};
use dxn_intl::parse_locale;

let ar = MessageFormatter::new(parse_locale("ar")?).with_bidi_isolation(BidiIsolation::Default);
let message = Message::parse("{$phone :string u:dir=ltr}")?;
let text = ar.format(&message, &Args::new().with("phone", "+385 1 234 5678")).text;
assert_eq!(text, "\u{2066}+385 1 234 5678\u{2069}");
```

An unknown value is a `BadOption` error and is treated as `inherit`. `u:dir` is an error on markup. Without bidi isolation turned on, the option has no effect on the text.

## Text direction in your UI

Isolation protects a sentence from its parts; it does not set the direction of the whole line. For that, ask `dxn_intl` for the locale's direction and set your widget's or HTML element's `dir` from it:

```rust
use dxn_intl::{Direction, direction, parse_locale};

assert_eq!(direction(&parse_locale("ar")?), Direction::Rtl);
assert_eq!(direction(&parse_locale("hr")?).as_str(), "ltr");
```

`dxn_intl` is documented at [dXn.Intl](https://dxnsix.github.io/dXn.Intl/).

Next: [Built-in functions](./functions.md).
