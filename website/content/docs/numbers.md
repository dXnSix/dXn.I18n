---
title: Numbers and money
description: Options for :number, :integer, :percent, :currency and :offset - fraction and integer digits, grouping, sign display, rounding, currencies and exact decimal arguments.
group: Functions
order: 70
---

Number functions format in the message's locale: its digits, decimal and grouping separators, percent and currency patterns. Everything here is computed by [dXn.Intl](https://dxnsix.github.io/dXn.Intl/) from CLDR data.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse("{$n :number}")?;
let args = Args::new().with("n", 1234567.5);
let show = |tag: &str| message.format(&parse_locale(tag).unwrap(), &args);

assert_eq!(show("en"), "1,234,567.5");
assert_eq!(show("de"), "1.234.567,5");
assert_eq!(show("de-CH"), "1'234'567.5");
assert_eq!(show("hi"), "12,34,567.5");
assert_eq!(show("en-u-nu-arab"), "١,٢٣٤,٥٦٧.٥");
```

The last locale shows the user's digit preference as a `-u-nu-` extension on the locale ([The Translator](./translator.md#user-preferences)).

## :number

| Option | Values | Default |
|---|---|---|
| `style` | `decimal`, `percent`, `currency` | `decimal` |
| `currency` | An ISO 4217 code such as `EUR` (three ASCII letters). Required for `style=currency`. | none |
| `minimumIntegerDigits` | 0 to 100. Pads with leading zeros. | 1 |
| `minimumFractionDigits` | 0 to 100. Pads with trailing zeros. | 0 |
| `maximumFractionDigits` | 0 to 100. Rounds half away from zero. | 3 (0 for `percent`), never below the minimum |
| `useGrouping` | `auto`, `always`, `never` (`false` also means never), `min2` | `auto` |
| `signDisplay` | `auto`, `always`, `exceptZero`, `negative`, `never` | `auto` |
| `select` | `plural`, `ordinal`, `exact` (a literal only) | `plural` |

Each option takes a literal or a variable. A value outside the table is a `BadOption` error and the option is ignored.

```rust
use dxn_i18n::{Args, Message, MessageFormatter};
use dxn_intl::parse_locale;

let en = MessageFormatter::new(parse_locale("en")?);
let show = |source: &str, n: f64| {
    en.format(&Message::parse(source).unwrap(), &Args::new().with("n", n)).text
};

assert_eq!(show("{$n :number}", 1234.5678), "1,234.568");
assert_eq!(show("{$n :number maximumFractionDigits=1}", 1234.5678), "1,234.6");
assert_eq!(show("{$n :number minimumFractionDigits=2}", 3.0), "3.00");
assert_eq!(show("{$n :number minimumIntegerDigits=3}", 7.0), "007");
assert_eq!(show("{$n :number useGrouping=never}", 1234567.0), "1234567");
assert_eq!(show("{$n :number useGrouping=min2}", 1234.0), "1234");
assert_eq!(show("{$n :number useGrouping=min2}", 12345.0), "12,345");
assert_eq!(show("{$n :number signDisplay=always}", 5.0), "+5");
assert_eq!(show("{$n :number signDisplay=exceptZero}", 0.0), "0");
```

An option can come from a variable: `{$n :number minimumFractionDigits=$digits}` with `.with("digits", 2)`.

### Arguments and exact decimals

A float argument becomes the shortest decimal that reads back as the same float, so `0.1` shows as `0.1`. Numbers are exact decimals (`dxn_intl::Number`), and big integers keep every digit. For money, build the number from text so no float is involved:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{Number, parse_locale};

let price: Number = "19.99".parse()?;
let args = Args::new().with("price", price);
let message = Message::parse("{$price :currency currency=USD}")?;
assert_eq!(message.format(&parse_locale("en-US")?, &args), "$19.99");
```

