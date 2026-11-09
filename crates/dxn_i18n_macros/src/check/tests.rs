//! The checks' errors, on the fixture crates in `tests/fixtures/` (a proc-macro crate exports
//! only its macros, so what they report is tested here; `tests/checked.rs` builds checked
//! calls).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::quote;

use super::{closest, distance, expand_check, expand_t};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn t(fixture_name: &str, input: TokenStream) -> String {
    expand_t(input, Some(&fixture(fixture_name))).to_string()
}

/// The messages of the `compile_error!`s an expansion is, or the expansion when it is none.
fn errors(expansion: &str) -> Vec<String> {
    expansion
        .split("compile_error ! {")
        .skip(1)
        .map(|part| {
            let start = part.find('"').unwrap() + 1;
            let end = part.rfind('"').unwrap();
            part[start..end].replace("\\\"", "\"")
        })
        .collect()
}

fn error(expansion: &str) -> String {
    let errors = errors(expansion);
    assert_eq!(errors.len(), 1, "{expansion}");
    errors.into_iter().next().unwrap()
}

#[test]
fn a_call_that_checks_expands_to_dxn_i18n_t_with_its_tokens() {
    let expansion = t("app", quote!(translator, "cart.items", count = items.len()));
    assert!(errors(&expansion).is_empty(), "{expansion}");
    assert!(
        expansion
            .contains(":: dxn_i18n :: t ! (translator , \"cart.items\" , count = items . len ())"),
        "{expansion}"
    );
    // Every message file is included, so the crate rebuilds when one changes.
    assert_eq!(expansion.matches("include_bytes").count(), 5, "{expansion}");
    for file in [
        "de.json",
        "en.json",
        "hr.json",
        "checkout.json",
        "translation.json",
    ] {
        assert!(expansion.contains(file), "{file}: {expansion}");
    }
}

