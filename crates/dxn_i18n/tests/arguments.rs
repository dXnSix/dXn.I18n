//! The arguments a message reads: its `.input` declarations and the variables it references
//! anywhere (placeholders, selectors, options, markup) that no `.local` declares.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxn_i18n::Message;

fn arguments(source: &str) -> Vec<String> {
    Message::parse(source).unwrap().arguments()
}

#[test]
fn a_simple_message_reads_its_placeholders() {
    assert!(arguments("Hello!").is_empty());
    assert_eq!(arguments("Hello, {$name}!"), ["name"]);
    assert_eq!(
        arguments("{$b} and {$a :number} and {$b :string}"),
        ["a", "b"]
    );
    // Literals are no arguments.
    assert!(arguments("{|text|} {42 :number}").is_empty());
}

#[test]
fn options_and_markup_read_arguments_too() {
    assert_eq!(
        arguments("{$amount :currency currency=$code minimumFractionDigits=2}"),
        ["amount", "code"]
    );
    assert_eq!(
        arguments("{#link href=$url @title=|Help|}help{/link} {#img src=$src/}"),
        ["src", "url"]
    );
}

#[test]
fn declarations_add_inputs_and_hide_locals() {
    assert_eq!(arguments(".input {$count :number} {{Items}}"), ["count"]);
    assert_eq!(
        arguments(".local $total = {$price :number} {{{$total} for {$who}}}"),
        ["price", "who"]
    );
    assert_eq!(
        arguments(
            ".input {$count :integer}
             .local $shown = {$count :number minimumFractionDigits=$digits}
             .match $count
             one {{{$shown} item}}
             * {{{$shown} items}}"
        ),
        ["count", "digits"]
    );
}

#[test]
fn a_matcher_reads_its_selectors_and_every_variant() {
    assert_eq!(
        arguments(
            ".input {$n :number}
             .input {$gender :string}
             .match $n $gender
             one female {{{$name} has one cat}}
             * * {{{$name} has {$n} cats, {$extra}}}"
        ),
        ["extra", "gender", "n", "name"]
    );
}

#[test]
fn names_are_in_nfc() {
    // `é` as `e` and a combining accent reads as the composed `é`, as `Args` compares names.
    assert_eq!(arguments("{$cafe\u{301}} {$caf\u{e9}}"), ["caf\u{e9}"]);
}
