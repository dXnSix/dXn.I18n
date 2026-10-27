use std::fmt;
use std::sync::Arc;

use dxn_intl::{Locale, negotiate};

use crate::message::{FormatError, Formatted, MessageFormatter, Part};
use crate::{Args, BidiIsolation, Bundle, FunctionRegistry};

/// What [`Translator::format`] shows for a message no bundle has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MissingMessage {
    /// The message's id, so the gap is visible and the screen still reads.
    #[default]
    Id,
    /// Nothing.
    Empty,
}

/// Why a message could not be formatted whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TranslateError {
    /// No bundle in the lookup chain has the message.
    Missing(String),
    /// The message formatted with fallbacks for these errors.
    Format {
        /// The message's id.
        id: String,
        /// The formatted text, fallbacks included.
        text: String,
        /// What went wrong.
        errors: Vec<FormatError>,
    },
}

impl fmt::Display for TranslateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(id) => write!(f, "no message `{id}`"),
            Self::Format { id, errors, .. } => {
                write!(f, "message `{id}`:")?;
                for error in errors {
                    write!(f, " {error};")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for TranslateError {}

/// An app's translations: a bundle per locale, the user's locale and the fallbacks behind it.
///
/// A message is looked up in the bundle that best fits the locale (see
/// [`dxn_intl::negotiate`]), then in its parents (`de-AT` → `de`), then in the fallback
/// locales' bundles, then in the default locale's.
///
/// ```
/// use dxn_i18n::{Args, Bundle, Translator};
/// use dxn_intl::parse_locale;
///
/// let mut en = Bundle::new(parse_locale("en")?);
/// en.add_json(r#"{ "hello": "Hello, {$name}!", "bye": "Goodbye!" }"#)?;
/// let mut de = Bundle::new(parse_locale("de")?);
/// de.add_json(r#"{ "hello": "Hallo, {$name}!" }"#)?;
///
/// let mut translator = Translator::new(parse_locale("en")?);
/// translator.add_bundle(en);
/// translator.add_bundle(de);
/// translator.set_locale(parse_locale("de-AT")?);
///
/// let args = Args::new().with("name", "Ana");
/// assert_eq!(translator.format("hello", &args), "Hallo, Ana!");
/// assert_eq!(translator.format("bye", &args), "Goodbye!"); // from the default locale
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Translator {
    bundles: Vec<Bundle>,
    default_locale: Locale,
    fallbacks: Vec<Locale>,
    locale: Locale,
    chain: Vec<usize>,
    functions: Arc<FunctionRegistry>,
    bidi: BidiIsolation,
    missing: MissingMessage,
}

impl Translator {
    /// A translator with no bundles whose default (and current) locale is `default_locale`.
    pub fn new(default_locale: Locale) -> Self {
        Self {
            bundles: Vec::new(),
            locale: default_locale.clone(),
            default_locale,
            fallbacks: Vec::new(),
            chain: Vec::new(),
            functions: FunctionRegistry::shared_default(),
            bidi: BidiIsolation::None,
            missing: MissingMessage::Id,
        }
    }

    /// Adds `bundle`; a bundle of a locale the translator has already is merged into it.
    pub fn add_bundle(&mut self, bundle: Bundle) {
        match self
            .bundles
            .iter_mut()
            .find(|existing| existing.locale() == bundle.locale())
        {
            Some(existing) => existing.merge(bundle),
            None => self.bundles.push(bundle),
        }
        self.relink();
    }

    /// The bundles, in the order they were added.
    pub fn bundles(&self) -> &[Bundle] {
        &self.bundles
    }

    /// The locales the translator has bundles for.
    pub fn available_locales(&self) -> Vec<Locale> {
        self.bundles
            .iter()
            .map(|bundle| bundle.locale().clone())
            .collect()
    }

    /// The user's locale.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Makes `locale` the user's locale. Its Unicode extensions (`-u-hc-h23`, `-u-nu-latn`)
    /// apply to the values messages format.
    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
        self.relink();
    }

    /// The locale messages fall back to when none fits better, last in the chain.
    pub fn default_locale(&self) -> &Locale {
        &self.default_locale
    }

    /// Sets the locales tried, in order, after the user's locale and before the default.
    pub fn set_fallbacks(&mut self, fallbacks: Vec<Locale>) {
        self.fallbacks = fallbacks;
        self.relink();
    }

    /// Sets the functions messages can call.
    pub fn set_functions(&mut self, functions: Arc<FunctionRegistry>) {
        self.functions = functions;
    }

    /// Sets how placeholders are isolated in mixed-direction text.
    pub fn set_bidi_isolation(&mut self, bidi: BidiIsolation) {
        self.bidi = bidi;
    }

    /// Sets what a missing message formats to.
    pub fn set_missing_message(&mut self, missing: MissingMessage) {
        self.missing = missing;
    }

    /// The bundles' locales in lookup order.
    pub fn lookup_chain(&self) -> Vec<&Locale> {
        self.chain
            .iter()
            .map(|index| self.bundles[*index].locale())
            .collect()
    }

    /// Whether a bundle in the lookup chain has the message `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.find(id).is_some()
    }

    /// The message `id` formatted with `args`; a missing message shows as
    /// [`MissingMessage`] says, and a placeholder that fails as its fallback (`{$name}`).
    pub fn format(&self, id: &str, args: &Args) -> String {
        match self.try_format(id, args) {
            Ok(text) | Err(TranslateError::Format { text, .. }) => text,
            Err(TranslateError::Missing(_)) => match self.missing {
                MissingMessage::Id => id.to_string(),
                MissingMessage::Empty => String::new(),
            },
        }
    }

    /// The message `id` formatted with `args`, or why it could not be formatted whole.
    pub fn try_format(&self, id: &str, args: &Args) -> Result<String, TranslateError> {
        let (bundle, formatter) = self
            .find(id)
            .ok_or_else(|| TranslateError::Missing(id.to_string()))?;
        let message = bundle
            .get(id)
            .ok_or_else(|| TranslateError::Missing(id.to_string()))?;
        let Formatted { text, errors } = formatter.format(message, args);
        if errors.is_empty() {
            Ok(text)
        } else {
            Err(TranslateError::Format {
                id: id.to_string(),
                text,
                errors,
            })
        }
    }

    /// The message `id` as parts, for UIs that render markup; `None` when it is missing.
    pub fn format_to_parts(&self, id: &str, args: &Args) -> Option<Formatted<Vec<Part>>> {
        let (bundle, formatter) = self.find(id)?;
        Some(formatter.format_to_parts(bundle.get(id)?, args))
    }

    fn find(&self, id: &str) -> Option<(&Bundle, MessageFormatter)> {
        let bundle = self
            .chain
            .iter()
            .map(|index| &self.bundles[*index])
            .find(|bundle| bundle.contains(id))?;
        Some((bundle, self.formatter_for(bundle.locale())))
    }

    /// The locale a bundle's messages format values in: the user's locale when it is the same
    /// language (so `de-CH`'s separators apply to a `de` bundle), else the bundle's, with the
    /// user's Unicode extensions either way.
    fn formatter_for(&self, bundle_locale: &Locale) -> MessageFormatter {
        let locale = if bundle_locale.id.language == self.locale.id.language {
            self.locale.clone()
        } else {
            let mut locale = bundle_locale.clone();
            locale.extensions.unicode = self.locale.extensions.unicode.clone();
            locale
        };
        MessageFormatter::new(locale)
            .with_functions(self.functions.clone())
            .with_bidi_isolation(self.bidi)
    }

    fn relink(&mut self) {
        let available = self.available_locales();
        let mut chain: Vec<usize> = Vec::new();
        let push = |locale: &Locale, chain: &mut Vec<usize>| {
            let mut step = Some(negotiate(
                std::slice::from_ref(locale),
                &available,
                &Locale::UNKNOWN,
            ));
            // The best fit, then the bundles of its parents: `de-AT`, `de`.
            while let Some(found) = step.take() {
                if let Some(index) = available
                    .iter()
                    .position(|candidate| candidate.id == found.id)
                    && !chain.contains(&index)
                {
                    chain.push(index);
                }
                if found.id.region.is_some() {
                    let mut parent = found.clone();
                    parent.id.region = None;
                    parent.extensions.unicode.clear();
                    if available.iter().any(|candidate| candidate.id == parent.id) {
                        step = Some(parent);
                    }
                }
            }
        };
        push(&self.locale.clone(), &mut chain);
        for fallback in self.fallbacks.clone() {
            push(&fallback, &mut chain);
        }
        push(&self.default_locale.clone(), &mut chain);
        self.chain = chain;
    }
}
