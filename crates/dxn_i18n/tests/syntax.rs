//! Parsing MessageFormat 2 and checking the data model's rules.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxn_i18n::syntax::{
    Body, DataModelError, Declaration, Key, MarkupKind, Operand, ParseError, PatternPart, parse,
};

fn data_model_error(source: &str) -> DataModelError {
    match parse(source) {
        Err(ParseError::DataModel(error)) => error,
        other => panic!("expected a data model error for {source:?}, got {other:?}"),
    }
}

fn syntax_error(source: &str) {
    assert!(
        matches!(parse(source), Err(ParseError::Syntax { .. })),
        "expected a syntax error for {source:?}, got {:?}",
        parse(source)
    );
}

#[test]
fn simple_messages_keep_their_text() {
    let message = parse("  Hello, {$name}! ").unwrap();
    let Body::Pattern(pattern) = message.body else {
        panic!()
    };
    assert_eq!(pattern[0], PatternPart::Text("  Hello, ".to_string()));
    assert!(
        matches!(&pattern[1], PatternPart::Expression(e) if e.operand == Some(Operand::Variable("name".into())))
    );
    assert_eq!(pattern[2], PatternPart::Text("! ".to_string()));
    assert_eq!(parse("").unwrap().body, Body::Pattern(Vec::new()));
}

#[test]
fn escapes_and_quoted_literals() {
    let message = parse(r"a \{ b \} c \\ d {|x \| y|}").unwrap();
    let Body::Pattern(pattern) = message.body else {
        panic!()
    };
    assert_eq!(pattern[0], PatternPart::Text(r"a { b } c \ d ".to_string()));
    assert!(
        matches!(&pattern[1], PatternPart::Expression(e) if e.operand == Some(Operand::Literal("x | y".into())))
    );
    syntax_error(r"bad \n escape");
    syntax_error("a } alone");
}

#[test]
fn functions_options_and_attributes() {
    let message =
        parse("{$n :number minimumFractionDigits=2 signDisplay=$sign @translate=no}").unwrap();
    let Body::Pattern(pattern) = message.body else {
        panic!()
    };
    let PatternPart::Expression(expression) = &pattern[0] else {
        panic!()
    };
    let function = expression.function.as_ref().unwrap();
    assert_eq!(function.name, "number");
    assert_eq!(function.options.len(), 2);
    assert_eq!(function.options[1].value, Operand::Variable("sign".into()));
    assert_eq!(expression.attributes[0].name, "translate");
    // A function alone, a namespaced one, and numbers as unquoted literals.
    assert!(parse("{:my:clock}").is_ok());
    assert!(parse("{-1.5e+3 :number}").is_ok());
    syntax_error("{$n:number}");
    syntax_error("{$n :number digits = }");
}

#[test]
fn markup() {
    let message = parse("{#b}bold{/b} {#img src=|a.png| /}").unwrap();
    let Body::Pattern(pattern) = message.body else {
        panic!()
    };
    let kinds: Vec<_> = pattern
        .iter()
        .filter_map(|part| match part {
            PatternPart::Markup(markup) => Some((markup.kind, markup.name.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (MarkupKind::Open, "b"),
            (MarkupKind::Close, "b"),
            (MarkupKind::Standalone, "img")
        ]
    );
}

#[test]
fn complex_messages() {
    let message = parse(
        ".input {$count :number}
         .local $total = {$count :number minimumFractionDigits=1}
         {{Total: {$total}}}",
    )
    .unwrap();
    assert!(matches!(&message.declarations[0], Declaration::Input { name, .. } if name == "count"));
    assert!(matches!(&message.declarations[1], Declaration::Local { name, .. } if name == "total"));
    assert!(matches!(message.body, Body::Pattern(_)));
    assert!(parse("{{quoted {$x} pattern}}").is_ok());
    syntax_error(".local $x = {1} trailing text {{x}}");
    syntax_error(".input $x {{x}}");
}

#[test]
fn matchers() {
    let message = parse(
        ".input {$a :number} .input {$b :string}
         .match $a $b
         1 x {{one x}}
         one * {{one any}}
         * * {{other}}",
    )
    .unwrap();
    let Body::Matcher(matcher) = message.body else {
        panic!()
    };
    assert_eq!(matcher.selectors, ["a", "b"]);
    assert_eq!(matcher.variants.len(), 3);
    assert_eq!(
        matcher.variants[1].keys,
        [Key::Literal("one".into()), Key::CatchAll]
    );
    // Variants may follow each other without whitespace.
    assert!(parse(".input {$a :string} .match $a a {{A}}* {{other}}").is_ok());
    syntax_error(".input {$a :string} .match $a{{x}}");
}

#[test]
fn data_model_errors() {
    assert_eq!(
        data_model_error(".input {$a :string} .match $a a b {{x}} * {{y}}"),
        DataModelError::VariantKeyMismatch
    );
    assert_eq!(
        data_model_error(".input {$a :string} .match $a a {{x}}"),
        DataModelError::MissingFallbackVariant
    );
    assert_eq!(
        data_model_error(".input {$a} .match $a * {{x}}"),
        DataModelError::MissingSelectorAnnotation("a".into())
    );
    assert_eq!(
        data_model_error(".match $a * {{x}}"),
        DataModelError::MissingSelectorAnnotation("a".into())
    );
    assert_eq!(
        data_model_error(".input {$a :string} .input {$a :number} {{x}}"),
        DataModelError::DuplicateDeclaration("a".into())
    );
    assert_eq!(
        data_model_error(".local $b = {$a} .input {$a :number} {{x}}"),
        DataModelError::DuplicateDeclaration("a".into())
    );
    assert_eq!(
        data_model_error(".local $a = {$a :number} {{x}}"),
        DataModelError::DuplicateDeclaration("a".into())
    );
    assert_eq!(
        data_model_error("{$n :number style=percent style=decimal}"),
        DataModelError::DuplicateOptionName("style".into())
    );
    assert_eq!(
        data_model_error(".input {$a :string} .match $a a {{x}} a {{y}} * {{z}}"),
        DataModelError::DuplicateVariant
    );
}

#[test]
fn selectors_annotated_through_locals() {
    assert!(parse(".input {$a :number} .local $b = {$a} .match $b * {{x}}").is_ok());
}
