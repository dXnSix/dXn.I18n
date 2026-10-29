---
title: Message syntax
description: Write MessageFormat 2 messages - simple and complex messages, placeholders, literals, escapes, .input and .local declarations, options, attributes and the parse errors you can get.
group: Messages
order: 20
---

A message is a string in the MessageFormat 2 language: text with `{placeholders}` for the values that change. This page covers how a message is written and parsed; [Selecting variants](./matching.md) covers `.match`, and [Markup and parts](./markup-and-parts.md) covers `{#b}`.

## Simple messages

Most messages are text and placeholders. `Message::parse` checks the text, and `format` fills it in for a locale.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::parse_locale;

let message = Message::parse("Hello, {$name}! You have {$count} new messages.")?;
let args = Args::new().with("name", "Ana").with("count", 1200);
assert_eq!(message.format(&parse_locale("en")?, &args), "Hello, Ana! You have 1,200 new messages.");
assert_eq!(message.format(&parse_locale("de")?, &args), "Hello, Ana! You have 1.200 new messages.");
```

`1200` is a number, so it is shown in the locale's digits and separators even without a function. `Message::format` uses the default functions and no bidi isolation, and never fails: what cannot be resolved shows as a fallback. Use [`MessageFormatter`](./errors.md#read-the-errors) when you need the errors.

Messages also parse with `str::parse`: `let m: Message = "Hi {$who}".parse()?;`.

### Placeholders

| Placeholder | Meaning |
|---|---|
| `{$name}` | The argument or declared variable `name`. |
| `{$n :number}` | A variable run through a function. |
| `{\|two words\|}` | A quoted literal. `\|` and `\\` escape inside it. |
| `{42}`, `{-1.5e3}`, `{name}` | An unquoted literal: a number or a name. |
| `{\|2026-10-27\| :date}` | A literal run through a function. |
| `{:my:clock}` | A function with no operand. |
| `{#b}`, `{/b}`, `{#img/}` | Markup. |

:::warning A name without $ is text
`Hello {name}` formats as `Hello name`: an unquoted `name` is a literal, not the variable. Variables always start with `$`.
:::

### Text and escapes

Everything outside `{ }` is text. Escape the characters that have a meaning with a backslash: `\{`, `\}` and `\\`. A bare `}` in text is a syntax error. A simple message keeps its leading and trailing whitespace exactly.

A simple message cannot start with `.` or `{{`, because that makes it a complex message. Wrap such text in a quoted pattern: `{{.hidden file}}` formats as `.hidden file`.

## Complex messages

A message that starts with a declaration (`.input`, `.local`), `.match` or `{{` is complex. It has declarations, then one body: a quoted pattern `{{ ... }}` or a `.match` matcher.

```rust
use dxn_i18n::{Args, Message};
use dxn_intl::{DateTime, parse_locale};

let message = Message::parse(
    ".input {$when :datetime dateStyle=long}
     .local $count = {$items :integer}
     {{{$count} items ship on {$when}.}}",
)?;
let args = Args::new()
    .with("when", DateTime::date(2026, 10, 27)?)
    .with("items", 2.6);
assert_eq!(
    message.format(&parse_locale("en")?, &args),
    "3 items ship on October 27, 2026."
);
```

Whitespace between declarations, selectors, keys and quoted patterns is free, so a complex message fits on one line, which is how you write it in a JSON resource (JSON strings cannot contain raw line breaks). Inside `{{ }}` the text is exact, spaces included.

### Declarations

| Declaration | Effect |
|---|---|
| `.input {$name :fn options}` | Re-declares the argument `name` with a function. Later uses of `$name` are formatted that way, and a matcher can select on it. The operand must be the variable itself. |
| `.local $name = {...}` | Binds a new variable to an expression: a literal, a variable, or either run through a function. |

A declared variable carries its options forward. In `.local $n = {$x :number minimumFractionDigits=2}`, every `{$n}` shows two fraction digits, and `{$n :offset add=1}` keeps them.

Declarations run in order, and each variable is declared once. These are errors: a variable declared twice, a `.local` that uses its own name (`.local $x = {$x :number}`), and a declaration after an earlier one already read the variable. If a `.local` fails at format time, the placeholder shows the variable's own name as a fallback (`{$x}`).

## Options and attributes

Options follow the function name as `name=value`. A value is a literal or a variable (`maximumFractionDigits=$digits`). Spaces around `=` are allowed, names are case-sensitive and an option given twice is an error. Which options a function takes, and which must be literals, is on [Built-in functions](./functions.md).

Attributes (`@name` or `@name=value`) never change the output. They carry notes for tools, such as `{$brand @translate=no}`, and are available on the parsed message.

Function names may have a namespace: `:my:money`. Put your own functions in one so they never clash with future built-ins ([Custom functions](./custom-functions.md)). The `u:` namespace is reserved for the options `u:locale`, `u:dir` and `u:id`.

## Parse errors

`Message::parse` and `Bundle::add` report a `ParseError` (`Bundle::add_json` collects them in a [`LoadError`](./bundles.md#when-a-resource-is-wrong)). A `Syntax` error carries the byte offset and what was expected:

```rust
use dxn_i18n::{Message, ParseError};
use dxn_i18n::syntax::DataModelError;

let error = Message::parse("Hello {$name").unwrap_err();
assert!(matches!(error, ParseError::Syntax { offset: 12, .. }));
assert_eq!(error.to_string(), "syntax error at byte 12: expected `}`");

let error = Message::parse(".input {$n :number} .match $n one {{One}}").unwrap_err();
assert_eq!(error, ParseError::DataModel(DataModelError::MissingFallbackVariant));
```

A `DataModel` error means the text parses but breaks one of MessageFormat 2's rules, so it would not be safe to format:

| `DataModelError` | Cause |
|---|---|
| `VariantKeyMismatch` | A variant has more or fewer keys than the matcher has selectors. |
| `MissingFallbackVariant` | No variant has only `*` keys. |
| `MissingSelectorAnnotation(name)` | A selector has no function, directly or through its declarations. |
| `DuplicateDeclaration(name)` | A variable is declared twice, or after something used it. |
| `DuplicateOptionName(name)` | A function or markup gives an option twice. |
| `DuplicateVariant` | Two variants have the same keys (compared in Unicode NFC). |

Parsing is the only place these are caught. Once a `Message` exists, formatting it cannot fail with a syntax or data-model error.

## The parsed form

`Message::syntax()` returns the parsed message as plain data, the same names as the specification's data model: `MessageSyntax { declarations, body }`, where the body is a `Pattern` or a `Matcher`. Use it for tooling, such as listing the variables a translation uses:

```rust
use dxn_i18n::Message;
use dxn_i18n::syntax::{Body, Operand, PatternPart};

let message = Message::parse("Hi {$name}, you have {$count :number} items.")?;
let Body::Pattern(pattern) = &message.syntax().body else {
    unreachable!("a simple message is one pattern");
};
let mut names = Vec::new();
for part in pattern {
    if let PatternPart::Expression(expression) = part {
        if let Some(Operand::Variable(name)) = &expression.operand {
            names.push(name.as_str());
        }
    }
}
assert_eq!(names, ["name", "count"]);
assert_eq!(message.source(), "Hi {$name}, you have {$count :number} items.");
```

`dxn_i18n::syntax::parse(source)` gives the same `MessageSyntax` without wrapping it in a `Message`.

Next: [Selecting variants](./matching.md) for plurals and `.match`, or [Built-in functions](./functions.md) for what goes after the colon.
