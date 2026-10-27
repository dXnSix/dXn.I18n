use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use dxn_intl::{DateTime, Number};
use unicode_normalization::UnicodeNormalization;

use crate::function::ResolvedValue;

/// A value a message formats: an argument, a literal or an option.
#[derive(Clone)]
pub enum Value {
    /// Text.
    String(String),
    /// An exact decimal number.
    Number(Number),
    /// A wall time on the ISO calendar.
    DateTime(DateTime),
    /// A value a custom function made, or one the app resolved itself.
    Resolved(Arc<dyn ResolvedValue>),
}

impl Value {
    /// The value as text, when it is text.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(text) => f.debug_tuple("String").field(text).finish(),
            Self::Number(number) => f.debug_tuple("Number").field(&number.to_string()).finish(),
            Self::DateTime(value) => f.debug_tuple("DateTime").field(value).finish(),
            Self::Resolved(value) => f.debug_tuple("Resolved").field(&value.format()).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::DateTime(a), Self::DateTime(b)) => a == b,
            (Self::Resolved(a), Self::Resolved(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::String(text)
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::String(text.to_string())
    }
}

impl From<&String> for Value {
    fn from(text: &String) -> Self {
        Self::String(text.clone())
    }
}

impl From<Number> for Value {
    fn from(number: Number) -> Self {
        Self::Number(number)
    }
}

impl From<DateTime> for Value {
    fn from(value: DateTime) -> Self {
        Self::DateTime(value)
    }
}

macro_rules! from_integer {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                Self::Number(Number::from(value))
            }
        }
    )*};
}
from_integer!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

impl From<f64> for Value {
    /// A finite float is a number; NaN and the infinities are text (`"NaN"`, `"inf"`), which
    /// `:number` reports as a bad operand.
    fn from(value: f64) -> Self {
        Number::try_from(value).map_or_else(|_| Self::String(value.to_string()), Self::Number)
    }
}

impl From<f32> for Value {
    /// As for `f64`.
    fn from(value: f32) -> Self {
        Number::try_from(value).map_or_else(|_| Self::String(value.to_string()), Self::Number)
    }
}

/// The named arguments a message is formatted with.
///
/// ```
/// use dxn_i18n::Args;
///
/// let args = Args::new().with("name", "Ana").with("count", 3);
/// assert_eq!(args.get("name").and_then(|value| value.as_str()), Some("Ana"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Args {
    values: HashMap<String, Value>,
}

impl Args {
    /// No arguments.
    pub fn new() -> Self {
        Self::default()
    }

    /// These arguments and `name` set to `value`.
    pub fn with(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.set(name, value);
        self
    }

    /// Sets `name` to `value`, replacing what it was. Names compare in Unicode NFC, as
    /// messages' variable names do.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.values
            .insert(name.into().nfc().collect(), value.into());
    }

    /// The argument `name`.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values
            .get(name)
            .or_else(|| self.values.get(&name.nfc().collect::<String>()))
    }

    /// Whether there are no arguments.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The arguments' names and values, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }
}

impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for Args {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut args = Self::new();
        for (name, value) in iter {
            args.set(name, value);
        }
        args
    }
}
