use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use dxn_intl::{Direction, Locale};

use crate::function::{FunctionContext, FunctionRegistry, Options, ResolvedValue, TextValue};
use crate::syntax::{
    self, Body, Expression, Key, MarkupKind, MessageSyntax, Operand, ParseError, PatternPart,
};
use crate::{Args, Value};

/// A MessageFormat 2 message, parsed and checked, ready to format.
///
/// ```
/// use dxn_i18n::{Args, Message};
/// use dxn_intl::parse_locale;
///
/// let message = Message::parse(
///     ".input {$count :number}
///      .match $count
///      0   {{Your cart is empty.}}
///      one {{One item in your cart.}}
///      *   {{{$count} items in your cart.}}",
/// )?;
/// let en = parse_locale("en")?;
/// assert_eq!(message.format(&en, &Args::new().with("count", 1)), "One item in your cart.");
/// assert_eq!(message.format(&en, &Args::new().with("count", 1200)), "1,200 items in your cart.");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    source: String,
    syntax: MessageSyntax,
}

impl Message {
    /// Parses `source` and checks it against the data model.
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        Ok(Self {
            source: source.to_string(),
            syntax: syntax::parse(source)?,
        })
    }

    /// The message's text.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The parsed message.
    pub fn syntax(&self) -> &MessageSyntax {
        &self.syntax
    }

    /// Formats the message for `locale` with the default functions and no bidi isolation;
    /// what fails shows as a fallback such as `{$name}`.
    pub fn format(&self, locale: &Locale, args: &Args) -> String {
        MessageFormatter::new(locale.clone())
            .format(self, args)
            .text
    }
}

impl std::str::FromStr for Message {
    type Err = ParseError;

    fn from_str(source: &str) -> Result<Self, ParseError> {
        Self::parse(source)
    }
}

/// How placeholders are kept from reordering the text around them in mixed-direction text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BidiIsolation {
    /// Placeholders are inserted as they are.
    #[default]
    None,
    /// MessageFormat 2's default strategy: a placeholder whose direction is not the
    /// message's, or is unknown (text arguments), is wrapped in Unicode isolates (FSI, LRI or
    /// RLI … PDI).
    Default,
}

/// A piece of a formatted message, for UIs that render markup or style values.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    /// Literal text of the pattern.
    Text(String),
    /// A placeholder's formatted value.
    Value {
        /// The function that formatted it, if any.
        function: Option<String>,
        /// Its text, isolated when the formatter asks for it.
        text: String,
    },
    /// A placeholder that could not be resolved, as `{$name}`.
    Fallback(String),
    /// A markup placeholder.
    Markup {
        /// Open, standalone or close.
        kind: MarkupKind,
        /// Its name, as `b`.
        name: String,
        /// Its resolved options.
        options: Vec<(String, Value)>,
    },
}

/// Something that went wrong formatting a message; the message still formats, with a
/// fallback where it went wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// No argument or declaration has this name.
    UnresolvedVariable(String),
    /// No function has this name.
    UnknownFunction(String),
    /// A function could not take its operand (missing, of the wrong kind, or a fallback).
    BadOperand {
        /// The function's name, or the markup's as `#name`.
        function: String,
        /// What is wrong.
        reason: String,
    },
    /// An option's value is not one the function takes; the function may have ignored it.
    BadOption {
        /// The function's name, or the markup's as `#name`.
        function: String,
        /// The option's name.
        option: String,
        /// What is wrong.
        reason: String,
    },
    /// A selector's value cannot select; only `*` keys matched it.
    BadSelector(String),
    /// A function failed for another reason.
    Function {
        /// The function's name.
        function: String,
        /// What went wrong.
        reason: String,
    },
}