#[test]
fn every_way_of_naming_an_argument_counts() {
    for input in [
        quote!(translator, "hello", name),
        quote!(translator, "hello", name = "Ana"),
        quote!(translator, "hello", "name" = user.name()),
        quote!(translator, "hello", r#name = "Ana",),
        quote!(
            &*translator,
            "hello",
            name = HashMap::<String, u8>::new().len()
        ),
        quote!(translator, "hello", name = { |a: u8, b: u8| a + b }(1, 2)),
        quote!(translator, "greeting", name, gender = "female"),
        quote!(translator, "welcome"),
        quote!(translator, "welcome",),
    ] {
        let expansion = t("app", input);
        assert!(errors(&expansion).is_empty(), "{expansion}");
    }
}

#[test]
fn an_unknown_id_names_the_nearest_one() {
    assert_eq!(
        error(&t("app", quote!(translator, "cart.itme", count = 1))),
        "no message `cart.itme` in the default locale `en` (`assets/locales`); did you mean \
         `cart.items`?"
    );
    assert_eq!(
        error(&t("app", quote!(translator, "nothing.like.it"))),
        "no message `nothing.like.it` in the default locale `en` (`assets/locales`)"
    );
}

#[test]
fn namespaces_prefix_ids_except_the_default_one() {
    assert!(errors(&t("app", quote!(translator, "checkout.title"))).is_empty());
    assert!(errors(&t("app", quote!(translator, "welcome"))).is_empty());
    assert!(
        error(&t("app", quote!(translator, "translation.welcome")))
            .starts_with("no message `translation.welcome`")
    );
    // An id in another locale only is not in the default one.
    assert!(error(&t("no-default", quote!(translator, "hallo"))).contains("default locale"));
}

#[test]
fn a_missing_argument_is_an_error() {
    assert_eq!(
        error(&t("app", quote!(translator, "cart.total"))),
        "message `cart.total` needs the argument `$amount`"
    );
    // One a translation reads, not the default locale's message.
    assert_eq!(
        error(&t("app", quote!(translator, "greeting", name = "Ana"))),
        "message `greeting` needs the argument `$gender`, which its `hr` translation reads"
    );
}

#[test]
fn an_argument_no_translation_reads_is_an_error() {
    assert_eq!(
        error(&t("app", quote!(translator, "hello", name, nmae = "Ana"))),
        "message `hello` has no argument `$nmae`; it takes `$name`"
    );
    assert_eq!(
        error(&t(
            "app",
            quote!(translator, "greeting", name, gender, mood)
        )),
        "message `greeting` has no argument `$mood`; it takes `$gender` and `$name`"
    );
    assert_eq!(
        error(&t("app", quote!(translator, "welcome", name = "Ana"))),
        "message `welcome` has no argument `$name`; it takes no arguments"
    );
    assert_eq!(
        error(&t("app", quote!(translator, "hello", name, name = "Iva"))),
        "the argument `$name` is given twice"
    );
}

#[test]
fn every_error_of_a_call_is_reported() {
    let errors = errors(&t("app", quote!(translator, "greeting", nmae)));
    assert_eq!(
        errors,
        [
            "message `greeting` has no argument `$nmae`; it takes `$gender` and `$name`",
            "message `greeting` needs the argument `$gender`, which its `hr` translation reads",
            "message `greeting` needs the argument `$name`",
        ]
    );
}

#[test]
fn a_translation_that_does_not_parse_is_an_error() {
    let message = error(&t("app", quote!(translator, "status")));
    assert!(
        message.starts_with("message `status` in `assets/locales/de.json` does not parse: "),
        "{message}"
    );
}

#[test]
fn the_id_is_a_string_literal() {
    assert_eq!(
        error(&t("app", quote!(translator, id))),
        "a checked message id is a string literal; `dxn_i18n::t!` takes an id made at run time"
    );
    assert!(!errors(&t("app", quote!(translator))).is_empty());
}

#[test]
fn the_settings_default_to_locales_and_en() {
    assert!(errors(&t("plain", quote!(translator, "hello", name))).is_empty());
    // A locale's name in either spelling.
    assert!(errors(&t("regional", quote!(translator, "hello"))).is_empty());
}

#[test]
fn a_frameworks_defaults_come_before_the_crates_metadata() {
    let check = |fixture_name: &str, input: TokenStream| {
        expand_check(input, Some(&fixture(fixture_name))).to_string()
    };
    let expansion = check(
        "framework",
        quote!("hello", (), messages = "assets/i18n" => my::translate("hello")),
    );
    assert!(errors(&expansion).is_empty(), "{expansion}");
    assert!(
        expansion.contains("my :: translate (\"hello\")"),
        "{expansion}"
    );
    // The crate's metadata says `assets/locales`, whatever the framework's default.
    let expansion = check(
        "app",
        quote!("hello", (name = 1), messages = "nowhere", default_namespace = "x" => ()),
    );
    assert!(errors(&expansion).is_empty(), "{expansion}");
    assert!(
        error(&check("framework", quote!("hello", () => ())))
            .starts_with("no messages directory `locales` beside Cargo.toml")
    );
    assert_eq!(
        error(&check("framework", quote!("hello", (), folder = "x" => ()))),
        "`folder` is no setting; the settings are `messages`, `default_locale` and \
         `default_namespace`"
    );
    assert_eq!(
        errors(&check("app", quote!("hello", (nmae) => ()))),
        [
            "message `hello` has no argument `$nmae`; it takes `$name`",
            "message `hello` needs the argument `$name`",
        ]
    );
}

#[test]
fn broken_settings_and_files_are_errors() {
    assert!(
        error(&t("missing-dir", quote!(translator, "hello")))
            .starts_with("no messages directory `locales` beside Cargo.toml")
    );
    assert!(
        error(&t("broken-json", quote!(translator, "hello")))
            .starts_with("`locales/en.json` is not a JSON object of messages: ")
    );
    assert_eq!(
        error(&t("no-default", quote!(translator, "hello"))),
        "`locales` has no messages for the default locale `en`; set `default-locale` in \
         `[package.metadata.dxn_i18n]`"
    );
    assert_eq!(
        error(&t("bad-metadata", quote!(translator, "hello"))),
        "`package.metadata.dxn_i18n.messages` in Cargo.toml is not a string"
    );
    assert_eq!(
        error(&t("unknown-metadata", quote!(translator, "hello"))),
        "`package.metadata.dxn_i18n.directory` in Cargo.toml is no setting; the settings are \
         `messages`, `default-locale` and `default-namespace`"
    );
    let expansion = expand_t(quote!(translator, "hello"), None).to_string();
    assert!(error(&expansion).starts_with("CARGO_MANIFEST_DIR is not set"));
}

#[test]
fn a_changed_file_is_read_again() {
    let dir = std::env::temp_dir().join(format!("dxn_i18n_macros-{}", std::process::id()));
    fs::create_dir_all(dir.join("locales")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"changing\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    fs::write(dir.join("locales/en.json"), r#"{ "a": "A" }"#).unwrap();
    let check = |id: &str| expand_t(quote!(translator, #id), Some(&dir)).to_string();
    assert!(errors(&check("a")).is_empty());
    fs::write(dir.join("locales/en.json"), r#"{ "b": "Bee" }"#).unwrap();
    assert!(error(&check("a")).starts_with("no message `a`"));
    assert!(errors(&check("b")).is_empty());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn near_ids_are_typos() {
    assert_eq!(distance("kitten", "sitting"), 3);
    assert_eq!(distance("", "abc"), 3);
    assert_eq!(distance("čaša", "casa"), 2);
    let ids = ["cart.items".to_owned(), "cart.total".to_owned()];
    assert_eq!(closest("cart.itme", ids.iter()), Some("cart.items"));
    assert_eq!(closest("cart.totl", ids.iter()), Some("cart.total"));
    assert_eq!(closest("profile", ids.iter()), None);
}
