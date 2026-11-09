//! `args!` and `t!`: arguments and translations at the call site.

use crate::{Args, Translator};

/// What [`t!`](crate::t) formats messages with: a [`Translator`], or a framework's service that
/// holds one.
///
/// A reference or smart pointer to a `Translate` is one too, so `t!` takes a translator, a
/// `&Translator` or an `Arc<Translator>` alike.
pub trait Translate {
    /// The message `id` formatted with `args`. It never fails: a missing message and a
    /// placeholder that cannot be formatted show as [`Translator::format`] says.
    fn translate(&self, id: &str, args: &Args) -> String;
}

impl Translate for Translator {
    fn translate(&self, id: &str, args: &Args) -> String {
        self.format(id, args)
    }
}

impl<T: Translate + ?Sized> Translate for &T {
    fn translate(&self, id: &str, args: &Args) -> String {
        (**self).translate(id, args)
    }
}

impl<T: Translate + ?Sized> Translate for &mut T {
    fn translate(&self, id: &str, args: &Args) -> String {
        (**self).translate(id, args)
    }
}

impl<T: Translate + ?Sized> Translate for Box<T> {
    fn translate(&self, id: &str, args: &Args) -> String {
        (**self).translate(id, args)
    }
}

impl<T: Translate + ?Sized> Translate for std::rc::Rc<T> {
    fn translate(&self, id: &str, args: &Args) -> String {
        (**self).translate(id, args)
    }
}

impl<T: Translate + ?Sized> Translate for std::sync::Arc<T> {
    fn translate(&self, id: &str, args: &Args) -> String {
        (**self).translate(id, args)
    }
}

/// Builds [`Args`] from `name = value` pairs, keeping each value's type, so a number stays a
/// number for `.match` and `:number`.
///
/// A name is an identifier or, for a variable name Rust cannot spell (`$user-name`), a string
/// literal; a lone identifier passes the variable of that name, as `format!` does.
///
/// ```
/// use dxn_i18n::{Value, args};
///
/// let name = "Ana";
/// let args = args!(name, count = 3, "user-id" = 42, r#type = "admin");
/// assert_eq!(args.get("name").and_then(Value::as_str), Some("Ana"));
/// assert_eq!(args.get("count"), Some(&Value::from(3)));
/// assert_eq!(args.get("user-id"), Some(&Value::from(42)));
/// assert_eq!(args.get("type").and_then(Value::as_str), Some("admin"));
/// assert!(args!().is_empty());
/// ```
#[macro_export]
macro_rules! args {
    ($($args:tt)*) => {{
        #[allow(unused_mut)]
        let mut args = $crate::Args::new();
        $crate::__args!(args; $($args)*);
        args
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __args {
    ($args:ident;) => {};
    ($args:ident; $name:ident = $value:expr $(, $($rest:tt)*)?) => {
        $args.set($crate::__private::variable_name(stringify!($name)), $value);
        $crate::__args!($args; $($($rest)*)?);
    };
    ($args:ident; $name:literal = $value:expr $(, $($rest:tt)*)?) => {
        $args.set($name, $value);
        $crate::__args!($args; $($($rest)*)?);
    };
    ($args:ident; $name:ident $(, $($rest:tt)*)?) => {
        $args.set($crate::__private::variable_name(stringify!($name)), $name);
        $crate::__args!($args; $($($rest)*)?);
    };
}

/// Formats the message `id` with a [`Translate`] (a [`Translator`]) and the arguments
/// [`args!`](crate::args) takes: `t!(translator, id, name = value, …)`.
///
/// ```
/// use dxn_i18n::{Bundle, Translator, t};
/// use dxn_intl::parse_locale;
///
/// let mut en = Bundle::new(parse_locale("en")?);
/// en.add_json(r#"{
///   "hello": "Hello, {$name}!",
///   "items": ".input {$count :number}\n.match $count\none {{One item}}\n* {{{$count} items}}"
/// }"#)?;
/// let mut translator = Translator::new(parse_locale("en")?);
/// translator.add_bundle(en);
///
/// let name = "Ana";
/// assert_eq!(t!(translator, "hello", name), "Hello, Ana!");
/// assert_eq!(t!(translator, "items", count = 1), "One item");
/// assert_eq!(t!(&translator, "items", count = 1200), "1,200 items");
/// assert_eq!(t!(translator, "missing"), "missing");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[macro_export]
macro_rules! t {
    ($translator:expr, $id:expr $(,)?) => {
        $crate::Translate::translate(&$translator, $id, &$crate::Args::new())
    };
    ($translator:expr, $id:expr, $($args:tt)+) => {
        $crate::Translate::translate(&$translator, $id, &$crate::args!($($args)+))
    };
}

/// What the macros use; not part of the API.
#[doc(hidden)]
pub mod __private {
    /// An argument's name as `stringify!` spells an identifier, without a raw identifier's
    /// `r#`.
    pub fn variable_name(identifier: &'static str) -> &'static str {
        identifier.strip_prefix("r#").unwrap_or(identifier)
    }
}
