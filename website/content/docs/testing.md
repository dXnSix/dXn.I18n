---
title: Testing and conformance
description: How dxn_i18n is tested - the Unicode MessageFormat Working Group's conformance suite, the feature test groups and CI - and how to test your own messages, plural forms and custom functions.
group: Reference
order: 150
---

The library's behavior is pinned by the Unicode MessageFormat Working Group's conformance suite plus a test file per feature. Your own messages deserve tests too: a plural form missing for one language is invisible until a user hits it.

## The conformance suite

`crates/dxn_i18n/tests/conformance/` holds the Working Group's JSON test files, copied unchanged from [unicode-org/message-format-wg](https://github.com/unicode-org/message-format-wg) (`test/tests/`) at commit `16372a6a02f41728be472a5921ba7c91774edeac`, under the Unicode License. `tests/conformance.rs` runs every file: 17 files with 462 test cases in this copy. It checks the formatted text and the *types* of the errors (`unresolved-variable`, `bad-operand`, `syntax-error` and so on), for syntax, the data model, pattern selection, fallbacks, bidi isolation, the `u:` options and each default function.

| Files | Cases |
|---|---|
| `syntax.json`, `syntax-errors.json` | 247 |
| `data-model-errors.json` | 23 |
| `pattern-selection.json`, `fallback.json` | 30 |
| `bidi.json`, `u-options.json` | 37 |
| `functions/` (`number`, `integer`, `offset`, `percent`, `currency`, `date`, `time`, `datetime`, `string`) | 125 |

The suite's own test functions (`:test:function`, `:test:select`, `:test:format`) are implemented in the runner as a custom `FunctionRegistry`, following the suite's README. The runner skips no case. All pass, and a case that fails fails the whole `the_working_groups_suite_passes` test with a list of every failure. To update, copy newer files over the directory, change the commit in `tests/conformance/README.md` and run `cargo test -p dxn_i18n --test conformance`.

## Feature tests and CI

Tests are grouped by feature, one file in `crates/dxn_i18n/tests/` each:

| File | Covers |
|---|---|
| `syntax.rs` | Parsing, escapes, options, attributes, markup, data-model rules (8 tests). |
| `formatting.rs` | Declarations, fallbacks, markup and parts, bidi isolation (7). |
| `selection.rs` | Plural, ordinal, exact, text and multi-selector matching (7). |
| `functions.rs` | Options of the default functions and custom functions (6). |
| `translator.rs` | Nested JSON, load errors, the lookup chain, user preferences (6). |
| `conformance.rs` | The suite above (1 test running all cases). |

The README's and rustdoc's examples run as 5 doc tests. Run one group with `cargo test -p dxn_i18n --test selection`. CI (`.github/workflows/ci.yml`) runs on Linux, macOS and Windows: `cargo fmt --check`, a check that no UI framework is in the dependency tree, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo check` for `wasm32-unknown-unknown`, and `cargo package --workspace --list`. Separate jobs build the rustdoc with missing-docs as an error and check the minimum Rust version, 1.97.

## Test your own messages

The pattern in the library's tests: format with a known locale and assert that no errors came out, so a bad option or a missing argument fails the test instead of passing with a fallback in the text.

```rust
use dxn_i18n::{Args, Message, MessageFormatter};
use dxn_intl::parse_locale;

fn render(tag: &str, source: &str, args: Args) -> String {
    let message = Message::parse(source).expect("the message parses");
    let formatted = MessageFormatter::new(parse_locale(tag).expect("a language tag")).format(&message, &args);
    assert!(formatted.errors.is_empty(), "{source}: {:?}", formatted.errors);
    formatted.text
}

assert_eq!(render("en", "Hi {$name}", Args::new().with("name", "Ana")), "Hi Ana");
```

Put calls like this in `#[test]` functions. For bundles, load the real resource files and render every message you care about, with `Translator::try_format` ([Errors and fallbacks](./errors.md#read-the-errors)); the [cookbook](./cookbook.md#check-translations-in-a-test) has a test that compares languages.

### Plural forms, per language

A table-driven test documents which forms a language needs and catches a missing one:

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let artikli = Message::parse(
    ".input {$n :number} .match $n 0 {{nema artikala}} one {{{$n} artikl}} few {{{$n} artikla}} * {{{$n} artikala}}",
)?;
let hr = parse_locale("hr")?;
for (n, expected) in [(0, "nema artikala"), (1, "1 artikl"), (3, "3 artikla"), (5, "5 artikala"), (21, "21 artikl"), (22, "22 artikla")] {
    assert_eq!(artikli.format(&hr, &Args::new().with("n", n)), expected);
}
```

### Custom functions

Register the function on a `FunctionRegistry`, build a `MessageFormatter` with it and assert both the text and `errors.is_empty()`, as in [Custom functions](./custom-functions.md). Also assert the failure paths: pass the wrong kind of operand and check for `FormatError::BadOperand`.

:::warning Expected strings depend on locale data
Dates, numbers and currencies come from CLDR data compiled into `dxn_intl`. Spaces matter (`\u{a0}` before `€`, `\u{202f}` before `PM`), and an upgrade of the `dxn_intl` tag can change a pattern. Write special spaces as escapes, and re-run the tests on every upgrade.
:::

Next: [Limitations](./limitations.md).
