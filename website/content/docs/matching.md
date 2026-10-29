---
title: Selecting variants
description: Choose between message variants with .match - plural and ordinal categories, exact numbers, text keys, several selectors, offsets and the ranking rules that pick the winner.
group: Messages
order: 30
---

A matcher chooses one of several patterns by the value of one or more variables. It is how a message says "1 item" and "3 items" in English, "1 artikl", "3 artikla" and "5 artikala" in Croatian, or picks a form by grammatical gender.

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
assert_eq!(message.format(&en, &Args::new().with("count", 0)), "Your cart is empty.");
assert_eq!(message.format(&en, &Args::new().with("count", 1)), "One item in your cart.");
assert_eq!(message.format(&en, &Args::new().with("count", 1200)), "1,200 items in your cart.");
```

`.match $count` names the selector. Each line after it is a *variant*: a key per selector, then a quoted pattern. Three rules are enforced when the message is parsed:

- Every selector must carry a function, either on its `.input`/`.local` declaration or through the declarations it refers to. `.match $count` with no annotation is `MissingSelectorAnnotation`.
- One variant must have only `*` keys. That is the fallback that always matches.
- Every variant has exactly one key per selector, and no two variants have the same keys.

The order of variants does not matter. Which one wins is decided by rank, described below.

## Plural categories

A number selects by its CLDR plural category: `zero`, `one`, `two`, `few`, `many` or `other`. Each language uses its own subset, so a Croatian message needs `few` where an English one does not. Use `*` for the "other" form.

| Language | Categories | Examples |
|---|---|---|
| `en`, `de` | `one`, `other` | 1 is `one` |
| `fr` | `one`, `many`, `other` | 0 and 1 are `one`, 1,000,000 is `many` |
| `hr` | `one`, `few`, `other` | 1 and 21 `one`, 2-4 and 22 `few`, 5 and 11 `other` |
| `pl` | `one`, `few`, `many`, `other` | 1 `one`, 2-4 `few`, 0, 5 and 21 `many` |
| `ru` | `one`, `few`, `many`, `other` | 1 and 21 `one`, 2-4 `few`, 0, 5 and 11 `many` |
| `ar` | all six | 0 `zero`, 1 `one`, 2 `two`, 3-10 `few`, 11-99 `many`, 100 `other` |
| `ja` | `other` | everything is `other` |

The rules come from CLDR through [dXn.Intl](https://dxnsix.github.io/dXn.Intl/). The categories are the same set for every message in a language, so a translator can read the table above to know which variants to write.

:::warning The category follows the digits shown
The number is rounded and padded as the function formats it, then selected. `{$n :number minimumFractionDigits=1}` turns 1 into `1.0`, which is `other` in English, not `one`. If a message shows "1.0 items", it must use the plural form for 1.0.
:::

### Exact numbers come first

A key that looks like a number matches that exact number before any category does. In the cart message, `0` wins for zero even where the language would call zero `other`, `many` or `one`. Keys compare numerically, so `1`, `1.0` and `|1|` are the same key.

To select by exact number only, write `select=exact`:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$n :number select=exact} .match $n one {{category}} 1 {{exact}} * {{other}}",
)?;
let en = parse_locale("en")?;
assert_eq!(message.format(&en, &Args::new().with("n", 1)), "exact");
assert_eq!(message.format(&en, &Args::new().with("n", 2)), "other");
```

## Ordinals

`select=ordinal` selects by ordinal category (1st, 2nd, 3rd). The categories are different from the cardinal ones: English uses `one` for 1, 21 and 31, `two` for 2 and 22, `few` for 3 and 23, and `other` for the rest, 11 to 13 included.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$place :number select=ordinal}
     .match $place
     one {{{$place}st}}
     two {{{$place}nd}}
     few {{{$place}rd}}
     *   {{{$place}th}}",
)?;
let en = parse_locale("en")?;
let places: Vec<String> = [1, 2, 3, 4, 11, 21, 22]
    .iter()
    .map(|n| message.format(&en, &Args::new().with("place", *n)))
    .collect();