Digits are not kept just because the argument has them: `"1.50"` shows as `1.5` unless you ask for `minimumFractionDigits=2`. The shown digits are also what plural selection reads, so `minimumFractionDigits` changes which variant wins ([Selecting variants](./matching.md#plural-categories)).

## :integer

`:integer` rounds to a whole number, half away from zero (`2.5` shows `3`). It takes `minimumIntegerDigits`, `useGrouping`, `signDisplay` and `select`; the fraction-digit options are ignored. It selects on the rounded value.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(".input {$n :integer} .match $n one {{one}} * {{other: {$n}}}")?;
let en = parse_locale("en")?;
assert_eq!(message.format(&en, &Args::new().with("n", 1.2)), "one");
assert_eq!(message.format(&en, &Args::new().with("n", 2.6)), "other: 3");
```

## :percent

The operand is a fraction: `0.256` shows as `26%`. The default is no fraction digits; `:number` options other than `style` apply.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let rounded = Message::parse("{$n :percent}")?;
let precise = Message::parse("{$n :percent maximumFractionDigits=1}")?;
let args = Args::new().with("n", 0.256);
assert_eq!(rounded.format(&parse_locale("en")?, &args), "26%");
assert_eq!(precise.format(&parse_locale("en")?, &args), "25.6%");
assert_eq!(rounded.format(&parse_locale("de")?, &args), "26\u{a0}%");
```

`{$n :number style=percent}` is the same thing. Selection reads the scaled value, so a `50` key matches `0.5`.

## :currency

`:currency` needs the `currency` option, and the operand is the amount in that currency's major unit (dollars, not cents).

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse("{$amount :currency currency=EUR}")?;
let args = Args::new().with("amount", 1234.5);
assert_eq!(message.format(&parse_locale("en")?, &args), "€1,234.50");
assert_eq!(message.format(&parse_locale("de-DE")?, &args), "1.234,50\u{a0}€");

let yen = Message::parse("{$amount :currency currency=JPY}")?;
assert_eq!(yen.format(&parse_locale("en")?, &args), "¥1,235");
```

What to know:

- The fraction digits are the currency's own (two for euros, none for yen) and `minimumFractionDigits` and `maximumFractionDigits` are ignored. `minimumIntegerDigits` and `signDisplay` apply, but `useGrouping` does not: a currency amount always groups as the locale does (`+€12,345.00` stays grouped with `useGrouping=never`).
- The currency is shown as its symbol in the locale's pattern. Options such as `currencyDisplay=name` are accepted and ignored.
- The code is case-insensitive and may come from a variable: `currency=$code`. A code that is not three letters is a `BadOption` error, and the placeholder shows its fallback.
- A well-formed code the locale data does not know shows as the code itself before the amount (`ZZZ 1,234.50`). `XXX` shows the generic symbol `¤`.
- An amount of money does not select. A matcher on a `:currency` value is a `BadSelector` error and only `*` matches. Select on the count, not the price.

## :offset

`:offset` adds or subtracts a whole number before the value is shown or selected. Give exactly one of `add` and `subtract`, each a non-negative integer.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse("{$n :offset add=1} {$n :offset subtract=3}")?;
let text = message.format(&parse_locale("en")?, &Args::new().with("n", 41));
assert_eq!(text, "42 38");
```

Both options, a negative amount, or neither is a `BadOption` error and the placeholder shows its fallback. `:offset` does its arithmetic in `f64`, so integers above 2^53 (9,007,199,254,740,992) can be off by one or more; use it for counts, not ids. For the "and N others" pattern see [Selecting variants](./matching.md#plurals-with-an-offset).

## Options this crate ignores

`:number` also accepts, and silently ignores, the options of the same MF2 function that this crate does not implement: `minimumSignificantDigits`, `maximumSignificantDigits`, `notation`, `compactDisplay`, `currencyDisplay`, `currencySign` and others. They produce no error, so a message written for another MF2 library formats, just without that option. Rounding is always half away from zero.

Next: [Dates and times](./dates-and-times.md), or [Arguments and values](./arguments.md) for how to pass numbers in.
