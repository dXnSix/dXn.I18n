//! A recursive descent parser for MessageFormat 2's syntax (`spec/message.abnf`).

use unicode_normalization::UnicodeNormalization;

use super::{
    Attribute, Body, Declaration, Expression, FunctionOption, FunctionRef, Key, Markup, MarkupKind,
    Matcher, MessageSyntax, Operand, ParseError, Pattern, PatternPart, Variant, validate,
};

/// Parses a message's text and checks it against the data model.
///
/// A message that starts (after optional whitespace) with `.` or `{{` is a complex message;
/// anything else is a simple message, its leading whitespace included in its text.
pub fn parse(source: &str) -> Result<MessageSyntax> {
    let mut parser = Parser { source, pos: 0 };
    let message = parser.message()?;
    validate::validate(&message).map_err(ParseError::DataModel)?;
    Ok(message)
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
}

type Result<T> = std::result::Result<T, ParseError>;

impl Parser<'_> {
    fn message(&mut self) -> Result<MessageSyntax> {
        let start = self.pos;
        self.skip_o();
        let rest = self.rest();
        if rest.starts_with('.') || rest.starts_with("{{") {
            return self.complex_message();
        }
        self.pos = start;
        let pattern = self.pattern(false)?;
        if self.pos < self.source.len() {
            return self.error("text or a placeholder");
        }
        Ok(MessageSyntax {
            declarations: Vec::new(),
            body: Body::Pattern(pattern),
        })
    }

    fn complex_message(&mut self) -> Result<MessageSyntax> {
        let mut declarations = Vec::new();
        loop {
            self.skip_o();
            let rest = self.rest();
            if rest.starts_with(".input") && !self.keyword_continues(".input") {
                self.pos += ".input".len();
                self.skip_o();
                let expression = self.expression_placeholder()?;
                let Some(Operand::Variable(name)) = expression.operand.clone() else {
                    return self.error("a variable expression after .input");
                };
                declarations.push(Declaration::Input { name, expression });
            } else if rest.starts_with(".local") && !self.keyword_continues(".local") {
                self.pos += ".local".len();
                self.require_s()?;
                let name = self.variable()?;
                self.skip_o();
                self.expect('=')?;
                self.skip_o();
                let expression = self.expression_placeholder()?;
                declarations.push(Declaration::Local { name, expression });
            } else if rest.starts_with(".match") && !self.keyword_continues(".match") {
                self.pos += ".match".len();
                let matcher = self.matcher()?;
                self.skip_o();
                return self.end(MessageSyntax {
                    declarations,
                    body: Body::Matcher(matcher),
                });
            } else if rest.starts_with("{{") {
                let pattern = self.quoted_pattern()?;
                self.skip_o();
                return self.end(MessageSyntax {
                    declarations,
                    body: Body::Pattern(pattern),
                });
            } else {
                return self.error(".input, .local, .match or a quoted pattern");
            }
        }
    }

    fn end(&self, message: MessageSyntax) -> Result<MessageSyntax> {
        if self.pos < self.source.len() {
            self.error("the end of the message")
        } else {
            Ok(message)
        }
    }

    /// Whether a keyword at the cursor runs on into a longer name (`.inputs`).
    fn keyword_continues(&self, keyword: &str) -> bool {
        self.rest()[keyword.len()..]
            .chars()
            .next()
            .is_some_and(is_name_char)
    }

    fn matcher(&mut self) -> Result<Matcher> {
        let mut selectors = Vec::new();
        loop {
            let save = self.pos;
            if !self.skip_s() {
                break;
            }
            if self.peek() == Some('$') {
                selectors.push(self.variable()?);
            } else {
                self.pos = save;
                break;
            }
        }
        if selectors.is_empty() {
            return self.error("a selector variable after .match");
        }
        let mut variants = Vec::new();
        loop {
            // The first variant needs whitespace after the selectors; later ones may follow
            // a `}}` directly.
            let save = self.pos;
            let had_space = self.skip_s_or_o();
            if self.pos >= self.source.len()
                || !(self
                    .peek()
                    .is_some_and(|c| c == '*' || c == '|' || is_name_char(c)))
            {
                self.pos = save;
                break;
            }
            if variants.is_empty() && !had_space {
                return self.error("whitespace before the first variant");
            }
            variants.push(self.variant()?);
        }
        if variants.is_empty() {
            return self.error("a variant");
        }
        Ok(Matcher {
            selectors,
            variants,
        })
    }

    fn variant(&mut self) -> Result<Variant> {
        let mut keys = vec![self.key()?];
        loop {
            let save = self.pos;
            if !self.skip_s() {
                break;
            }
            match self.peek() {
                Some('{') => {
                    self.pos = save;
                    break;
                }
                Some(c) if c == '*' || c == '|' || is_name_char(c) => keys.push(self.key()?),
                _ => {
                    self.pos = save;
                    break;
                }
            }
        }
        self.skip_o();
        let pattern = self.quoted_pattern()?;
        Ok(Variant { keys, pattern })
    }

    fn key(&mut self) -> Result<Key> {
        if self.peek() == Some('*') {
            self.pos += 1;
            return Ok(Key::CatchAll);
        }
        Ok(Key::Literal(self.literal()?))
    }

    fn quoted_pattern(&mut self) -> Result<Pattern> {
        if !self.rest().starts_with("{{") {
            return self.error("`{{`");
        }
        self.pos += 2;
        let pattern = self.pattern(true)?;
        if !self.rest().starts_with("}}") {
            return self.error("`}}`");
        }
        self.pos += 2;
        Ok(pattern)
    }

    /// Text and placeholders, up to the end or (when `quoted`) a `}`.
    fn pattern(&mut self, quoted: bool) -> Result<Pattern> {
        let mut parts = Vec::new();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            match c {
                '\\' => text.push(self.escape()?),
                '{' => {
                    if !text.is_empty() {
                        parts.push(PatternPart::Text(std::mem::take(&mut text)));
                    }
                    parts.push(self.placeholder()?);
                }
                '}' => {
                    if quoted {
                        break;
                    }
                    return self.error("text; `}` must be escaped as `\\}`");
                }
                '\0' => return self.error("text; NUL is not allowed"),
                c => {
                    text.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
        if !text.is_empty() {
            parts.push(PatternPart::Text(text));
        }
        Ok(parts)
    }

    fn escape(&mut self) -> Result<char> {
        self.pos += 1;
        match self.peek() {
            Some(c @ ('\\' | '{' | '|' | '}')) => {
                self.pos += 1;
                Ok(c)
            }
            _ => self.error("`\\\\`, `\\{`, `\\|` or `\\}` after a backslash"),
        }
    }

    fn placeholder(&mut self) -> Result<PatternPart> {
        let save = self.pos;
        self.expect('{')?;
        self.skip_o();
        if matches!(self.peek(), Some('#' | '/')) {
            self.pos = save;
            return self.markup().map(PatternPart::Markup);
        }
        self.pos = save;
        self.expression_placeholder().map(PatternPart::Expression)
    }

    /// `{ operand [s function] *(s attribute) }` or `{ function *(s attribute) }`.
    fn expression_placeholder(&mut self) -> Result<Expression> {
        self.expect('{')?;
        self.skip_o();
        let operand = match self.peek() {
            Some('$') => Some(Operand::Variable(self.variable()?)),
            Some(':') => None,
            Some(c) if c == '|' || is_name_char(c) => Some(Operand::Literal(self.literal()?)),
            _ => return self.error("a literal, a variable or a function"),
        };
        let mut function = None;
        if operand.is_none() {
            function = Some(self.function()?);
        } else {
            let save = self.pos;
            if self.skip_s() && self.peek() == Some(':') {
                function = Some(self.function()?);
            } else {
                self.pos = save;
            }
        }
        let attributes = self.attributes()?;
        self.skip_o();
        self.expect('}')?;
        Ok(Expression {
            operand,
            function,
            attributes,
        })
    }

    fn function(&mut self) -> Result<FunctionRef> {
        self.expect(':')?;
        let name = self.identifier()?;
        let options = self.options()?;
        Ok(FunctionRef { name, options })
    }

    fn options(&mut self) -> Result<Vec<FunctionOption>> {
        let mut options = Vec::new();
        loop {
            let save = self.pos;
            if !self.skip_s() || !self.peek().is_some_and(is_name_start) {
                self.pos = save;
                return Ok(options);
            }
            let name = self.identifier()?;
            self.skip_o();
            self.expect('=')?;
            self.skip_o();
            let value = if self.peek() == Some('$') {
                Operand::Variable(self.variable()?)
            } else {
                Operand::Literal(self.literal()?)
            };
            options.push(FunctionOption { name, value });
        }
    }

    fn attributes(&mut self) -> Result<Vec<Attribute>> {
        let mut attributes = Vec::new();
        loop {
            let save = self.pos;
            if !self.skip_s() || self.peek() != Some('@') {
                self.pos = save;
                return Ok(attributes);
            }
            self.pos += 1;
            let name = self.identifier()?;
            let save = self.pos;
            self.skip_o();
            let value = if self.peek() == Some('=') {
                self.pos += 1;
                self.skip_o();
                Some(self.literal()?)
            } else {
                self.pos = save;
                None
            };
            attributes.push(Attribute { name, value });
        }
    }

    fn markup(&mut self) -> Result<Markup> {
        self.expect('{')?;
        self.skip_o();
        let close = self.peek() == Some('/');
        self.pos += 1;
        let name = self.identifier()?;
        let options = self.options()?;
        let attributes = self.attributes()?;
        self.skip_o();
        let kind = if close {
            MarkupKind::Close
        } else if self.peek() == Some('/') {
            self.pos += 1;
            MarkupKind::Standalone
        } else {
            MarkupKind::Open
        };
        self.expect('}')?;
        Ok(Markup {
            kind,
            name,
            options,
            attributes,
        })
    }

    fn variable(&mut self) -> Result<String> {
        self.expect('$')?;
        self.name()
    }

    /// `[namespace ":"] name`.
    fn identifier(&mut self) -> Result<String> {
        let mut identifier = self.name()?;
        if self.peek() == Some(':') && self.rest()[1..].chars().next().is_some_and(is_name_start) {
            self.pos += 1;
            identifier.push(':');
            identifier.push_str(&self.name()?);
        }
        Ok(identifier)
    }

    /// A name, with optional bidi marks around it that are not part of it.
    fn name(&mut self) -> Result<String> {
        self.skip_bidi();
        if !self.peek().is_some_and(is_name_start) {
            return self.error("a name");
        }
        let start = self.pos;
        while self.peek().is_some_and(is_name_char) {
            self.pos += self.peek().map_or(0, char::len_utf8);
        }
        // Names compare in Unicode NFC.
        let name = self.source[start..self.pos].nfc().collect();
        self.skip_bidi();
        Ok(name)
    }

    /// A quoted literal `|…|` or an unquoted one (a name or a number).
    fn literal(&mut self) -> Result<String> {
        if self.peek() == Some('|') {
            self.pos += 1;
            let mut value = String::new();
            loop {
                match self.peek() {
                    None => return self.error("`|` closing the literal"),
                    Some('|') => {
                        self.pos += 1;
                        return Ok(value);
                    }
                    Some('\\') => value.push(self.escape()?),
                    Some('\0') => return self.error("a literal character; NUL is not allowed"),
                    Some(c) => {
                        value.push(c);
                        self.pos += c.len_utf8();
                    }
                }
            }
        }
        // An unquoted literal is a run of name characters (names and numbers alike).
        let start = self.pos;
        while let Some(c) = self.peek().filter(|c| is_name_char(*c)) {
            self.pos += c.len_utf8();
        }
        if self.pos == start {
            return self.error("a literal");
        }
        Ok(self.source[start..self.pos].to_string())
    }

    fn expect(&mut self, wanted: char) -> Result<()> {
        if self.peek() == Some(wanted) {
            self.pos += wanted.len_utf8();
            Ok(())
        } else {
            self.error(match wanted {
                '{' => "`{`",
                '}' => "`}`",
                '=' => "`=`",
                ':' => "`:`",
                '$' => "`$`",
                _ => "another character",
            })
        }
    }

    fn require_s(&mut self) -> Result<()> {
        if self.skip_s() {
            Ok(())
        } else {
            self.error("whitespace")
        }
    }

    /// `s = *bidi ws o`: true when at least one whitespace character was skipped.
    fn skip_s(&mut self) -> bool {
        let save = self.pos;
        self.skip_bidi();
        if !self.peek().is_some_and(is_ws) {
            self.pos = save;
            return false;
        }
        self.skip_o();
        true
    }

    /// Skips `o`; true when it held whitespace.
    fn skip_s_or_o(&mut self) -> bool {
        let start = self.pos;
        self.skip_o();
        self.source[start..self.pos].chars().any(is_ws)
    }

    fn skip_o(&mut self) {
        while self.peek().is_some_and(|c| is_ws(c) || is_bidi(c)) {
            self.pos += self.peek().map_or(0, char::len_utf8);
        }
    }

    fn skip_bidi(&mut self) {
        while self.peek().is_some_and(is_bidi) {
            self.pos += self.peek().map_or(0, char::len_utf8);
        }
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn rest(&self) -> &str {
        &self.source[self.pos..]
    }

    fn error<T>(&self, expected: &'static str) -> Result<T> {
        Err(ParseError::Syntax {
            offset: self.pos,
            expected,
        })
    }
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{3000}')
}

fn is_bidi(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{2066}'..='\u{2069}'
    )
}

/// `name-start`: letters, `+`, `_`, and from U+00A1 on everything but whitespace, controls,
/// bidi marks, surrogates and noncharacters.
fn is_name_start(c: char) -> bool {
    let code = u32::from(c);
    c.is_ascii_alphabetic()
        || c == '+'
        || c == '_'
        || (code >= 0xA1
            && !matches!(code, 0x61C | 0x1680 | 0x2000..=0x200A | 0x200E..=0x200F | 0x2028..=0x202F | 0x205F | 0x2066..=0x2069 | 0x3000 | 0xFDD0..=0xFDEF)
            && code & 0xFFFE != 0xFFFE)
}

fn is_name_char(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || matches!(c, '-' | '.')
}
