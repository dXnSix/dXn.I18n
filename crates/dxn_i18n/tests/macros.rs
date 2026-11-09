//! `args!` builds arguments that keep their types; `t!` formats a message with anything that
//! implements `Translate`: a translator, a reference or smart pointer to one, or a framework's
//! own service.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use dxn_i18n::{Args, Bundle, Translate, Translator, Value, args, t};
use dxn_intl::{DateTime, Number, parse_locale};

fn translator() -> Translator {
    let mut en = Bundle::new(parse_locale("en").unwrap());
    en.add_json(
        r#"{
          "hello": "Hello, {$name}!",
          "items": ".input {$count :number}\n.match $count\none {{One item}}\n* {{{$count} items}}",
          "profile": { "id": "User {$user-id} is {$type}." },
          "due": "Due {$when :date length=long}."
        }"#,
    )
    .unwrap();
    let mut translator = Translator::new(parse_locale("en").unwrap());
    translator.add_bundle(en);
    translator
}

#[test]
fn args_keep_their_types() {
    let name = "Ana";
    let when = DateTime::date(2026, 10, 27).unwrap();
    let args = args!(
        name,
        count = 3,
        ratio = 0.25,
        price = "19.99".parse::<Number>().unwrap(),
        when,
        "user-id" = 42u64,
        r#type = String::from("admin"),
    );
    assert_eq!(args.get("name").and_then(Value::as_str), Some("Ana"));
    assert_eq!(args.get("count"), Some(&Value::from(3)));
    assert_eq!(args.get("ratio"), Some(&Value::from(0.25)));
    assert_eq!(
        args.get("price"),
        Some(&Value::Number("19.99".parse().unwrap()))
    );
    assert_eq!(args.get("when"), Some(&Value::DateTime(when)));
    assert_eq!(args.get("user-id"), Some(&Value::from(42)));
    // A raw identifier names the variable without its `r#`.
    assert_eq!(args.get("type").and_then(Value::as_str), Some("admin"));
    assert_eq!(args.iter().count(), 7);
}

#[test]
fn args_is_the_builder_it_replaces() {
    assert_eq!(args!(), Args::new());
    assert_eq!(
        args!(name = "Ana", count = 3),
        Args::new().with("name", "Ana").with("count", 3)
    );
    // A later value for a name replaces the earlier one, as `Args::set` does.
    assert_eq!(args!(count = 1, count = 2), Args::new().with("count", 2));
    // Names compare in NFC, as variable names in messages do.
    let args = args!("cafe\u{301}" = 1);
    assert_eq!(args.get("caf\u{e9}"), Some(&Value::from(1)));
}

#[test]
fn args_values_are_expressions() {
    let items = ["a", "b", "c"];
    let user = ("Ana", 7);
    let args = args!(
        count = items.len(),
        name = user.0,
        total = if user.1 > 5 { 5 } else { 0 }
    );
    assert_eq!(args.get("count"), Some(&Value::from(3)));
    assert_eq!(args.get("name").and_then(Value::as_str), Some("Ana"));
    assert_eq!(args.get("total"), Some(&Value::from(5)));
    // The macro's own variable does not shadow the caller's.
    let args_count = {
        let args = 2;
        args!(count = args)
    };
    assert_eq!(args_count.get("count"), Some(&Value::from(2)));
}

#[test]
fn t_formats_with_a_translator() {
    let translator = translator();
    let name = "Ana";
    assert_eq!(t!(translator, "hello", name), "Hello, Ana!");
    assert_eq!(t!(translator, "hello", name = "Iva",), "Hello, Iva!");
    assert_eq!(t!(translator, "items", count = 1), "One item");
    assert_eq!(t!(translator, "items", count = 1200), "1,200 items");
    assert_eq!(
        t!(translator, "profile.id", "user-id" = 7, r#type = "an admin"),
        "User 7 is an admin."
    );
    assert_eq!(
        t!(
            translator,
            "due",
            when = DateTime::date(2026, 10, 27).unwrap()
        ),
        "Due October 27, 2026."
    );
    // What `Translator::format` shows: the id of a missing message, the fallback of a
    // missing argument.
    assert_eq!(t!(translator, "missing"), "missing");
    assert_eq!(t!(translator, "hello"), "Hello, {$name}!");
    // An id made at run time.
    let section = "profile";
    assert_eq!(
        t!(
            translator,
            &format!("{section}.id"),
            "user-id" = 1,
            r#type = "new"
        ),
        "User 1 is new."
    );
}

#[test]
fn t_takes_references_and_smart_pointers() {
    let translator = translator();
    let by_ref = &translator;
    assert_eq!(t!(by_ref, "items", count = 2), "2 items");
    assert_eq!(t!(&translator, "items", count = 2), "2 items");
    let shared = Arc::new(translator.clone());
    assert_eq!(t!(shared, "items", count = 2), "2 items");
    let local = Rc::new(translator.clone());
    assert_eq!(t!(local, "items", count = 2), "2 items");
    let boxed: Box<dyn Translate> = Box::new(translator.clone());
    assert_eq!(t!(boxed, "items", count = 2), "2 items");
    // A guard derefs to the translator.
    let cell = RefCell::new(translator);
    assert_eq!(t!(*cell.borrow(), "items", count = 2), "2 items");
    cell.borrow_mut().set_locale(parse_locale("en-IN").unwrap());
    assert_eq!(
        t!(*cell.borrow(), "items", count = 120000),
        "1,20,000 items"
    );
}

/// A framework's service: it holds a translator and decides the locale itself.
struct I18nService {
    translator: RefCell<Translator>,
    calls: RefCell<Vec<String>>,
}

impl Translate for I18nService {
    fn translate(&self, id: &str, args: &Args) -> String {
        self.calls.borrow_mut().push(id.to_string());
        self.translator.borrow().format(id, args)
    }
}

#[test]
fn t_takes_a_frameworks_service() {
    let service = Rc::new(I18nService {
        translator: RefCell::new(translator()),
        calls: RefCell::new(Vec::new()),
    });
    assert_eq!(t!(service, "hello", name = "Ana"), "Hello, Ana!");
    assert_eq!(t!(service, "items", count = 5), "5 items");
    assert_eq!(*service.calls.borrow(), ["hello", "items"]);

    fn greet(i18n: &impl Translate) -> String {
        t!(i18n, "hello", name = "Iva")
    }
    assert_eq!(greet(&service), "Hello, Iva!");
}
