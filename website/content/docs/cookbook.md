---
title: Cookbook
description: Recipes for dxn_i18n - embedding translations, choosing the user's locale from a platform tag, checking translations in a test, sharing a translator between threads, and mapping names to translated labels.
group: Guides
order: 140
---

Short, complete answers to common tasks. Each snippet is a function you can paste next to your app code.

## Ship translations inside the binary

`include_str!` puts the JSON in the executable, which works on every target including WebAssembly, where there is no file system. Keep one file per language in a `locales/` directory beside `src/`.

```rust
use dxn_i18n::{Bundle, Translator};
use dxn_intl::parse_locale;

fn build_translator(user_tag: &str) -> Result<Translator, Box<dyn std::error::Error>> {
    let mut translator = Translator::new(parse_locale("en")?);
    for (tag, json) in [
        ("en", include_str!("../locales/en.json")),
        ("hr", include_str!("../locales/hr.json")),
    ] {
        let mut bundle = Bundle::new(parse_locale(tag)?);
        bundle.add_json(json)?;
        translator.add_bundle(bundle);
    }
    translator.set_locale(parse_locale(user_tag)?);
    Ok(translator)
}
```

To load from disk or the network instead, read the text yourself and call `add_json` with it. To add a language, drop in a file and one line in the list; a user whose locale has no bundle sees the default language.

## Take the user's language from the platform

The library reads no system settings. Ask the platform for the user's language tag (a BCP 47 tag, or a POSIX one like `pt_BR.UTF-8`) and fall back to the default when it is missing or unreadable:

```rust
use dxn_i18n::Translator;
use dxn_intl::{Locale, parse_locale};

/// `platform_tag` is what the OS or browser reported, if anything.
fn follow_the_system(translator: &mut Translator, platform_tag: Option<&str>) {
    let locale = platform_tag
        .and_then(|tag| parse_locale(tag).ok())
        .unwrap_or(Locale::UNKNOWN);
    translator.set_locale(locale);
}
```

