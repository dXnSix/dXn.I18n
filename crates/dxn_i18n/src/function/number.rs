//! `:number`, `:integer`, `:offset`, `:percent` and `:currency`.

use std::sync::Arc;

use dxn_intl::{
    Grouping, Locale, Number, NumberFormatter, NumberOptions, NumberStyle, PluralKind, PluralRules,
    SignDisplay, direction,
};

use super::{
    Function, FunctionContext, FunctionError, Options, ResolvedValue, TextValue, downcast,
};
use crate::Value;

#[derive(Clone, Copy, Debug)]
pub(super) enum NumberFunction {
    Number,
    Integer,
    Offset,
    Percent,
    Currency,
}

/// How a number selects a variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberSelect {
    /// By its cardinal plural category (`one`, `few`, …), after exact numbers. The default.
    Plural,
    /// By its ordinal category (`one` for 1st, `two` for 2nd, …), after exact numbers.
    Ordinal,
    /// By exact numbers only.
    Exact,
}

/// A number a numeric function resolved: the operand, its formatting options and how it
/// selects.
#[derive(Debug)]
pub struct NumberValue {
    locale: Locale,
    operand: Number,
    options: NumberOptions,
    /// `None` when the value cannot select: `select` came from a variable, or an operand
    /// already had it.
    select: Option<NumberSelect>,
    /// Whether this expression's own `select` literal set `select`.
    select_set: bool,
    shown: Number,
    text: String,
}

impl NumberValue {
    /// The number as given, before rounding.
    pub fn operand(&self) -> &Number {
        &self.operand
    }

    /// The number as shown: scaled, rounded and padded.
    pub fn shown(&self) -> &Number {
        &self.shown
    }

    /// The formatting options.
    pub fn options(&self) -> &NumberOptions {
        &self.options
    }

    /// How it selects variants; `None` when it cannot.
    pub fn selection(&self) -> Option<NumberSelect> {
        self.select
    }
}

impl ResolvedValue for NumberValue {
    fn format(&self) -> Result<String, FunctionError> {
        Ok(self.text.clone())
    }

    fn direction(&self) -> Option<dxn_intl::Direction> {
        Some(direction(&self.locale))
    }

    fn select(&self, keys: &[&str]) -> Option<Vec<usize>> {
        // Amounts of money do not select: which plural form fits "$1.00" is the currency's
        // business, not the number's.
        if matches!(self.options.style, NumberStyle::Currency(_)) {
            return None;
        }
        let select = self.select?;
        let shown = trimmed(&self.shown);
        let mut order: Vec<usize> = keys
            .iter()
            .enumerate()
            .filter(|(_, key)| parse_number_literal(key).is_some_and(|key| trimmed(&key) == shown))
            .map(|(index, _)| index)
            .collect();
        let kind = match select {
            NumberSelect::Exact => return Some(order),
            NumberSelect::Plural => PluralKind::Cardinal,
            NumberSelect::Ordinal => PluralKind::Ordinal,
        };
        let rules = PluralRules::new(&self.locale, kind).ok()?;
        let category = rules.category(self.shown.clone());
        order.extend(keys.iter().position(|key| *key == category.as_str()));
        Some(order)
    }
}

/// `number` without trailing fraction zeros or a sign on zero, for comparing with keys.
fn trimmed(number: &Number) -> String {
    let text = number.to_string();
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    match text {
        "-0" => "0".to_string(),
        other => other.trim_start_matches('+').to_string(),
    }
}

