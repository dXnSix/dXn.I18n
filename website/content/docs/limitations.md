---
title: Limitations
description: What dxn_i18n leaves out or does differently - ignored number options, currency digits, time zones, the f64 offset, default bidi, lookup rules - and what to do instead.
group: Reference
order: 160
---

What the library does not do, where it differs from what you might expect, and the alternative where there is one. Version 0.1.0.

## Left out

- **Significant digits, notation and compact numbers.** `minimumSignificantDigits`, `maximumSignificantDigits`, `notation`, `compactDisplay`, `currencyDisplay` and `currencySign` are accepted on `:number` and ignored, without an error. `1234.5` with `notation=compact` shows `1,234.5`. Format such a value in your code and pass the text.
- **Currency fraction digits and names.** A currency shows its own fraction digits (two for euros, none for yen) and its symbol. `minimumFractionDigits`, `maximumFractionDigits` and `useGrouping` do not apply, and there is no "euros" or "EUR" display.
- **Time zones.** A date is a wall time (`dxn_intl::DateTime`): no zone, no offset, no zone names. ISO 8601 text with `Z` or an offset is a bad operand. Convert instants to the user's local time before formatting ([Dates and times](./dates-and-times.md#there-is-no-time-zone)).
- **Relative times, lists, units and collation.** "In 3 days", "A, B and C", "5 km" and sorting rules are not provided. [dXn.Intl](https://dxnsix.github.io/dXn.Intl/), which does the formatting here, lists the same gaps.
- **Other message formats.** Messages are MessageFormat 2 only: no ICU MessageFormat 1 (`{n, plural, one {...}}`), gettext or Fluent syntax, and resources are JSON objects.
- **Loading and watching.** The crate takes JSON text and locale tags. Reading files or assets, following the system language and reloading on change belong to your app or to a framework wrapped around this crate.
- **`u:id` and attributes in parts.** `u:id` is accepted and ignored, and attributes (`@name`) are parsed but not part of `Part`.

## Behavior with caveats

- **Bidi isolation is off by default.** Turn on `BidiIsolation::Default` for mixed-direction text ([Text direction](./bidi.md)). `Message::format` never isolates.
- **`:offset` uses `f64`.** Integers above 2^53 can come out off by one or more. Use it for counts.
- **`select` is literal-only.** `select=$mode` is a bad option, because which forms a message has is the translator's decision.
- **Amounts of money and dates do not select.** A matcher on `:currency`, `:date`, `:time` or `:datetime` matches only `*`.
- **Values format in the message's language.** A fallback-language message formats numbers and dates in that language with the user's `-u-` preferences, not in the user's region ([The Translator](./translator.md#which-locale-formats-the-values)).
- **Script matters in the lookup.** `zh-TW` (Traditional) does not fall back to a `zh-Hans` bundle; add a `zh-Hant` bundle or list it in the fallbacks.
- **Parent lookup removes the region.** `de-AT` finds `de`; the chain is not configurable beyond the fallback list.
- **Number text is strict.** Text arguments for `:number` must be JSON number syntax. `" 5"`, `"+5"`, `".5"` and `"1_000"` are bad operands.
- **The category follows the digits shown.** `minimumFractionDigits=1` changes `1` to `1.0`, which is not `one` in English.
- **Duplicate ids.** Two JSON entries that make the same id (`"a.b"` and a nested `a` / `b`) replace each other.
- **Locale data is compiled in.** ICU4X's data for the formatters you use is part of your binary, and a program carries only the data of the formatters it uses ([dXn.Intl](https://dxnsix.github.io/dXn.Intl/)).
- **Debug builds log unknown currencies.** A well-formed currency code the data does not know formats as the code, and a debug build also prints an ICU4X data error to standard error.
- **Draft functions.** `:offset`, `:percent`, `:currency`, `:date`, `:time` and `:datetime` follow the specification's draft; their options can change with it.
- **No docs.rs page.** The crate is consumed as a git tag, so build the API reference locally with `cargo doc --open`.

## Not covered by tests

- Output depends on the CLDR data inside `dxn_intl`. The conformance suite uses `en-US` and a few locales; per-language plural, date and number output is `dxn_intl`'s and ICU4X's to prove.
- CI checks the `wasm32-unknown-unknown` build with `cargo check`; it does not run the library in a browser.

Next: [Releases](./releasing.md).