impl FormatError {
    fn from_function(function: &str, error: crate::FunctionError) -> Self {
        let function = function.to_string();
        match error {
            crate::FunctionError::BadOperand(reason) => Self::BadOperand { function, reason },
            crate::FunctionError::BadOption { option, reason } => Self::BadOption {
                function,
                option,
                reason,
            },
            crate::FunctionError::Failed(reason) => Self::Function { function, reason },
        }
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnresolvedVariable(name) => write!(f, "unresolved variable `${name}`"),
            Self::UnknownFunction(name) => write!(f, "unknown function `:{name}`"),
            Self::BadOperand { function, reason } => {
                write!(f, "`{function}`: bad operand: {reason}")
            }
            Self::BadOption {
                function,
                option,
                reason,
            } => {
                write!(f, "`{function}`: bad option `{option}`: {reason}")
            }
            Self::BadSelector(name) => write!(f, "`${name}` cannot select a variant"),
            Self::Function { function, reason } => write!(f, "`{function}`: {reason}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// A formatted message and what went wrong formatting it.
#[derive(Clone, Debug, PartialEq)]
pub struct Formatted<T> {
    /// The formatted message.
    pub text: T,
    /// The errors, in the order they happened; empty when all went well.
    pub errors: Vec<FormatError>,
}

/// Formats messages for one locale with a set of functions.
#[derive(Clone, Debug)]
pub struct MessageFormatter {
    locale: Locale,
    functions: Arc<FunctionRegistry>,
    bidi: BidiIsolation,
}

impl MessageFormatter {
    /// A formatter for `locale` with the default functions and no bidi isolation.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            functions: FunctionRegistry::shared_default(),
            bidi: BidiIsolation::None,
        }
    }

    /// The same formatter with `functions`.
    pub fn with_functions(mut self, functions: Arc<FunctionRegistry>) -> Self {
        self.functions = functions;
        self
    }

    /// The same formatter with `bidi` isolation.
    pub fn with_bidi_isolation(mut self, bidi: BidiIsolation) -> Self {
        self.bidi = bidi;
        self
    }

    /// The locale values are formatted for.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// `message` as text; markup leaves no text.
    pub fn format(&self, message: &Message, args: &Args) -> Formatted<String> {
        let Formatted {
            text: parts,
            errors,
        } = self.format_to_parts(message, args);
        let mut text = String::new();
        for part in parts {
            match part {
                Part::Text(piece) | Part::Value { text: piece, .. } => text.push_str(&piece),
                Part::Fallback(fallback) => {
                    text.push('{');
                    text.push_str(&fallback);
                    text.push('}');
                }
                Part::Markup { .. } => {}
            }
        }
        Formatted { text, errors }
    }

    /// `message` as parts: text, values, fallbacks and markup.
    pub fn format_to_parts(&self, message: &Message, args: &Args) -> Formatted<Vec<Part>> {
        let mut run = Run {
            formatter: self,
            args,
            locals: HashMap::new(),
            errors: Vec::new(),
        };
        for declaration in &message.syntax.declarations {
            let resolution = run.expression(declaration.expression());
            run.locals
                .insert(declaration.name().to_string(), resolution);
        }
        let pattern = match &message.syntax.body {
            Body::Pattern(pattern) => pattern,
            Body::Matcher(matcher) => run.select(matcher),
        };
        let mut parts = Vec::new();
        for part in pattern {
            match part {
                PatternPart::Text(text) => parts.push(Part::Text(text.clone())),
                PatternPart::Expression(expression) => parts.push(run.placeholder(expression)),
                PatternPart::Markup(markup) => {
                    run.u_options(&format!("#{}", markup.name), &markup.options, true);
                    let options = markup
                        .options
                        .iter()
                        .filter(|option| !option.name.starts_with("u:"))
                        .filter_map(|option| {
                            run.option(&option.value)
                                .map(|(value, _)| (option.name.clone(), value))
                        })
                        .collect();
                    parts.push(Part::Markup {
                        kind: markup.kind,
                        name: markup.name.clone(),
                        options,
                    });
                }
            }
        }
        Formatted {
            text: parts,
            errors: run.errors,
        }
    }
}

/// The `u:dir` option: the direction of an expression's value, for bidi isolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ValueDirection {
    /// No `u:dir`: the value's own direction.
    #[default]
    Inherit,
    Ltr,
    Rtl,
    Auto,
}

