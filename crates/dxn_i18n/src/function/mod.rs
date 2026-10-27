//! The functions messages call (`{$count :number}`): the default ones and an app's own.

mod datetime;
mod number;

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, LazyLock};

use dxn_intl::Locale;
use unicode_normalization::UnicodeNormalization;

use crate::Value;

pub use datetime::DateTimeValue;
pub use number::{NumberSelect, NumberValue};

/// What a function makes of its operand: text to show and, for functions that select, the
/// variant keys it matches.
pub trait ResolvedValue: Any + Send + Sync {
    /// The value as text, or why it cannot be shown (the placeholder then shows its fallback).
    fn format(&self) -> Result<String, FunctionError>;

    /// The writing direction of [`ResolvedValue::format`]'s text, when it is known; text of
    /// unknown direction is isolated by [`crate::BidiIsolation::Default`].
    fn direction(&self) -> Option<dxn_intl::Direction> {
        None
    }

    /// The `keys` this value matches, best first, as indexes into `keys`; `None` when the
    /// value cannot select (a selector error: only `*` variants match).
    fn select(&self, keys: &[&str]) -> Option<Vec<usize>> {
        let _ = keys;
        None
    }
}

/// A function's resolved options: literals as text, variables as their values.
#[derive(Clone, Debug, Default)]
pub struct Options {
    entries: Vec<OptionEntry>,
}

#[derive(Clone, Debug)]
struct OptionEntry {
    name: String,
    value: Value,
    literal: bool,
}

impl Options {
    pub(crate) fn push(&mut self, name: String, value: Value, literal: bool) {
        self.entries.push(OptionEntry {
            name,
            value,
            literal,
        });
    }

    /// The option `name`'s value.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }

    /// Whether the option `name` is set by a literal in the message (`select=exact`) rather
    /// than by a variable (`select=$mode`).
    pub fn is_literal(&self, name: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.name == name && entry.literal)
    }

    /// The option `name` as text: text itself, a number's digits, a resolved value's
    /// formatted text.
    pub fn text(&self, name: &str) -> Option<String> {
        self.get(name).and_then(|value| match value {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::DateTime(_) => None,
            Value::Resolved(resolved) => match downcast::<NumberValue>(resolved) {
                Some(number) => Some(number.operand().to_string()),
                None => resolved.format().ok(),
            },
        })
    }

    /// The options' names and values, in source order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries
            .iter()
            .map(|entry| (entry.name.as_str(), &entry.value))
    }
}

/// Why a function could not resolve, format or select its operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionError {
    /// The operand is missing or of a kind the function does not take.
    BadOperand(String),
    /// An option's value is not one the function takes.
    BadOption {
        /// The option's name.
        option: String,
        /// What is wrong with it.
        reason: String,
    },
    /// Anything else.
    Failed(String),
}

impl FunctionError {
    /// A [`FunctionError::BadOption`] for `option`, which expected `wanted`.
    pub fn bad_option(option: &str, wanted: &str) -> Self {
        Self::BadOption {
            option: option.to_string(),
            reason: format!("expected {wanted}"),
        }
    }
}

impl fmt::Display for FunctionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadOperand(reason) => write!(f, "bad operand: {reason}"),
            Self::BadOption { option, reason } => write!(f, "bad option `{option}`: {reason}"),
            Self::Failed(reason) => f.write_str(reason),
        }
    }
}

impl std::error::Error for FunctionError {}

/// What a function is called with besides its operand and options: the locale, and a place
/// to report problems it worked around (an option it ignored) without failing.
#[derive(Debug)]
pub struct FunctionContext<'a> {
    locale: &'a Locale,
    operand_is_fallback: bool,
    reported: Vec<FunctionError>,
}

impl<'a> FunctionContext<'a> {
    pub(crate) fn new(locale: &'a Locale, operand_is_fallback: bool) -> Self {
        Self {
            locale,
            operand_is_fallback,
            reported: Vec::new(),
        }
    }

    /// Whether the operand is a fallback: its variable could not be resolved, and the
    /// operand is the fallback's text (`$name`). Most functions refuse it as a bad operand
    /// (a number function does, since `$name` is not a number); the placeholder shows the
    /// fallback either way.
    pub fn operand_is_fallback(&self) -> bool {
        self.operand_is_fallback
    }

    /// The locale to format for.
    pub fn locale(&self) -> &Locale {
        self.locale
    }

    /// Reports `error` while the function still resolves, such as an option it ignored.
    pub fn report(&mut self, error: FunctionError) {
        self.reported.push(error);
    }

    pub(crate) fn into_reported(self) -> Vec<FunctionError> {
        self.reported
    }
}

