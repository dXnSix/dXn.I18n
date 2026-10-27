//! The data model's rules a message must keep beyond its syntax.

use std::collections::HashSet;

use unicode_normalization::UnicodeNormalization;

use super::{
    Body, DataModelError, Declaration, Expression, FunctionOption, Key, MessageSyntax, Operand,
    PatternPart,
};

pub(super) fn validate(message: &MessageSyntax) -> Result<(), DataModelError> {
    declarations(&message.declarations)?;
    match &message.body {
        Body::Pattern(pattern) => pattern_options(pattern),
        Body::Matcher(matcher) => {
            for selector in &matcher.selectors {
                if !annotated(selector, &message.declarations) {
                    return Err(DataModelError::MissingSelectorAnnotation(selector.clone()));
                }
            }
            let mut seen = HashSet::new();
            let mut fallback = false;
            for variant in &matcher.variants {
                if variant.keys.len() != matcher.selectors.len() {
                    return Err(DataModelError::VariantKeyMismatch);
                }
                fallback |= variant.keys.iter().all(|key| *key == Key::CatchAll);
                let normalized: Vec<Option<String>> = variant
                    .keys
                    .iter()
                    .map(|key| match key {
                        Key::Literal(value) => Some(value.nfc().collect()),
                        Key::CatchAll => None,
                    })
                    .collect();
                if !seen.insert(normalized) {
                    return Err(DataModelError::DuplicateVariant);
                }
                pattern_options(&variant.pattern)?;
            }
            if !fallback {
                return Err(DataModelError::MissingFallbackVariant);
            }
            Ok(())
        }
    }
}

/// A variable is declared once, never after a declaration before it read it, and a `.local`
/// never reads itself.
fn declarations(declarations: &[Declaration]) -> Result<(), DataModelError> {
    let mut taken: HashSet<&str> = HashSet::new();
    for declaration in declarations {
        let name = declaration.name();
        let expression = declaration.expression();
        if taken.contains(name) {
            return Err(DataModelError::DuplicateDeclaration(name.to_string()));
        }
        if let Declaration::Local { .. } = declaration
            && variables(expression).any(|used| used == name)
        {
            return Err(DataModelError::DuplicateDeclaration(name.to_string()));
        }
        unique_options(
            expression
                .function
                .as_ref()
                .map(|function| &function.options[..])
                .unwrap_or(&[]),
        )?;
        taken.extend(variables(expression));
        taken.insert(name);
    }
    Ok(())
}

fn variables(expression: &Expression) -> impl Iterator<Item = &str> {
    let operand = match &expression.operand {
        Some(Operand::Variable(name)) => Some(name.as_str()),
        _ => None,
    };
    let options = expression
        .function
        .iter()
        .flat_map(|function| &function.options)
        .filter_map(|option| match &option.value {
            Operand::Variable(name) => Some(name.as_str()),
            Operand::Literal(_) => None,
        });
    operand.into_iter().chain(options)
}

/// Whether the selector's declaration, or one it refers to, has a function.
fn annotated(selector: &str, declarations: &[Declaration]) -> bool {
    let mut name = selector;
    // Each step goes to an earlier declaration, so this ends.
    for _ in 0..=declarations.len() {
        let Some(declaration) = declarations
            .iter()
            .rev()
            .find(|declaration| declaration.name() == name)
        else {
            return false;
        };
        let expression = declaration.expression();
        if expression.function.is_some() {
            return true;
        }
        match (&expression.operand, declaration) {
            (Some(Operand::Variable(next)), Declaration::Local { .. }) => name = next,
            _ => return false,
        }
    }
    false
}

fn pattern_options(pattern: &[PatternPart]) -> Result<(), DataModelError> {
    for part in pattern {
        match part {
            PatternPart::Expression(expression) => {
                if let Some(function) = &expression.function {
                    unique_options(&function.options)?;
                }
            }
            PatternPart::Markup(markup) => unique_options(&markup.options)?,
            PatternPart::Text(_) => {}
        }
    }
    Ok(())
}

fn unique_options(options: &[FunctionOption]) -> Result<(), DataModelError> {
    let mut names = HashSet::new();
    for option in options {
        if !names.insert(option.name.as_str()) {
            return Err(DataModelError::DuplicateOptionName(option.name.clone()));
        }
    }
    Ok(())
}