/// An expression's outcome: a value, or the fallback text that stands for it.
#[derive(Clone, Debug)]
enum Resolution {
    /// `shown_as` is the fallback the value shows as when its operand was a fallback.
    Value {
        value: Value,
        function: Option<String>,
        direction: ValueDirection,
        shown_as: Option<String>,
    },
    Fallback(String),
}

struct Run<'a> {
    formatter: &'a MessageFormatter,
    args: &'a Args,
    locals: HashMap<String, Resolution>,
    errors: Vec<FormatError>,
}

impl Run<'_> {
    fn variable(&mut self, name: &str) -> Resolution {
        if let Some(resolution) = self.locals.get(name) {
            return match resolution.clone() {
                Resolution::Fallback(_) => Resolution::Fallback(format!("${name}")),
                Resolution::Value {
                    value,
                    function,
                    direction,
                    shown_as,
                } => Resolution::Value {
                    value,
                    function,
                    direction,
                    shown_as: shown_as.map(|_| format!("${name}")),
                },
            };
        }
        match self.args.get(name) {
            Some(value) => Resolution::Value {
                value: value.clone(),
                function: None,
                direction: ValueDirection::Inherit,
                shown_as: None,
            },
            None => {
                self.errors
                    .push(FormatError::UnresolvedVariable(name.to_string()));
                Resolution::Fallback(format!("${name}"))
            }
        }
    }

    /// An option's value, and whether it is a literal; an unresolved variable is reported and
    /// left out.
    fn option(&mut self, operand: &Operand) -> Option<(Value, bool)> {
        match operand {
            Operand::Literal(text) => Some((Value::String(text.clone()), true)),
            Operand::Variable(name) => match self.variable(name) {
                Resolution::Value { value, .. } => Some((value, false)),
                Resolution::Fallback(_) => None,
            },
        }
    }

    /// The `u:` options of a function or markup: `u:dir` and `u:locale` (`u:id` only names the
    /// placeholder for parts).
    fn u_options(
        &mut self,
        owner: &str,
        options: &[syntax::FunctionOption],
        markup: bool,
    ) -> (ValueDirection, Option<Locale>) {
        let mut direction = ValueDirection::Inherit;
        let mut locale = None;
        for option in options
            .iter()
            .filter(|option| option.name.starts_with("u:"))
        {
            let Some((value, _)) = self.option(&option.value) else {
                continue;
            };
            let text = value.as_str().unwrap_or_default().to_string();
            let bad = |wanted: &str| FormatError::BadOption {
                function: owner.to_string(),
                option: option.name.clone(),
                reason: format!("expected {wanted}"),
            };
            match option.name.as_str() {
                "u:dir" if markup => self.errors.push(bad("no `u:dir` on markup")),
                "u:dir" => match text.as_str() {
                    "ltr" => direction = ValueDirection::Ltr,
                    "rtl" => direction = ValueDirection::Rtl,
                    "auto" => direction = ValueDirection::Auto,
                    "inherit" => direction = ValueDirection::Inherit,
                    _ => self.errors.push(bad("ltr, rtl, auto or inherit")),
                },
                "u:locale" if markup => self.errors.push(bad("no `u:locale` on markup")),
                "u:locale" => match dxn_intl::parse_locale(&text) {
                    Ok(parsed) => locale = Some(parsed),
                    Err(_) => self.errors.push(bad("a language tag")),
                },
                _ => {}
            }
        }
        (direction, locale)
    }

    fn expression(&mut self, expression: &Expression) -> Resolution {
        let operand = match &expression.operand {
            None => None,
            Some(Operand::Literal(text)) => Some(Ok(Value::String(text.clone()))),
            Some(Operand::Variable(name)) => match self.variable(name) {
                resolved @ Resolution::Value { .. } if expression.function.is_none() => {
                    return resolved;
                }
                Resolution::Value { value, .. } => Some(Ok(value)),
                Resolution::Fallback(fallback) => Some(Err(fallback)),
            },
        };
        let Some(function) = &expression.function else {
            let value = match operand {
                Some(Ok(value)) => value,
                Some(Err(fallback)) => return Resolution::Fallback(fallback),
                None => Value::String(String::new()),
            };
            return Resolution::Value {
                value,
                function: None,
                direction: ValueDirection::Inherit,
                shown_as: None,
            };
        };
        let name = function.name.clone();
        let fallback = fallback_of(expression);
        let (direction, locale) = self.u_options(&format!(":{name}"), &function.options, false);
        let mut options = Options::default();
        for option in function
            .options
            .iter()
            .filter(|option| !option.name.starts_with("u:"))
        {
            if let Some((value, literal)) = self.option(&option.value) {
                options.push(option.name.clone(), value, literal);
            }
        }
        let Some(handler) = self.formatter.functions.get(&name).cloned() else {
            self.errors.push(FormatError::UnknownFunction(name));
            return Resolution::Fallback(fallback);
        };
        // A failed operand reaches the function as its fallback's text, flagged as such.
        let (operand, operand_is_fallback) = match operand {
            Some(Err(fallback)) => (Some(Value::String(fallback)), true),
            Some(Ok(value)) => (Some(value), false),
            None => (None, false),
        };
        let locale = locale.unwrap_or_else(|| self.formatter.locale.clone());
        let mut context = FunctionContext::new(&locale, operand_is_fallback);
        let result = handler.call(&mut context, operand.as_ref(), &options);
        self.errors.extend(
            context
                .into_reported()
                .into_iter()
                .map(|error| FormatError::from_function(&name, error)),
        );
        match result {
            Ok(resolved) => Resolution::Value {
                value: Value::Resolved(resolved),
                function: Some(name),
                direction,
                shown_as: operand_is_fallback.then(|| fallback.clone()),
            },
            Err(error) => {
                self.errors.push(FormatError::from_function(&name, error));
                Resolution::Fallback(fallback)
            }
        }
    }

    fn placeholder(&mut self, expression: &Expression) -> Part {
        let (value, function, direction) = match self.expression(expression) {
            Resolution::Fallback(fallback)
            | Resolution::Value {
                shown_as: Some(fallback),
                ..
            } => {
                return Part::Fallback(fallback);
            }
            Resolution::Value {
                value,
                function,
                direction,
                shown_as: None,
            } => (value, function, direction),
        };
        let resolved: Arc<dyn ResolvedValue> = match value {
            Value::Resolved(resolved) => resolved,
            Value::String(text) => Arc::new(TextValue(text)),
            implicit @ (Value::Number(_) | Value::DateTime(_)) => {
                let name = if matches!(implicit, Value::Number(_)) {
                    "number"
                } else {
                    "datetime"
                };
                let Some(handler) = self.formatter.functions.get(name).cloned() else {
                    self.errors
                        .push(FormatError::UnknownFunction(name.to_string()));
                    return Part::Fallback(fallback_of(expression));
                };
                let mut context = FunctionContext::new(&self.formatter.locale, false);
                match handler.call(&mut context, Some(&implicit), &Options::default()) {
                    Ok(resolved) => resolved,
                    Err(error) => {
                        self.errors.push(FormatError::from_function(name, error));
                        return Part::Fallback(fallback_of(expression));
                    }
                }
            }
        };
        match resolved.format() {
            Ok(text) => {
                let text = self.isolate(text, direction, &*resolved);
                Part::Value { function, text }
            }
            Err(error) => {
                let name = function.as_deref().unwrap_or("string");
                self.errors.push(FormatError::from_function(name, error));
                Part::Fallback(fallback_of(expression))
            }
        }
    }

    /// MessageFormat 2's default bidi strategy.
    fn isolate(
        &self,
        text: String,
        direction: ValueDirection,
        value: &dyn ResolvedValue,
    ) -> String {
        if self.formatter.bidi == BidiIsolation::None {
            return text;
        }
        let message_direction = dxn_intl::direction(&self.formatter.locale);
        // Plain text's direction is unknown, even when it is text the app typed.
        let own = if (value as &dyn std::any::Any).is::<TextValue>() {
            None
        } else {
            value.direction()
        };
        let (value_direction, isolate) = match direction {
            ValueDirection::Inherit => (own, false),
            ValueDirection::Ltr => (Some(Direction::Ltr), true),
            ValueDirection::Rtl => (Some(Direction::Rtl), true),
            ValueDirection::Auto => (None, true),
        };
        match value_direction {
            Some(Direction::Ltr) if message_direction == Direction::Ltr && !isolate => text,
            Some(Direction::Ltr) => format!("\u{2066}{text}\u{2069}"),
            Some(Direction::Rtl) => format!("\u{2067}{text}\u{2069}"),
            None => format!("\u{2068}{text}\u{2069}"),
        }
    }

    /// Picks the matcher's variant: MessageFormat 2's resolve preferences, filter and sort.
    fn select<'m>(&mut self, matcher: &'m syntax::Matcher) -> &'m syntax::Pattern {
        let mut preferences: Vec<Vec<String>> = Vec::with_capacity(matcher.selectors.len());
        for (index, selector) in matcher.selectors.iter().enumerate() {
            let mut keys: Vec<&str> = Vec::new();
            for variant in &matcher.variants {
                if let Key::Literal(key) = &variant.keys[index]
                    && !keys.contains(&key.as_str())
                {
                    keys.push(key);
                }
            }
            let order = match self.variable(selector) {
                Resolution::Value {
                    value: Value::Resolved(resolved),
                    ..
                } => resolved.select(&keys),
                Resolution::Value {
                    value: Value::String(text),
                    ..
                } => TextValue(text).select(&keys),
                _ => None,
            };
            let order = order.unwrap_or_else(|| {
                self.errors.push(FormatError::BadSelector(selector.clone()));
                Vec::new()
            });
            preferences.push(
                order
                    .into_iter()
                    .filter_map(|key| keys.get(key))
                    .map(|key| key.to_string())
                    .collect(),
            );
        }
        let rank = |variant: &syntax::Variant| -> Option<Vec<usize>> {
            variant
                .keys
                .iter()
                .zip(&preferences)
                .map(|(key, preferred)| match key {
                    Key::CatchAll => Some(preferred.len()),
                    Key::Literal(key) => preferred.iter().position(|wanted| wanted == key),
                })
                .collect()
        };
        let mut best: Option<(Vec<usize>, &syntax::Variant)> = None;
        for variant in &matcher.variants {
            if let Some(ranks) = rank(variant)
                && best
                    .as_ref()
                    .is_none_or(|(best_ranks, _)| ranks < *best_ranks)
            {
                best = Some((ranks, variant));
            }
        }
        // The data model guarantees an all-`*` variant, which always ranks.
        best.map_or(&EMPTY, |(_, variant)| &variant.pattern)
    }
}

static EMPTY: syntax::Pattern = Vec::new();

fn fallback_of(expression: &Expression) -> String {
    match (&expression.operand, &expression.function) {
        (Some(Operand::Literal(text)), _) => quote(text),
        (Some(Operand::Variable(name)), _) => format!("${name}"),
        (None, Some(function)) => format!(":{}", function.name),
        (None, None) => String::new(),
    }
}

/// A literal as a quoted literal, escaped.
fn quote(text: &str) -> String {
    let mut quoted = String::from("|");
    for c in text.chars() {
        if matches!(c, '\\' | '|') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('|');
    quoted
}