impl Function for NumberFunction {
    fn call(
        &self,
        context: &mut FunctionContext<'_>,
        operand: Option<&Value>,
        options: &Options,
    ) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
        let previous = match operand {
            Some(Value::Resolved(resolved)) => downcast::<NumberValue>(resolved),
            _ => None,
        };
        let (mut number, mut formatting, mut select) = match (operand, previous) {
            (_, Some(previous)) => {
                // A `select` an earlier expression set cannot be selected on again.
                let select = if previous.select_set {
                    None
                } else {
                    previous.select
                };
                if previous.select_set && !options.is_literal("select") {
                    context.report(FunctionError::bad_option(
                        "select",
                        "a literal on this expression",
                    ));
                }
                (previous.operand.clone(), previous.options.clone(), select)
            }
            (None, _) => {
                return Err(FunctionError::BadOperand(
                    "a numeric function needs an operand".to_string(),
                ));
            }
            (Some(Value::Number(number)), _) => (
                number.clone(),
                NumberOptions::default(),
                Some(NumberSelect::Plural),
            ),
            (Some(Value::String(text)), _) => (
                literal(text)?,
                NumberOptions::default(),
                Some(NumberSelect::Plural),
            ),
            (Some(Value::Resolved(resolved)), _) => match downcast::<TextValue>(resolved) {
                Some(TextValue(text)) => (
                    literal(text)?,
                    NumberOptions::default(),
                    Some(NumberSelect::Plural),
                ),
                None => {
                    return Err(FunctionError::BadOperand(
                        "the operand is not a number".to_string(),
                    ));
                }
            },
            (Some(Value::DateTime(_)), _) => {
                return Err(FunctionError::BadOperand(
                    "a date is not a number".to_string(),
                ));
            }
        };
        match self {
            Self::Number => {
                if let Some(style) = options.text("style") {
                    match style.as_str() {
                        "decimal" => formatting.style = NumberStyle::Decimal,
                        "percent" => formatting.style = NumberStyle::Percent,
                        "currency" => {
                            formatting.style =
                                NumberStyle::Currency(currency(options, &formatting)?)
                        }
                        _ => context.report(FunctionError::bad_option(
                            "style",
                            "decimal, percent or currency",
                        )),
                    }
                }
            }
            Self::Integer => {
                formatting.style = NumberStyle::Decimal;
                formatting.minimum_fraction_digits = Some(0);
                formatting.maximum_fraction_digits = Some(0);
            }
            Self::Offset => number = offset(&number, options)?,
            Self::Percent => formatting.style = NumberStyle::Percent,
            Self::Currency => {
                formatting.style = NumberStyle::Currency(currency(options, &formatting)?)
            }
        }
        if !matches!(self, Self::Integer | Self::Offset) {
            if let Some(digits) = digits(context, options, "minimumFractionDigits") {
                formatting.minimum_fraction_digits = Some(digits);
            }
            if let Some(digits) = digits(context, options, "maximumFractionDigits") {
                formatting.maximum_fraction_digits = Some(digits);
            }
        }
        if !matches!(self, Self::Offset) {
            if let Some(digits) = digits(context, options, "minimumIntegerDigits") {
                formatting.minimum_integer_digits = digits;
            }
            if let Some(grouping) = options.text("useGrouping") {
                match grouping.as_str() {
                    "auto" => formatting.grouping = Grouping::Auto,
                    "always" => formatting.grouping = Grouping::Always,
                    "never" | "false" => formatting.grouping = Grouping::Never,
                    "min2" => formatting.grouping = Grouping::Min2,
                    _ => context.report(FunctionError::bad_option(
                        "useGrouping",
                        "auto, always, never or min2",
                    )),
                }
            }
            if let Some(sign) = options.text("signDisplay") {
                match sign.as_str() {
                    "auto" => formatting.sign_display = SignDisplay::Auto,
                    "always" => formatting.sign_display = SignDisplay::Always,
                    "exceptZero" => formatting.sign_display = SignDisplay::ExceptZero,
                    "negative" => formatting.sign_display = SignDisplay::Negative,
                    "never" => formatting.sign_display = SignDisplay::Never,
                    _ => context.report(FunctionError::bad_option(
                        "signDisplay",
                        "auto, always, exceptZero, negative or never",
                    )),
                }
            }
        }
        let mut select_set = false;
        if !matches!(self, Self::Offset)
            && let Some(mode) = options.text("select")
        {
            if !options.is_literal("select") {
                // Which variants a message has is the translator's call, not the caller's.
                context.report(FunctionError::bad_option(
                    "select",
                    "a literal, not a variable",
                ));
                select = None;
            } else {
                match mode.as_str() {
                    "plural" => select = Some(NumberSelect::Plural),
                    "ordinal" => select = Some(NumberSelect::Ordinal),
                    "exact" => select = Some(NumberSelect::Exact),
                    _ => context.report(FunctionError::bad_option(
                        "select",
                        "plural, ordinal or exact",
                    )),
                }
                select_set = true;
            }
        }
        let formatter = NumberFormatter::new(context.locale(), formatting.clone())
            .map_err(|error| FunctionError::Failed(error.to_string()))?;
        let shown = formatter
            .resolve(number.clone())
            .map_err(|error| FunctionError::BadOperand(error.to_string()))?;
        let text = formatter
            .format(number.clone())
            .map_err(|error| FunctionError::BadOperand(error.to_string()))?;
        Ok(Arc::new(NumberValue {
            locale: context.locale().clone(),
            operand: number,
            options: formatting,
            select,
            select_set,
            shown,
            text,
        }))
    }
}

