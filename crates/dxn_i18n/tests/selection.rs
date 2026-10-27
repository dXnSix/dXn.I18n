//! Picking a matcher's variant: plurals, ordinals, exact numbers, text and several selectors.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxn_i18n::{Args, FormatError, Message, MessageFormatter};
use dxn_intl::parse_locale;

fn format(tag: &str, source: &str, args: Args) -> String {
    let formatted = MessageFormatter::new(parse_locale(tag).unwrap())
        .format(&Message::parse(source).unwrap(), &args);
    assert!(formatted.errors.is_empty(), "{:?}", formatted.errors);
    formatted.text
}

const CART: &str = ".input {$n :number}
.match $n
0 {{empty}}
one {{one item}}
few {{{$n} items (few)}}
many {{{$n} items (many)}}
* {{{$n} items}}";

#[test]
fn exact_numbers_beat_categories() {
    assert_eq!(format("en", CART, Args::new().with("n", 0)), "empty");
    assert_eq!(format("en", CART, Args::new().with("n", 1)), "one item");
    assert_eq!(format("en", CART, Args::new().with("n", 2)), "2 items");
}

#[test]
fn plural_categories_per_language() {
    assert_eq!(
        format("pl", CART, Args::new().with("n", 3)),
        "3 items (few)"
    );
    assert_eq!(
        format("pl", CART, Args::new().with("n", 5)),
        "5 items (many)"
    );
    assert_eq!(format("hr", CART, Args::new().with("n", 21)), "one item");
    // Fraction digits shown count: 1.0 is not "one" in English.
    let shown = CART.replace(":number}", ":number minimumFractionDigits=1}");
    assert_eq!(format("en", &shown, Args::new().with("n", 1)), "1.0 items");
    // Numbers given as text are read as numbers.
    assert_eq!(format("en", CART, Args::new().with("n", "1")), "one item");
}

#[test]
fn ordinals_and_exact_selection() {
    let ordinal = ".input {$n :number select=ordinal}
.match $n
one {{{$n}st}}
two {{{$n}nd}}
few {{{$n}rd}}
* {{{$n}th}}";
    let pick = |n: i32| format("en", ordinal, Args::new().with("n", n));
    assert_eq!(
        [pick(1), pick(2), pick(3), pick(4), pick(11), pick(22)],
        ["1st", "2nd", "3rd", "4th", "11th", "22nd"]
    );
    let exact =
        ".input {$n :number select=exact} .match $n one {{category}} 1 {{exact}} * {{other}}";
    assert_eq!(format("en", exact, Args::new().with("n", 1)), "exact");
    assert_eq!(format("en", exact, Args::new().with("n", 2)), "other");
}

#[test]
fn integer_selects_on_the_rounded_value() {
    let message = ".input {$n :integer} .match $n one {{one}} * {{other {$n}}}";
    assert_eq!(format("en", message, Args::new().with("n", 1.2)), "one");
    assert_eq!(format("en", message, Args::new().with("n", 2.6)), "other 3");
}

#[test]
fn text_selection() {
    let message = ".input {$gender :string}
.match $gender
female {{She replied.}}
male {{He replied.}}
* {{They replied.}}";
    assert_eq!(
        format("en", message, Args::new().with("gender", "female")),
        "She replied."
    );
    assert_eq!(
        format("en", message, Args::new().with("gender", "other")),
        "They replied."
    );
    // Keys and values match in NFC: é composed and decomposed.
    let nfc = ".input {$x :string} .match $x |\u{e9}| {{e-acute}} * {{other}}";
    assert_eq!(
        format("en", nfc, Args::new().with("x", "e\u{301}")),
        "e-acute"
    );
}

#[test]
fn several_selectors_rank_the_first_highest() {
    let message = ".input {$host :string} .input {$guests :number}
.match $host $guests
female 0 {{{$host} does not give a party.}}
female 1 {{{$host} invites one guest to her party.}}
female * {{{$host} invites {$guests} people to her party.}}
* 0 {{{$host} does not give a party.}}
* one {{{$host} invites one guest to their party.}}
* * {{{$host} invites {$guests} people to their party.}}";
    let args = |host: &str, guests: i32| {
        Args::new()
            .with("host", host)
            .with("gender", "x")
            .with("guests", guests)
    };
    assert_eq!(
        format("en", message, args("female", 1)),
        "female invites one guest to her party."
    );
    assert_eq!(
        format("en", message, args("Sam", 1)),
        "Sam invites one guest to their party."
    );
    assert_eq!(
        format("en", message, args("female", 4)),
        "female invites 4 people to her party."
    );
}

#[test]
fn a_selector_that_cannot_select_takes_the_fallback_variant() {
    let message = Message::parse(".input {$n :number} .match $n one {{one}} * {{other}}").unwrap();
    let formatted =
        MessageFormatter::new(parse_locale("en").unwrap()).format(&message, &Args::new());
    assert_eq!(formatted.text, "other");
    assert_eq!(
        formatted.errors,
        [
            FormatError::UnresolvedVariable("n".into()),
            FormatError::BadOperand {
                function: "number".into(),
                reason: "`$n` is not a number".into()
            },
            FormatError::BadSelector("n".into())
        ]
    );
}
