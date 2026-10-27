//! MessageFormat 2's data model: what a message's text parses into.
//!
//! The names follow the [MessageFormat 2 specification]'s data model. [`parse`] reads a
//! message's text and checks it against the data model's rules; formatting happens in
//! [`crate::Message`].
//!
//! [MessageFormat 2 specification]: https://www.unicode.org/reports/tr35/tr35-messageFormat.html

mod parser;
mod validate;

use std::fmt;

pub use parser::parse;

/// A parsed message: its declarations, then a pattern or a matcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageSyntax {
    /// `.input` and `.local` declarations, in order.
    pub declarations: Vec<Declaration>,
    /// What the message formats to.
    pub body: Body,
}

/// A declaration of a complex message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Declaration {
    /// `.input {$name …}`: the argument `name`, annotated.
    Input {
        /// The argument's name, without `$`.
        name: String,
        /// The expression, whose operand is `$name`.
        expression: Expression,
    },
    /// `.local $name = {…}`: a new variable.
    Local {
        /// The variable's name, without `$`.
        name: String,
        /// The expression it is bound to.
        expression: Expression,
    },
}

impl Declaration {
    /// The declared variable's name.
    pub fn name(&self) -> &str {
        match self {
            Self::Input { name, .. } | Self::Local { name, .. } => name,
        }
    }

    /// The declared variable's expression.
    pub fn expression(&self) -> &Expression {
        match self {
            Self::Input { expression, .. } | Self::Local { expression, .. } => expression,
        }
    }
}

/// A message's body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// One pattern: a simple message, or a complex message's quoted pattern.
    Pattern(Pattern),
    /// `.match $a $b` and its variants.
    Matcher(Matcher),
}

/// `.match` with its selectors and variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matcher {
    /// The selected variables' names, without `$`.
    pub selectors: Vec<String>,
    /// The variants, in source order.
    pub variants: Vec<Variant>,
}

/// One variant of a matcher: a key per selector and a pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// The keys, one per selector.
    pub keys: Vec<Key>,
    /// The pattern formatted when this variant is selected.
    pub pattern: Pattern,
}

/// A variant key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// A literal key, such as `one` or `|1.5|`.
    Literal(String),
    /// `*`, which matches anything.
    CatchAll,
}

/// Text and placeholders.
pub type Pattern = Vec<PatternPart>;

/// A piece of a pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternPart {
    /// Text, its escapes resolved.
    Text(String),
    /// A placeholder `{…}` that formats a value.
    Expression(Expression),
    /// A markup placeholder: `{#b}`, `{/b}` or `{#img/}`.
    Markup(Markup),
}

/// An expression: an operand, a function, or both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expression {
    /// A literal or a variable.
    pub operand: Option<Operand>,
    /// `:name option=value …`.
    pub function: Option<FunctionRef>,
    /// `@name=value` attributes, which do not change formatting.
    pub attributes: Vec<Attribute>,
}

/// A literal or a variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    /// A literal's value, its quotes and escapes resolved.
    Literal(String),
    /// A variable's name, without `$`.
    Variable(String),
}

/// A function annotation `:name option=value …`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionRef {
    /// The function's name, with its namespace (`u:`, `my:`) if it has one.
    pub name: String,
    /// The options, in source order.
    pub options: Vec<FunctionOption>,
}

/// An option `name=value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionOption {
    /// The option's name.
    pub name: String,
    /// A literal or a variable.
    pub value: Operand,
}

/// What a markup placeholder does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkupKind {
    /// `{#b}`.
    Open,
    /// `{#img/}`.
    Standalone,
    /// `{/b}`.
    Close,
}

/// A markup placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Markup {
    /// Open, standalone or close.
    pub kind: MarkupKind,
    /// Its name, as `b` or `html:img`.
    pub name: String,
    /// Its options.
    pub options: Vec<FunctionOption>,
    /// Its attributes.
    pub attributes: Vec<Attribute>,
}

/// An attribute `@name` or `@name=value`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// The attribute's name.
    pub name: String,
    /// Its literal value, if it has one.
    pub value: Option<String>,
}

/// A message's text that is not MessageFormat 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The text does not follow the syntax: at `offset` (in bytes), `expected`.
    Syntax {
        /// The byte offset in the message's text.
        offset: usize,
        /// What the parser expected there.
        expected: &'static str,
    },
    /// The text follows the syntax but breaks a rule of the data model.
    DataModel(DataModelError),
}

/// A rule of MessageFormat 2's data model that a message breaks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataModelError {
    /// A variant has more or fewer keys than the matcher has selectors.
    VariantKeyMismatch,
    /// No variant has only `*` keys.
    MissingFallbackVariant,
    /// A selected variable has no function annotation, directly or through its declarations.
    MissingSelectorAnnotation(String),
    /// A variable is declared twice, or declared after a declaration before it used it.
    DuplicateDeclaration(String),
    /// A function or markup has an option twice.
    DuplicateOptionName(String),
    /// Two variants have the same keys.
    DuplicateVariant,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { offset, expected } => {
                write!(f, "syntax error at byte {offset}: expected {expected}")
            }
            Self::DataModel(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl fmt::Display for DataModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VariantKeyMismatch => f.write_str("a variant's keys do not match the selectors"),
            Self::MissingFallbackVariant => f.write_str("no variant has only `*` keys"),
            Self::MissingSelectorAnnotation(name) => {
                write!(f, "the selector `${name}` has no function annotation")
            }
            Self::DuplicateDeclaration(name) => write!(f, "`${name}` is declared twice"),
            Self::DuplicateOptionName(name) => write!(f, "the option `{name}` is given twice"),
            Self::DuplicateVariant => f.write_str("two variants have the same keys"),
        }
    }
}

impl std::error::Error for ParseError {}