/// `:offset`'s result: the operand plus `add` or minus `subtract`, exactly one of them.
fn offset(number: &Number, options: &Options) -> Result<Number, FunctionError> {
    let amount = |name: &str| {
        options.text(name).map(|text| {
            text.parse::<u64>()
                .map_err(|_| FunctionError::bad_option(name, "a non-negative integer"))
        })
    };
    let delta: i128 = match (amount("add"), amount("subtract")) {
        (Some(add), None) => i128::from(add?),
        (None, Some(subtract)) => -i128::from(subtract?),
        _ => {
            return Err(FunctionError::bad_option(
                "add",
                "exactly one of `add` and `subtract`",
            ));
        }
    };
    let sum = number.to_f64() + delta as f64;
    // Through decimal text, so 1.5 + 1 stays 2.5 exactly for the digits a message shows.
    let digits = number
        .to_string()
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    format!("{sum:.digits$}")
        .parse()
        .map_err(|_| FunctionError::BadOperand("the sum is out of range".to_string()))
}

fn currency(options: &Options, formatting: &NumberOptions) -> Result<String, FunctionError> {
    match (options.text("currency"), &formatting.style) {
        (Some(code), _) if code.len() == 3 && code.bytes().all(|b| b.is_ascii_alphabetic()) => {
            Ok(code)
        }
        (Some(_), _) => Err(FunctionError::bad_option(
            "currency",
            "an ISO 4217 code such as EUR",
        )),
        (None, NumberStyle::Currency(code)) => Ok(code.clone()),
        (None, _) => Err(FunctionError::BadOperand(
            "a currency needs the `currency` option".to_string(),
        )),
    }
}

fn digits(context: &mut FunctionContext<'_>, options: &Options, name: &str) -> Option<u8> {
    let value = options.text(name)?;
    let digits = value.parse::<u8>().ok().filter(|digits| *digits <= 100);
    if digits.is_none() {
        context.report(FunctionError::bad_option(name, "an integer from 0 to 100"));
    }
    digits
}

fn literal(text: &str) -> Result<Number, FunctionError> {
    parse_number_literal(text)
        .ok_or_else(|| FunctionError::BadOperand(format!("`{text}` is not a number")))
}

/// Reads MessageFormat 2's `number-literal` (JSON's number syntax): `-?(0|[1-9][0-9]*)`, an
/// optional fraction `.[0-9]+` and an optional exponent `e[-+]?[0-9]+`.
pub(crate) fn parse_number_literal(text: &str) -> Option<Number> {
    let bytes = text.as_bytes();
    let mut i = usize::from(bytes.first() == Some(&b'-'));
    let digits = |i: &mut usize| {
        let start = *i;
        while bytes.get(*i).is_some_and(u8::is_ascii_digit) {
            *i += 1;
        }
        *i - start
    };
    let integer_start = i;
    let integer = digits(&mut i);
    if integer == 0 || (integer > 1 && bytes[integer_start] == b'0') {
        return None;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        if digits(&mut i) == 0 {
            return None;
        }
    }
    let mantissa = &text[..i];
    let mut exponent = 0i32;
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        let negative = bytes.get(i) == Some(&b'-');
        if matches!(bytes.get(i), Some(b'-' | b'+')) {
            i += 1;
        }
        let start = i;
        if digits(&mut i) == 0 {
            return None;
        }
        exponent = text[start..i].parse().ok().filter(|e: &i32| *e <= 1000)?;
        if negative {
            exponent = -exponent;
        }
    }
    if i != bytes.len() {
        return None;
    }
    shift(mantissa, exponent).parse().ok()
}

/// `mantissa` (as `-1.25`) times ten to the `exponent`, as decimal text.
fn shift(mantissa: &str, exponent: i32) -> String {
    if exponent == 0 {
        return mantissa.to_string();
    }
    let (sign, unsigned) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |rest| ("-", rest));
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = format!("{integer}{fraction}");
    let point = integer.len() as i32 + exponent;
    let text = if point <= 0 {
        format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        format!(
            "{}.{}",
            &digits[..point as usize],
            &digits[point as usize..]
        )
    };
    let trimmed = text.trim_start_matches('0');
    let text = if trimmed.is_empty() || trimmed.starts_with('.') {
        format!("0{trimmed}")
    } else {
        trimmed.to_string()
    };
    format!("{sign}{text}")
}
