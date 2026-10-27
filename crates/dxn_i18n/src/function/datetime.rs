//! `:date`, `:time` and `:datetime`.

use std::sync::Arc;

use dxn_intl::{
    DateFields, DateTime, DateTimeFormatter, DateTimeOptions, HourCycle, Length, Locale,
    TimePrecision, direction,
};

use super::{
    Function, FunctionContext, FunctionError, Options, ResolvedValue, TextValue, downcast,
};
use crate::Value;

#[derive(Clone, Copy, Debug)]
pub(super) enum DateTimeFunction {
    Date,
    Time,
    DateTime,
}

/// A date or time a date function resolved.
#[derive(Debug)]
pub struct DateTimeValue {
    locale: Locale,
    value: DateTime,
    options: DateTimeOptions,
    text: String,
}

impl DateTimeValue {
    /// The date and time of day as given.
    pub fn value(&self) -> &DateTime {
        &self.value
    }

    /// The formatting options.
    pub fn options(&self) -> &DateTimeOptions {
        &self.options
    }
}

impl ResolvedValue for DateTimeValue {
    fn format(&self) -> Result<String, FunctionError> {
        Ok(self.text.clone())
    }

    fn direction(&self) -> Option<dxn_intl::Direction> {
        Some(direction(&self.locale))
    }
}

impl Function for DateTimeFunction {
    fn call(
        &self,
        context: &mut FunctionContext<'_>,
        operand: Option<&Value>,
        options: &Options,
    ) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
        let locale = context.locale().clone();
        let value = match operand {
            None => {
                return Err(FunctionError::BadOperand(
                    "a date function needs an operand".to_string(),
                ));
            }
            Some(Value::DateTime(value)) => *value,
            Some(Value::String(text)) => parse(text)?,
            Some(Value::Number(_)) => {
                return Err(FunctionError::BadOperand(
                    "a number is not a date".to_string(),
                ));
            }
            Some(Value::Resolved(resolved)) => {
                if let Some(previous) = downcast::<DateTimeValue>(resolved) {
                    previous.value
                } else if let Some(TextValue(text)) = downcast::<TextValue>(resolved) {
                    parse(text)?
                } else {
                    return Err(FunctionError::BadOperand(
                        "the operand is not a date".to_string(),
                    ));
                }
            }
        };
        let mut formatting = DateTimeOptions {
            date_fields: None,
            time_precision: None,
            length: Length::Medium,
            hour_cycle: None,
        };
        match self {
            Self::Date => date_options(
                context,
                &mut formatting,
                options,
                "fields",
                &["style", "length"],
            ),
            Self::Time => time_options(context, &mut formatting, options, "precision", "style"),
            Self::DateTime => {
                let has = |names: &[&str]| names.iter().any(|name| options.get(name).is_some());
                let date = has(&["dateStyle", "dateFields", "dateLength"]);
                let time = has(&["timeStyle", "timePrecision"]);
                if date || !time {
                    date_options(
                        context,
                        &mut formatting,
                        options,
                        "dateFields",
                        &["dateStyle", "dateLength"],
                    );
                }
                if time || !date {
                    time_options(
                        context,
                        &mut formatting,
                        options,
                        "timePrecision",
                        "timeStyle",
                    );
                }
            }
        }
        if let Some(hour12) = options.text("hour12") {
            match hour12.as_str() {
                "true" => formatting.hour_cycle = Some(HourCycle::H12),
                "false" => formatting.hour_cycle = Some(HourCycle::H23),
                _ => context.report(bad_option("hour12", "true or false")),
            }
        }
        if let Some(cycle) = options.text("hourCycle") {
            match cycle.parse() {
                Ok(cycle) => formatting.hour_cycle = Some(cycle),
                Err(_) => context.report(bad_option("hourCycle", "h11, h12, h23 or h24")),
            }
        }
        let formatter = DateTimeFormatter::new(&locale, formatting)
            .map_err(|error| FunctionError::Failed(error.to_string()))?;
        let text = formatter
            .format(&value)
            .map_err(|error| FunctionError::BadOperand(error.to_string()))?;
        Ok(Arc::new(DateTimeValue {
            locale,
            value,
            options: formatting,
            text,
        }))
    }
}