`Locale::UNKNOWN` finds no bundle, so the lookup goes straight to the fallbacks and the default. Preferences such as the 24-hour clock arrive in the same tag (`en-US-u-hc-h23`); [The Translator](./translator.md#user-preferences) lists the extensions.

## Check translations in a test

The parser catches broken syntax when a resource loads, so loading every resource with `?` in a test already fails on a typo. Two more checks are cheap and catch what reviewers miss: ids that a language lacks, and variables a translation uses that the default language does not provide (these show as `{$name}` on screen).

```rust
use std::collections::BTreeSet;
use dxn_i18n::syntax::{Body, Declaration, Expression, Operand, PatternPart};
use dxn_i18n::{Bundle, Message};
use dxn_intl::parse_locale;

fn expression_variables(expression: &Expression, out: &mut BTreeSet<String>) {
    if let Some(Operand::Variable(name)) = &expression.operand {
        out.insert(name.clone());
    }
    for option in expression.function.iter().flat_map(|function| &function.options) {
        if let Operand::Variable(name) = &option.value {
            out.insert(name.clone());
        }
    }
}

/// The arguments a message needs: variables it reads that no `.local` declares.
fn needed_arguments(message: &Message) -> BTreeSet<String> {
    let syntax = message.syntax();
    let mut used = BTreeSet::new();
    let mut locals = BTreeSet::new();
    for declaration in &syntax.declarations {
        expression_variables(declaration.expression(), &mut used);
        if let Declaration::Local { name, .. } = declaration {
            locals.insert(name.clone());
        }
    }
    let mut patterns = Vec::new();
    match &syntax.body {
        Body::Pattern(pattern) => patterns.push(pattern),
        Body::Matcher(matcher) => {
            used.extend(matcher.selectors.iter().cloned());
            patterns.extend(matcher.variants.iter().map(|variant| &variant.pattern));
        }
    }
    for part in patterns.into_iter().flatten() {
        match part {
            PatternPart::Expression(expression) => expression_variables(expression, &mut used),
            PatternPart::Markup(markup) => {
                for option in &markup.options {
                    if let Operand::Variable(name) = &option.value {
                        used.insert(name.clone());
                    }
                }
            }
            PatternPart::Text(_) => {}
        }
    }
    used.difference(&locals).cloned().collect()
}

/// Problems with `translated` compared to the `default` language's bundle.
fn compare(default: &Bundle, translated: &Bundle) -> Vec<String> {
    let mut problems = Vec::new();
    let mut ids: Vec<&str> = default.ids().collect();
    ids.sort();
    for id in ids {
        let (Some(original), Some(message)) = (default.get(id), translated.get(id)) else {
            problems.push(format!("{id}: not translated"));
            continue;
        };
        let known = needed_arguments(original);
        for name in needed_arguments(message).difference(&known) {
            problems.push(format!("{id}: uses ${name}, which the default does not pass"));
        }
    }
    problems
}

let mut en = Bundle::new(parse_locale("en")?);
en.add_json(r#"{ "greeting": "Hello, {$name}!", "bye": "Goodbye", "items": ".input {$n :number} .match $n one {{{$n} item}} * {{{$n} items}}" }"#)?;
let mut hr = Bundle::new(parse_locale("hr")?);
hr.add_json(r#"{ "greeting": "Bok, {$nam}!", "items": ".input {$n :number} .match $n one {{{$n} artikl}} few {{{$n} artikla}} * {{{$n} artikala}}" }"#)?;

assert_eq!(
    compare(&en, &hr),
    ["bye: not translated", "greeting: uses $nam, which the default does not pass"]
);
```

In a real test, run `compare` for each language and `assert!(problems.is_empty(), "{problems:#?}")`. Add a test that calls `try_format` on each default-language message with sample arguments to catch bad options ([Errors and fallbacks](./errors.md#read-the-errors)).

## Share one translator between threads

`Translator::format` takes `&self` and the type is `Send + Sync`, so readers need no lock among themselves. Wrap it in an `RwLock` when something changes the language at runtime:

```rust
use std::sync::{Arc, RwLock};
use std::thread;
use dxn_i18n::{Args, Bundle, Translator};
use dxn_intl::parse_locale;

let mut bundle = Bundle::new(parse_locale("en")?);
bundle.add("hello", "Hello, {$name}!")?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(bundle);

let shared = Arc::new(RwLock::new(translator));

let reader = Arc::clone(&shared);
let worker = thread::spawn(move || {
    let translator = reader.read().expect("lock not poisoned");
    translator.format("hello", &Args::new().with("name", "worker"))
});
assert_eq!(worker.join().expect("worker finished"), "Hello, worker!");

shared.write().expect("lock not poisoned").set_locale(parse_locale("de")?);
```

## Turn a code or name into a translated label

Messages are looked up by id, so keep ids stable and derive them from your enum:

```rust
use dxn_i18n::{Args, Bundle, Translator};
use dxn_intl::parse_locale;

enum Status {
    Draft,
    Published,
}

impl Status {
    fn message_id(&self) -> &'static str {
        match self {
            Status::Draft => "status.draft",
            Status::Published => "status.published",
        }
    }
}

let mut en = Bundle::new(parse_locale("en")?);
en.add_json(r#"{ "status": { "draft": "Draft", "published": "Published" } }"#)?;
let mut translator = Translator::new(parse_locale("en")?);
translator.add_bundle(en);

assert_eq!(translator.format(Status::Published.message_id(), &Args::new()), "Published");
assert_eq!(translator.format(Status::Draft.message_id(), &Args::new()), "Draft");
```

A new variant without a message shows its id (`status.archived`), which is visible in development. For text that changes by value rather than by label, use a [matcher](./matching.md#text-keys) on a fixed string.

## Other tasks

| Task | Page |
|---|---|
| Bold text and links in a sentence | [Markup and parts](./markup-and-parts.md) |
| Show a Unix timestamp in the user's zone | [Dates and times](./dates-and-times.md#there-is-no-time-zone) |
| "Ana and 4 others" | [Selecting variants](./matching.md#plurals-with-an-offset) |
| Prices that never touch a float | [Numbers and money](./numbers.md#arguments-and-exact-decimals) |
| A format MF2 does not have | [Custom functions](./custom-functions.md) |
| Right-to-left text | [Text direction](./bidi.md) |