assert_eq!(places, ["1st", "2nd", "3rd", "4th", "11th", "21st", "22nd"]);
```

`select` must be a literal on the expression that declares the selector (`select=ordinal`, never `select=$mode`). A variable there is reported as a bad option and the value cannot select. The same holds for `:integer`, which selects on its rounded value: `.input {$n :integer} .match $n one {{...}}` treats 1.2 as 1.

## Text keys

`:string` selects the variant whose key is the same text. Keys and values compare in Unicode NFC, so a composed and a decomposed accent match. The comparison is case-sensitive.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$gender :string}
     .match $gender
     female {{She replied.}}
     male   {{He replied.}}
     *      {{They replied.}}",
)?;
let en = parse_locale("en")?;
assert_eq!(message.format(&en, &Args::new().with("gender", "female")), "She replied.");
assert_eq!(message.format(&en, &Args::new().with("gender", "nonbinary")), "They replied.");
```

A key is a name or a number written bare, or a quoted literal `|two words|` for anything else. Text from users should not drive selection; pass a value your code chose, such as an enum turned into a fixed string.

## Several selectors

List more selectors after `.match` and give each variant one key per selector. The first selector is the most important: a variant that matches it better beats any variant that matches only later selectors better.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$gender :string}
     .input {$count :number}
     .match $gender $count
     female one {{She has one new message.}}
     female *   {{She has {$count} new messages.}}
     *      one {{They have one new message.}}
     *      *   {{They have {$count} new messages.}}",
)?;
let en = parse_locale("en")?;
let say = |gender: &str, count: i32| {
    message.format(&en, &Args::new().with("gender", gender).with("count", count))
};
assert_eq!(say("female", 1), "She has one new message.");
assert_eq!(say("female", 5), "She has 5 new messages.");
assert_eq!(say("male", 1), "They have one new message.");
assert_eq!(say("male", 2), "They have 2 new messages.");
```

A matcher needs the full grid you want to distinguish, because each variant is one cell. Translators may collapse cells with `*`, as the `female *` line does.

## Plurals with an offset

`:offset` adds or subtracts a whole number, so a message can say "Ana and 2 others". Select on the original count for the exact cases and on the offset value for the plural form of the rest:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse(
    ".input {$count :number}
     .local $rest = {$count :offset subtract=1}
     .match $count $rest
     0 * {{Nobody likes this.}}
     1 * {{{$name} likes this.}}
     * one {{{$name} and one other like this.}}
     * *   {{{$name} and {$rest} others like this.}}",
)?;
let en = parse_locale("en")?;
let say = |count: i32| message.format(&en, &Args::new().with("count", count).with("name", "Ana"));
assert_eq!(say(0), "Nobody likes this.");
assert_eq!(say(1), "Ana likes this.");
assert_eq!(say(2), "Ana and one other like this.");
assert_eq!(say(5), "Ana and 4 others like this.");
```

## How the winner is chosen

For each selector, the value lists the keys it matches, best first: exact numbers, then its plural or ordinal category (a number), or the equal text (a string). Then:

1. A variant is a candidate only if each of its keys is in that selector's list, or is `*`.
2. Each candidate gets a rank per selector: its key's position in the list, with `*` after every real match.
3. The candidate with the lowest first rank wins; ties go to the second selector, and so on.

Since two variants cannot have the same keys, there is always one winner.

## When a selector cannot select

If a selector is unresolved, is not a number for a number function, or comes from a function that cannot select, only `*` keys match it. The message still formats from its fallback variant and the error list gets a `BadSelector` entry, along with whatever caused it.

| Selector | Selects by |
|---|---|
| `:number`, `:integer`, `:percent` | exact numbers, then plural (default), ordinal or exact per `select` |
| `:string` | equal text |
| `:currency`, `:date`, `:time`, `:datetime` | nothing: only `*` keys match, and `BadSelector` is reported |
| `:offset` | the offset number, plural by default |
| your own function | whatever its `select` method returns ([Custom functions](./custom-functions.md)) |

A `select` set on one declaration cannot be selected on again by a later declaration that builds on it. After `.input {$i :number select=ordinal}`, matching on `.local $n = {$i :offset add=1}` reports a bad option and a `BadSelector`. Put `select=ordinal` after the offset instead: `.local $o = {$n :number select=ordinal}`, then `.match $o`.

Next: [Markup and parts](./markup-and-parts.md), or [Built-in functions](./functions.md) for every option a selector takes.
