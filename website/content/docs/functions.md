---
title: Built-in functions
description: "The nine functions messages can call (:string, :number, :integer, :offset, :percent, :currency, :date, :time and :datetime), the operands they take, how options and composition work, and the u: options."
group: Functions
order: 60
---

A function turns a value into text, and possibly into a selector. You write it after the operand in a placeholder: `{$count :number}`, `{$when :date style=long}`. `dxn_i18n` registers nine of them by default.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse("{$count} {$count :integer} {$ratio :percent} {$text :number}")?;
let args = Args::new().with("count", 2.6).with("ratio", 0.256).with("text", "1e3");
assert_eq!(message.format(&parse_locale("en")?, &args), "2.6 3 26% 1,000");
```

| Function | Operand | Formats | Selects |
|---|---|---|---|
| `:string` | any value except a date | The value as text, unlocalized. | Equal text. |
| `:number` | number, or text that is a number | A decimal number, or a percent or currency by `style`. [Numbers and money](./numbers.md) | Plural, ordinal or exact. |
| `:integer` | as `:number` | The number rounded to a whole number. | As `:number`, on the rounded value. |
| `:offset` | as `:number` | The number plus `add` or minus `subtract`. | As `:number`. |
| `:percent` | as `:number` | A fraction as a percentage: `0.256` is `26%`. | As `:number`. |
| `:currency` | as `:number` | An amount of money in the `currency` option. | Nothing. |
| `:date` | date, or ISO 8601 text | A date. [Dates and times](./dates-and-times.md) | Nothing. |
| `:time` | date and time, or ISO 8601 text | A time of day. | Nothing. |
| `:datetime` | as `:time` | A date, a time, or both. | Nothing. |

MF2 itself specifies `:string`, `:number` and `:integer`. The crate's own documentation calls `:offset`, `:percent`, `:currency`, `:date`, `:time` and `:datetime` the specification's draft functions, so their options can change when the specification does.

## Which function do I use

| You want | Use |
|---|---|
| A number in the user's separators | `{$n}` or `{$n :number}` (they are the same for a number argument) |
| At most N fraction digits, or always N | `:number maximumFractionDigits=N`, `minimumFractionDigits=N` |
| A whole number, rounded | `:integer` |
| A fraction as a percentage | `:percent` |
| Money | `:currency currency=EUR` |
| "N and M others" arithmetic | `:offset add=1`, `subtract=1` |
| A date | `:date`; with a time of day, `:datetime` |
| A time of day | `:time` |
| Select by exact text | `.input {$x :string}` then `.match $x` |
| One value in another locale | `u:locale=en` on any function |

## What each function accepts as an operand

Arguments are [`Value`s](./arguments.md): text, a number, a date or an already resolved value.

| Operand | `:number` family | `:date` family | `:string` |
|---|---|---|---|
| Number | Formats it. | Bad operand. | Its plain digits, not localized. |
| Text | Read as a number if it is a JSON number literal (`42`, `-1.5`, `1e3`); else bad operand. | Read as an ISO 8601 local date or date-time; else bad operand. | Used as is. |
| Date | Bad operand. | Formats it. | Bad operand. |
| An earlier function's result | Continues it (see below). | Continues it. | Its formatted text. |
| None (`{:number}`) | Bad operand. | Bad operand. | Bad operand. |

Number text must be JSON-shaped: no leading `+` or `.`, no spaces, no `0x`, no `_`. A bad operand is a `BadOperand` error and the placeholder shows its fallback (`{$n}`), as [Errors and fallbacks](./errors.md) describes.

### Placeholders without a function

A bare `{$value}` formats by the value's type: a number as `:number` with default options, a date as `:datetime` with default options, and text as itself. A number or date argument therefore already shows in the user's locale; add a function when you need options.

## Options

Options go after the function name as `name=value`, with a literal or a variable. An option the function does not know is ignored without an error, which is why options from other MF2 implementations that this crate does not implement (such as `notation`) do nothing. See [Limitations](./limitations.md).

| What goes wrong | What you get |
|---|---|
| A known option has a bad value (`useGrouping=nope`) | A `BadOption` error; the option is ignored and the value still formats. |
| The operand is unusable | A `BadOperand` error; the placeholder shows its fallback. |
| The options contradict each other (`minimumFractionDigits=3 maximumFractionDigits=1`) | A `Function` error; the placeholder shows its fallback. |

`select` is the one option that must be a **literal**: `select=ordinal` works, `select=$mode` does not. Which variants a message has is the translator's decision, not the caller's.

## Composition

A variable declared with a function keeps that function's options, and the next function builds on them:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$price :number minimumFractionDigits=2}
     .local $next = {$price :offset add=1}
     {{{$price} then {$next}}}",
)?;
let args = Args::new().with("price", 3);
assert_eq!(message.format(&parse_locale("en")?, &args), "3.00 then 4.00");
```

`:offset` keeps `minimumFractionDigits=2`, so `$next` shows "4.00". Composition works for the number family together, and for the date family together. Mixing them (a date into `:number`) is a bad operand.

## The u: options

Every function accepts three options in the reserved `u:` namespace:

| Option | Effect |
|---|---|
| `u:locale=tag` | Formats this placeholder for another locale than the message's. A bad tag is a `BadOption` error and the message's locale is used. |
| `u:dir=ltr\|rtl\|auto\|inherit` | The direction of the value, for [bidi isolation](./bidi.md). |
| `u:id=name` | Accepted for compatibility and ignored: parts do not carry an id. |

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse("{$n :number} / {$n :number u:locale=en}")?;
let args = Args::new().with("n", 1234.5);
assert_eq!(message.format(&parse_locale("de")?, &args), "1.234,5 / 1,234.5");
```

## :string

`:string` is for text. As a selector it matches the key that is the same text in Unicode NFC. As a formatter it shows the operand unchanged: a number passed through `:string` keeps its plain digits (`1234.5`, not `1,234.5`), which is what you want for an id or a year. Without an operand, or with a date, it is a bad operand.

## Unknown functions

A name no function has is an `UnknownFunction` error. A placeholder with an operand shows the operand's fallback (`{$n}`), one without shows `{:name}`. Add your own with `FunctionRegistry::register` ([Custom functions](./custom-functions.md)); `FunctionRegistry::empty()` leaves even the built-ins out.

Next: [Numbers and money](./numbers.md) and [Dates and times](./dates-and-times.md) list every option.
