//! The default functions' options, and an app's own functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use dxn_i18n::{
    Args, FormatError, FunctionContext, FunctionError, FunctionRegistry, Message, MessageFormatter,
    Options, ResolvedValue, TextValue, Value,
};
use dxn_intl::{DateTime, parse_locale};

fn format(tag: &str, source: &str, args: Args) -> String {
    let formatted = MessageFormatter::new(parse_locale(tag).unwrap())
        .format(&Message::parse(source).unwrap(), &args);
    assert!(formatted.errors.is_empty(), "{:?}", formatted.errors);
    formatted.text
}

#[test]
fn number_options() {
    let n = || Args::new().with("n", 1234.5678);
    assert_eq!(format("en", "{$n :number}", n()), "1,234.568");
    assert_eq!(
        format("de", "{$n :number maximumFractionDigits=1}", n()),
        "1.234,6"
    );
    assert_eq!(
        format(
            "en",
            "{$n :number useGrouping=never minimumFractionDigits=5}",
            n()
        ),
        "1234.56780"
    );
    assert_eq!(
        format("en", "{$n :number signDisplay=always}", n()),
        "+1,234.568"
    );
    assert_eq!(format("en", "{$n :integer}", n()), "1,235");
    assert_eq!(
        format("en", "{7 :number minimumIntegerDigits=3}", Args::new()),
        "007"
    );
    let digits = Args::new().with("n", 2).with("digits", 2);
    assert_eq!(
        format("en", "{$n :number minimumFractionDigits=$digits}", digits),
        "2.00"
    );
}

#[test]
fn percent_and_currency() {
    assert_eq!(
        format("en", "{$n :percent}", Args::new().with("n", 0.256)),
        "26%"
    );
    assert_eq!(
        format(
            "en",
            "{$n :number style=percent}",
            Args::new().with("n", 0.5)
        ),
        "50%"
    );
    assert_eq!(
        format(
            "de-DE",
            "{$n :currency currency=EUR}",
            Args::new().with("n", 9.5)
        ),
        "9,50\u{a0}€"
    );
    assert_eq!(
        format(
            "en-US",
            "{$n :number style=currency currency=USD}",
            Args::new().with("n", 3)
        ),
        "$3.00"
    );
}

#[test]
fn dates_and_times() {
    let when = || Args::new().with("when", DateTime::new(2026, 10, 27, 15, 45, 30).unwrap());
    assert_eq!(format("en-US", "{$when :date}", when()), "Oct 27, 2026");
    assert_eq!(
        format("en-US", "{$when :date style=full}", when()),
        "Tuesday, October 27, 2026"
    );
    assert_eq!(
        format(
            "en-US",
            "{$when :date fields=month-day length=long}",
            when()
        ),
        "October 27"
    );
    assert_eq!(format("en-US", "{$when :time}", when()), "3:45\u{202f}PM");
    assert_eq!(
        format(
            "en-US",
            "{$when :time precision=second hour12=false}",
            when()
        ),
        "15:45:30"
    );
    assert_eq!(
        format(
            "de-DE",
            "{$when :datetime dateStyle=long timeStyle=short}",
            when()
        ),
        "27. Oktober 2026 um 15:45"
    );
    assert_eq!(
        format("en-US", "{|2026-01-02| :date length=short}", Args::new()),
        "1/2/26"
    );
    assert_eq!(
        format("en-US", "{|2026-01-02T09:05| :time}", Args::new()),
        "9:05\u{202f}AM"
    );
    // An unannotated date formats as a date and time.
    assert_eq!(
        format("en-US", "{$when}", when()),
        "Oct 27, 2026, 3:45\u{202f}PM"
    );
    // The locale's -u-hc- applies.
    assert_eq!(format("en-US-u-hc-h23", "{$when :time}", when()), "15:45");
}

#[test]
fn bad_operands_fall_back_and_bad_options_are_ignored() {
    let formatter = MessageFormatter::new(parse_locale("en").unwrap());
    let formatted = formatter.format(
        &Message::parse("{$n :number} {|2026-13-01| :date}").unwrap(),
        &Args::new().with("n", "abc"),
    );
    assert_eq!(formatted.text, "{$n} {|2026-13-01|}");
    assert!(
        formatted
            .errors
            .iter()
            .all(|error| matches!(error, FormatError::BadOperand { .. }))
    );
    // A bad option is reported and ignored; the value still formats.
    let formatted = formatter.format(
        &Message::parse("{$n :number select=sideways useGrouping=nope}").unwrap(),
        &Args::new().with("n", 1234),
    );
    assert_eq!(formatted.text, "1,234");
    assert_eq!(formatted.errors.len(), 2);
    assert!(
        matches!(&formatted.errors[0], FormatError::BadOption { option, .. } if option == "useGrouping")
    );
    let formatted = formatter.format(
        &Message::parse("{$n :currency}").unwrap(),
        &Args::new().with("n", 1),
    );
    assert!(
        matches!(&formatted.errors[0], FormatError::BadOperand { function, .. } if function == "currency")
    );
}

#[test]
fn offset_adds_and_subtracts() {
    assert_eq!(format("en", "{41 :offset add=1}", Args::new()), "42");
    assert_eq!(
        format(
            "en",
            ".local $n = {$x :number minimumFractionDigits=1} {{{$n :offset subtract=1}}}",
            Args::new().with("x", 3)
        ),
        "2.0"
    );
    let ordinal = ".input {$i :number select=ordinal} .local $n = {$i :offset add=1} .match $n one {{{$n}st}} two {{{$n}nd}} * {{{$n}th}}";
    let formatted = MessageFormatter::new(parse_locale("en").unwrap())
        .format(&Message::parse(ordinal).unwrap(), &Args::new().with("i", 1));
    // `select` set on an earlier expression cannot be selected on again.
    assert_eq!(formatted.text, "2th");
    assert!(matches!(
        formatted.errors[..],
        [FormatError::BadOption { .. }, FormatError::BadSelector(_)]
    ));
}

#[derive(Debug)]
struct Shout(String);

impl ResolvedValue for Shout {
    fn format(&self) -> Result<String, FunctionError> {
        Ok(self.0.to_uppercase())
    }
}

#[test]
fn an_apps_own_functions() {
    let mut functions = FunctionRegistry::default();
    functions.register(
        "my:shout",
        |_: &mut FunctionContext<'_>,
         operand: Option<&Value>,
         options: &Options|
         -> Result<Arc<dyn ResolvedValue>, FunctionError> {
            let text = operand
                .and_then(Value::as_str)
                .ok_or_else(|| FunctionError::BadOperand("text".into()))?;
            let mark = options.text("mark").unwrap_or_default();
            Ok(Arc::new(Shout(format!("{text}{mark}"))))
        },
    );
    let formatter =
        MessageFormatter::new(parse_locale("en").unwrap()).with_functions(Arc::new(functions));
    let message = Message::parse("{$x :my:shout mark=|!|}").unwrap();
    assert_eq!(
        formatter
            .format(&message, &Args::new().with("x", "hey"))
            .text,
        "HEY!"
    );
    // A custom value can be an argument too.
    let resolved: Arc<dyn ResolvedValue> = Arc::new(TextValue("ready".into()));
    let args = Args::new().with("x", Value::Resolved(resolved));
    assert_eq!(
        formatter
            .format(&Message::parse("{$x} {$x :string}").unwrap(), &args)
            .text,
        "ready ready"
    );
    assert!(FunctionRegistry::empty().get("number").is_none());
}