/// The date part: `fields` (as `year-month-day-weekday`) and a style (`full`, `long`, `medium`,
/// `short`; `full` adds the weekday). Medium year-month-day by default.
fn date_options(
    context: &mut FunctionContext<'_>,
    formatting: &mut DateTimeOptions,
    options: &Options,
    fields_option: &str,
    style_options: &[&str],
) {
    let mut fields = DateFields::YearMonthDay;
    let style = style_options
        .iter()
        .find_map(|name| options.text(name).map(|value| (*name, value)));
    if let Some((name, style)) = style {
        match style.as_str() {
            "full" => {
                fields = DateFields::YearMonthDayWeekday;
                formatting.length = Length::Long;
            }
            "long" => formatting.length = Length::Long,
            "medium" => formatting.length = Length::Medium,
            "short" => formatting.length = Length::Short,
            _ => context.report(bad_option(name, "full, long, medium or short")),
        }
    }
    if let Some(text) = options.text(fields_option) {
        match text.as_str() {
            "day" => fields = DateFields::Day,
            "month-day" => fields = DateFields::MonthDay,
            "year-month-day" => fields = DateFields::YearMonthDay,
            "day-weekday" => fields = DateFields::DayWeekday,
            "month-day-weekday" => fields = DateFields::MonthDayWeekday,
            "year-month-day-weekday" => fields = DateFields::YearMonthDayWeekday,
            "weekday" => fields = DateFields::Weekday,
            "month" => fields = DateFields::Month,
            "year-month" => fields = DateFields::YearMonth,
            "year" => fields = DateFields::Year,
            _ => context.report(bad_option(
                fields_option,
                "a field list such as year-month-day",
            )),
        }
    }
    formatting.date_fields = Some(fields);
}

/// The time part: a precision (`hour`, `minute`, `second`) or a style (`short` to the minute,
/// `medium`, `long` and `full` to the second). To the minute by default.
fn time_options(
    context: &mut FunctionContext<'_>,
    formatting: &mut DateTimeOptions,
    options: &Options,
    precision_option: &str,
    style_option: &str,
) {
    let mut precision = TimePrecision::Minute;
    if let Some(style) = options.text(style_option) {
        match style.as_str() {
            "short" => precision = TimePrecision::Minute,
            "medium" | "long" | "full" => precision = TimePrecision::Second,
            _ => context.report(bad_option(style_option, "full, long, medium or short")),
        }
        if formatting.date_fields.is_none() {
            formatting.length = if style == "short" {
                Length::Short
            } else {
                Length::Medium
            };
        }
    }
    if let Some(text) = options.text(precision_option) {
        match text.as_str() {
            "hour" => precision = TimePrecision::Hour,
            "minute" => precision = TimePrecision::Minute,
            "second" => precision = TimePrecision::Second,
            _ => context.report(bad_option(precision_option, "hour, minute or second")),
        }
    }
    formatting.time_precision = Some(precision);
}

/// Reads an ISO 8601 date (`2026-10-27`) or local date and time (`2026-10-27T15:45` with
/// optional seconds and fraction). Zone offsets are refused: the value is a wall time.
fn parse(text: &str) -> Result<DateTime, FunctionError> {
    let bad = || {
        FunctionError::BadOperand(format!(
            "`{text}` is not an ISO 8601 local date or date and time"
        ))
    };
    let (date, time) = match text.split_once(['T', 't']) {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let mut date_parts = date.splitn(3, '-');
    let year: i32 = date_parts
        .next()
        .and_then(|part| part.parse().ok())
        .ok_or_else(bad)?;
    let month: u8 = date_parts
        .next()
        .filter(|part| part.len() == 2)
        .and_then(|part| part.parse().ok())
        .ok_or_else(bad)?;
    let day: u8 = date_parts
        .next()
        .filter(|part| part.len() == 2)
        .and_then(|part| part.parse().ok())
        .ok_or_else(bad)?;
    let Some(time) = time else {
        return DateTime::date(year, month, day).map_err(|_| bad());
    };
    let (clock, fraction) = time
        .split_once('.')
        .map_or((time, None), |(clock, fraction)| (clock, Some(fraction)));
    let mut clock_parts = clock.split(':');
    let mut field = |required: bool| -> Result<u8, FunctionError> {
        match clock_parts.next() {
            Some(part) if part.len() == 2 => part.parse().map_err(|_| bad()),
            None if !required => Ok(0),
            _ => Err(bad()),
        }
    };
    let (hour, minute, second) = (field(true)?, field(true)?, field(false)?);
    if clock_parts.next().is_some() {
        return Err(bad());
    }
    let mut value = DateTime::new(year, month, day, hour, minute, second).map_err(|_| bad())?;
    if let Some(fraction) = fraction {
        if fraction.is_empty()
            || fraction.len() > 9
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(bad());
        }
        value.nanosecond = format!("{fraction:0<9}").parse().map_err(|_| bad())?;
    }
    Ok(value)
}

fn bad_option(option: &str, wanted: &str) -> FunctionError {
    FunctionError::bad_option(option, wanted)
}
