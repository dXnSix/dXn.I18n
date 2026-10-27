//! Bundles from JSON resources and the translator's locale chain.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxn_i18n::{Args, Bundle, LoadError, MessageError, MissingMessage, TranslateError, Translator};
use dxn_intl::parse_locale;

fn bundle(tag: &str, json: &str) -> Bundle {
    let mut bundle = Bundle::new(parse_locale(tag).unwrap());
    bundle.add_json(json).unwrap();
    bundle
}

fn translator() -> Translator {
    let mut translator = Translator::new(parse_locale("en").unwrap());
    translator.add_bundle(bundle(
        "en",
        r#"{"hello": "Hello", "only_en": "English", "price": "{$p :number}"}"#,
    ));
    translator.add_bundle(bundle(
        "de",
        r#"{"hello": "Hallo", "only_de": "Deutsch", "price": "{$p :number}"}"#,
    ));
    translator.add_bundle(bundle("de-AT", r#"{"hello": "Servus"}"#));
    translator.add_bundle(bundle("fr", r#"{"hello": "Bonjour"}"#));
    translator
}

#[test]
fn nested_resources_make_dotted_ids() {
    let bundle = bundle("en", r#"{"a": {"b": {"c": "deep"}, "d": "x"}}"#);
    let mut ids: Vec<_> = bundle.ids().collect();
    ids.sort();
    assert_eq!(ids, ["a.b.c", "a.d"]);
    let mut prefixed = Bundle::new(parse_locale("en").unwrap());
    prefixed
        .add_json_with_prefix("checkout", r#"{"title": "Checkout"}"#)
        .unwrap();
    assert!(prefixed.contains("checkout.title"));
}

#[test]
fn bad_messages_are_listed_and_the_rest_kept() {
    let mut bundle = Bundle::new(parse_locale("en").unwrap());
    let error = bundle
        .add_json(r#"{"ok": "fine", "broken": "{$x", "number": 3}"#)
        .unwrap_err();
    let LoadError::Messages(mut failures) = error else {
        panic!()
    };
    failures.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(failures[0].0, "broken");
    assert!(matches!(failures[0].1, MessageError::Parse(_)));
    assert_eq!(failures[1], ("number".to_string(), MessageError::NotText));
    assert!(bundle.contains("ok"));
    assert!(matches!(bundle.add_json("[1]"), Err(LoadError::Json(_))));
}

#[test]
fn lookup_walks_region_parent_fallbacks_and_default() {
    let mut translator = translator();
    translator.set_locale(parse_locale("de-AT").unwrap());
    let chain: Vec<String> = translator
        .lookup_chain()
        .iter()
        .map(|locale| locale.to_string())
        .collect();
    assert_eq!(chain, ["de-AT", "de", "en"]);
    assert_eq!(translator.format("hello", &Args::new()), "Servus");
    assert_eq!(translator.format("only_de", &Args::new()), "Deutsch");
    assert_eq!(translator.format("only_en", &Args::new()), "English");

    translator.set_fallbacks(vec![parse_locale("fr").unwrap()]);
    translator.set_locale(parse_locale("it").unwrap());
    let chain: Vec<String> = translator
        .lookup_chain()
        .iter()
        .map(|locale| locale.to_string())
        .collect();
    assert_eq!(chain, ["fr", "en"]);
    assert_eq!(translator.format("hello", &Args::new()), "Bonjour");
}

#[test]
fn values_format_in_the_users_region_or_the_bundles_language() {
    let mut translator = translator();
    let args = Args::new().with("p", 1234.5);
    translator.set_locale(parse_locale("de-CH").unwrap());
    // A `de` message in de-CH uses Swiss separators.
    assert_eq!(translator.format("price", &args), "1'234.5");
    translator.set_locale(parse_locale("it-u-nu-arab").unwrap());
    // An English message for an Italian user formats as English, with the user's digits.
    assert_eq!(translator.format("price", &args), "١٬٢٣٤٫٥");
}

#[test]
fn missing_messages_and_errors() {
    let mut translator = translator();
    assert_eq!(translator.format("nope", &Args::new()), "nope");
    translator.set_missing_message(MissingMessage::Empty);
    assert_eq!(translator.format("nope", &Args::new()), "");
    assert_eq!(
        translator.try_format("nope", &Args::new()),
        Err(TranslateError::Missing("nope".into()))
    );
    let error = translator.try_format("price", &Args::new()).unwrap_err();
    assert!(matches!(error, TranslateError::Format { ref text, .. } if text == "{$p}"));
    assert!(translator.contains("hello"));
    assert!(translator.format_to_parts("nope", &Args::new()).is_none());
}

#[test]
fn bundles_of_one_locale_merge() {
    let mut translator = Translator::new(parse_locale("en").unwrap());
    translator.add_bundle(bundle("en", r#"{"a": "A"}"#));
    translator.add_bundle(bundle("en", r#"{"b": "B"}"#));
    assert_eq!(translator.bundles().len(), 1);
    assert_eq!(translator.format("b", &Args::new()), "B");
}
