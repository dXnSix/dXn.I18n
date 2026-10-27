//! Formatting patterns: declarations, fallbacks, markup, parts and bidi isolation.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxn_i18n::{
    Args, BidiIsolation, FormatError, MarkupKind, Message, MessageFormatter, Part, Value,
};
use dxn_intl::parse_locale;

fn en() -> MessageFormatter {
    MessageFormatter::new(parse_locale("en").unwrap())
}

fn format(source: &str, args: &Args) -> (String, Vec<FormatError>) {
    let formatted = en().format(&Message::parse(source).unwrap(), args);
    (formatted.text, formatted.errors)
}

#[test]
fn variables_and_literals() {
    let args = Args::new().with("name", "Ana").with("count", 1234.5);
    assert_eq!(
        format("Hi {$name}, {$count} and {|literal|} and {42}", &args).0,
        "Hi Ana, 1,234.5 and literal and 42"
    );
}

#[test]
fn declarations_compose() {
    let args = Args::new().with("price", 3);
    let (text, errors) = format(
        ".input {$price :number minimumFractionDigits=2}
         .local $double = {$price :number maximumFractionDigits=2}
         .local $copy = {$double}
         {{{$price} {$double} {$copy}}}",
        &args,
    );
    assert!(errors.is_empty(), "{errors:?}");
    // The local inherits `minimumFractionDigits=2` from the input.
    assert_eq!(text, "3.00 3.00 3.00");
}

#[test]
fn fallbacks_show_what_failed() {
    let (text, errors) = format(
        "Hello {$missing} {|lit| :nope} {:clock} {$name :number}",
        &Args::new().with("name", "x"),
    );
    assert_eq!(text, "Hello {$missing} {|lit|} {:clock} {$name}");
    assert_eq!(errors.len(), 4);
    assert_eq!(errors[0], FormatError::UnresolvedVariable("missing".into()));
    assert_eq!(errors[1], FormatError::UnknownFunction("nope".into()));
    assert!(
        matches!(errors[3], FormatError::BadOperand { ref function, .. } if function == "number")
    );
    // A local that failed falls back to its own name.
    let (text, _) = format(".local $x = {$missing :number} {{[{$x}]}}", &Args::new());
    assert_eq!(text, "[{$x}]");
}

#[test]
fn markup_is_kept_in_parts_and_left_out_of_text() {
    let message = Message::parse("Read {#link href=$url}the guide{/link}.").unwrap();
    let args = Args::new().with("url", "https://example.com");
    assert_eq!(en().format(&message, &args).text, "Read the guide.");
    let parts = en().format_to_parts(&message, &args).text;
    assert_eq!(parts[0], Part::Text("Read ".into()));
    assert_eq!(
        parts[1],
        Part::Markup {
            kind: MarkupKind::Open,
            name: "link".into(),
            options: vec![("href".into(), Value::String("https://example.com".into()))],
        }
    );
    assert!(matches!(
        &parts[3],
        Part::Markup {
            kind: MarkupKind::Close,
            ..
        }
    ));
}

#[test]
fn parts_name_the_function() {
    let message = Message::parse("{$n :integer} {$s}").unwrap();
    let parts = en()
        .format_to_parts(&message, &Args::new().with("n", 2.7).with("s", "x"))
        .text;
    assert_eq!(
        parts[0],
        Part::Value {
            function: Some("integer".into()),
            text: "3".into()
        }
    );
    assert_eq!(
        parts[2],
        Part::Value {
            function: None,
            text: "x".into()
        }
    );
}

#[test]
fn bidi_isolation() {
    let message = Message::parse("{$name} has {$n :number} points").unwrap();
    let args = Args::new().with("name", "שלום").with("n", 5);
    let isolated = en()
        .with_bidi_isolation(BidiIsolation::Default)
        .format(&message, &args)
        .text;
    // Text of unknown direction is isolated; an LTR number in an LTR message is not.
    assert_eq!(isolated, "\u{2068}שלום\u{2069} has 5 points");
    let arabic = MessageFormatter::new(parse_locale("ar").unwrap())
        .with_bidi_isolation(BidiIsolation::Default);
    let text = arabic
        .format(
            &Message::parse("{$n :number}").unwrap(),
            &Args::new().with("n", 5),
        )
        .text;
    assert!(text.starts_with('\u{2067}') && text.ends_with('\u{2069}'));
    assert_eq!(en().format(&message, &args).text, "שלום has 5 points");
}

#[test]
fn message_format_shortcut() {
    let message: Message = "Hi {$who}".parse().unwrap();
    assert_eq!(
        message.format(
            &parse_locale("en").unwrap(),
            &Args::new().with("who", "you")
        ),
        "Hi you"
    );
    assert_eq!(message.source(), "Hi {$who}");
}
