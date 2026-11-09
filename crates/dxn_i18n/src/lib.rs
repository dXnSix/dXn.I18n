//! Translations for Rust apps with [Unicode MessageFormat 2] messages, independent of any UI
//! framework and building for `wasm32-unknown-unknown`.
//!
//! - [`Message`]: one message, parsed and checked against MessageFormat 2's syntax and data
//!   model, formatted with [`Args`] for a locale.
//! - [`Bundle`]: one locale's messages, from JSON resources.
//! - [`Translator`]: an app's bundles, the user's locale and its fallbacks.
//! - [`FunctionRegistry`]: the functions messages call; the default ones format and select
//!   numbers and dates with [`dxn_intl`] (ICU4X), and apps add their own.
//! - [`args!`] and [`t!`]: arguments and translations at the call site, through the
//!   [`Translate`] trait a framework's service can implement too.
//!
//! [Unicode MessageFormat 2]: https://www.unicode.org/reports/tr35/tr35-messageFormat.html

mod bundle;
mod function;
mod macros;
mod message;
pub mod syntax;
mod translator;
mod value;

pub use bundle::{Bundle, LoadError, MessageError};
pub use function::{
    DateTimeValue, Function, FunctionContext, FunctionError, FunctionRegistry, NumberSelect,
    NumberValue, Options, ResolvedValue, TextValue, downcast,
};
#[doc(hidden)]
pub use macros::__private;
pub use macros::Translate;
pub use message::{BidiIsolation, FormatError, Formatted, Message, MessageFormatter, Part};
pub use syntax::{MarkupKind, ParseError};
pub use translator::{MissingMessage, TranslateError, Translator};
pub use value::{Args, Value};

/// Runs the README's examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct ReadmeExamples;