/// A function a message can call, such as `:number` or an app's `:my:money`.
///
/// Closures with the same signature are functions.
pub trait Function: Send + Sync {
    /// Resolves `operand` (when the expression has one) with `options`. An operand another
    /// function resolved arrives as [`Value::Resolved`], so functions compose:
    /// `.local $n = {$x :number minimumFractionDigits=2}` then `{$n :number}`. An `Err` makes
    /// the placeholder show its fallback; problems it can work around go to
    /// [`FunctionContext::report`].
    fn call(
        &self,
        context: &mut FunctionContext<'_>,
        operand: Option<&Value>,
        options: &Options,
    ) -> Result<Arc<dyn ResolvedValue>, FunctionError>;
}

impl<F> Function for F
where
    F: Fn(
            &mut FunctionContext<'_>,
            Option<&Value>,
            &Options,
        ) -> Result<Arc<dyn ResolvedValue>, FunctionError>
        + Send
        + Sync,
{
    fn call(
        &self,
        context: &mut FunctionContext<'_>,
        operand: Option<&Value>,
        options: &Options,
    ) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
        self(context, operand, options)
    }
}

/// The functions messages can call, by name.
///
/// [`FunctionRegistry::default`] has MessageFormat 2's default functions `:string`, `:number`
/// and `:integer`, and its draft `:offset`, `:percent`, `:currency`, `:date`, `:time` and
/// `:datetime`. Add an app's own with [`FunctionRegistry::register`], in a namespace such as
/// `my:` so they never clash with future default functions.
#[derive(Clone)]
pub struct FunctionRegistry {
    functions: HashMap<String, Arc<dyn Function>>,
}

impl FunctionRegistry {
    /// No functions, not even the default ones.
    pub fn empty() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Adds `function` as `name` (without the `:`), replacing a function of that name.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        function: impl Function + 'static,
    ) -> &mut Self {
        self.functions.insert(name.into(), Arc::new(function));
        self
    }

    /// The function `name`.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Function>> {
        self.functions.get(name)
    }

    /// The registry the crate formats with when none is given.
    pub(crate) fn shared_default() -> Arc<Self> {
        static DEFAULT: LazyLock<Arc<FunctionRegistry>> =
            LazyLock::new(|| Arc::new(FunctionRegistry::default()));
        DEFAULT.clone()
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        let mut registry = Self::empty();
        registry
            .register("string", string)
            .register("number", number::NumberFunction::Number)
            .register("integer", number::NumberFunction::Integer)
            .register("offset", number::NumberFunction::Offset)
            .register("percent", number::NumberFunction::Percent)
            .register("currency", number::NumberFunction::Currency)
            .register("date", datetime::DateTimeFunction::Date)
            .register("time", datetime::DateTimeFunction::Time)
            .register("datetime", datetime::DateTimeFunction::DateTime);
        registry
    }
}

impl fmt::Debug for FunctionRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: Vec<_> = self.functions.keys().collect();
        names.sort();
        f.debug_struct("FunctionRegistry")
            .field("functions", &names)
            .finish()
    }
}

/// Text, as `:string` and literals resolve: it selects the key that is the same text (both
/// in Unicode NFC).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextValue(pub String);

impl ResolvedValue for TextValue {
    fn format(&self) -> Result<String, FunctionError> {
        Ok(self.0.clone())
    }

    fn select(&self, keys: &[&str]) -> Option<Vec<usize>> {
        let wanted: String = self.0.nfc().collect();
        Some(
            keys.iter()
                .position(|key| key.nfc().eq(wanted.chars()))
                .into_iter()
                .collect(),
        )
    }
}

/// A fallback operand as `:string` takes it: it shows as its fallback and matches no key.
#[derive(Debug)]
struct FallbackText;

impl ResolvedValue for FallbackText {
    fn format(&self) -> Result<String, FunctionError> {
        Err(FunctionError::BadOperand(
            "the operand is a fallback".to_string(),
        ))
    }

    fn select(&self, _: &[&str]) -> Option<Vec<usize>> {
        Some(Vec::new())
    }
}

/// `:string`: the operand as text.
fn string(
    context: &mut FunctionContext<'_>,
    operand: Option<&Value>,
    _: &Options,
) -> Result<Arc<dyn ResolvedValue>, FunctionError> {
    if context.operand_is_fallback() {
        return Ok(Arc::new(FallbackText));
    }
    let text = match operand {
        None => {
            return Err(FunctionError::BadOperand(
                "`:string` needs an operand".to_string(),
            ));
        }
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::DateTime(_)) => {
            return Err(FunctionError::BadOperand(
                "`:string` does not take a date".to_string(),
            ));
        }
        Some(Value::Resolved(resolved)) => resolved.format()?,
    };
    Ok(Arc::new(TextValue(text)))
}

/// A resolved value as the value type `T`, when it is one.
pub fn downcast<T: ResolvedValue>(value: &Arc<dyn ResolvedValue>) -> Option<&T> {
    let any: &dyn Any = &**value;
    any.downcast_ref::<T>()
}
