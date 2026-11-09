//! Checked calls build and format: `t!` against `tests/locales` (this crate's
//! `[package.metadata.dxn_i18n]`), and a framework's own macro through `check!`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;

use dxn_i18n::{Bundle, Translator};
use dxn_i18n_macros::t;
use dxn_intl::parse_locale;

fn bundle(locale: &str, resources: &[(&str, &str)]) -> Bundle {
    let mut bundle = Bundle::new(parse_locale(locale).unwrap());
    for (namespace, json) in resources {
        bundle.add_json_with_prefix(namespace, json).unwrap();
    }
    bundle
}

fn translator() -> Translator {
    let mut translator = Translator::new(parse_locale("en").unwrap());
    translator.add_bundle(bundle(
        "en",
        &[
            ("", include_str!("locales/en.json")),
            ("", include_str!("locales/en/translation.json")),
            ("checkout", include_str!("locales/en/checkout.json")),
        ],
    ));
    translator.add_bundle(bundle("hr", &[("", include_str!("locales/hr.json"))]));
    translator
}

#[test]
fn checked_calls_format_as_dxn_i18n_t_does() {
    let mut translator = translator();
    let name = "Ana";
    assert_eq!(t!(translator, "hello", name), "Hello, Ana!");
    assert_eq!(t!(translator, "cart.items", count = 1), "One item");
    assert_eq!(t!(translator, "welcome"), "Welcome!");
    assert_eq!(
        t!(translator, "checkout.total", amount = 12.5),
        "Total: €12.50"
    );
    assert_eq!(
        t!(translator, "profile", "user-id" = 7, r#type = "an admin"),
        "User 7 is an admin."
    );
    // `hr`'s greeting reads `$gender`, so every call passes it.
    assert_eq!(
        t!(translator, "greeting", name, gender = "female"),
        "Hi, Ana."
    );
    translator.set_locale(parse_locale("hr").unwrap());
    assert_eq!(
        t!(translator, "greeting", name, gender = "female"),
        "Bok, Ana, dobro došla."
    );
    assert_eq!(t!(&translator, "cart.items", count = 3), "3 artikla");
}

thread_local! {
    static TRANSLATOR: RefCell<Translator> = RefCell::new(translator());
}

/// A framework's macro: its own translator, the id and arguments checked by `check!`, and
/// defaults the crate's metadata overrides.
macro_rules! tr {
    ($id:literal $(, $($args:tt)*)?) => {
        dxn_i18n_macros::check!(
            $id, ($($($args)*)?), messages = "assets/locales" =>
            TRANSLATOR.with_borrow(|translator| dxn_i18n::t!(translator, $id $(, $($args)*)?))
        )
    };
}

#[test]
fn a_frameworks_macro_checks_through_check() {
    assert_eq!(tr!("hello", name = "Iva"), "Hello, Iva!");
    assert_eq!(tr!("cart.items", count = 1200), "1,200 items");
    assert_eq!(tr!("checkout.title"), "Checkout");
}
