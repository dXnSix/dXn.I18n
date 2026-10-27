use std::collections::HashMap;
use std::fmt;

use dxn_intl::Locale;
use serde_json::Value as Json;

use crate::{Message, ParseError};

/// One locale's messages, by id.
///
/// Messages come one by one ([`Bundle::add`]) or from JSON resources ([`Bundle::add_json`]),
/// whose nested objects make dotted ids:
///
/// ```
/// use dxn_i18n::Bundle;
/// use dxn_intl::parse_locale;
///
/// let mut bundle = Bundle::new(parse_locale("en")?);
/// bundle.add_json(r#"{ "cart": { "title": "Your cart", "empty": "Nothing here yet." } }"#)?;
/// assert!(bundle.get("cart.title").is_some());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Bundle {
    locale: Locale,
    messages: HashMap<String, Message>,
}

impl Bundle {
    /// An empty bundle for `locale`.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            messages: HashMap::new(),
        }
    }

    /// The bundle's locale.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Parses `source` and adds it as `id`, replacing a message with that id.
    pub fn add(&mut self, id: impl Into<String>, source: &str) -> Result<(), ParseError> {
        self.messages.insert(id.into(), Message::parse(source)?);
        Ok(())
    }

    /// Adds every message of a JSON resource: an object whose values are message texts or
    /// nested objects (`{"cart": {"title": …}}` is `cart.title`).
    ///
    /// The messages that parse are added even when others do not; the error lists the ones
    /// that did not.
    pub fn add_json(&mut self, json: &str) -> Result<(), LoadError> {
        self.add_json_with_prefix("", json)
    }

    /// As [`Bundle::add_json`], with every id under `prefix` (`"checkout"` makes
    /// `checkout.title`); an empty prefix adds the ids as they are.
    pub fn add_json_with_prefix(&mut self, prefix: &str, json: &str) -> Result<(), LoadError> {
        let root: Json =
            serde_json::from_str(json).map_err(|error| LoadError::Json(error.to_string()))?;
        let Json::Object(_) = root else {
            return Err(LoadError::Json("a resource is a JSON object".to_string()));
        };
        let mut failures = Vec::new();
        self.walk(prefix, &root, &mut failures);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(LoadError::Messages(failures))
        }
    }

    fn walk(&mut self, id: &str, node: &Json, failures: &mut Vec<(String, MessageError)>) {
        match node {
            Json::Object(entries) => {
                for (key, child) in entries {
                    let child_id = if id.is_empty() {
                        key.clone()
                    } else {
                        format!("{id}.{key}")
                    };
                    self.walk(&child_id, child, failures);
                }
            }
            Json::String(source) => {
                if let Err(error) = self.add(id, source) {
                    failures.push((id.to_string(), MessageError::Parse(error)));
                }
            }
            _ => failures.push((id.to_string(), MessageError::NotText)),
        }
    }

    /// The message `id`.
    pub fn get(&self, id: &str) -> Option<&Message> {
        self.messages.get(id)
    }

    /// Whether the bundle has the message `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.messages.contains_key(id)
    }

    /// The ids of the bundle's messages, in no particular order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.messages.keys().map(String::as_str)
    }

    /// How many messages the bundle has.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the bundle has no messages.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Adds `other`'s messages, replacing those with the same ids.
    pub fn merge(&mut self, other: Bundle) {
        self.messages.extend(other.messages);
    }
}

/// A resource that could not be loaded whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The resource is not a JSON object.
    Json(String),
    /// These messages were left out; the others were added.
    Messages(Vec<(String, MessageError)>),
}

/// Why a message of a resource was left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageError {
    /// Its value is not text or an object (a number, a list, …).
    NotText,
    /// Its text is not a valid message.
    Parse(ParseError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(reason) => write!(f, "not a message resource: {reason}"),
            Self::Messages(failures) => {
                f.write_str("messages left out:")?;
                for (id, error) in failures {
                    match error {
                        MessageError::NotText => write!(f, " `{id}` (not text);")?,
                        MessageError::Parse(error) => write!(f, " `{id}` ({error});")?,
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LoadError {}
