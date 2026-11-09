//! Compile-time checked translations for [`dxn_i18n`]: a message id that is not in the crate's
//! messages, an argument a message does not take, or one it needs and the call does not pass,
//! fails the build instead of showing a fallback at run time.
//!
//! [`t!`] is `dxn_i18n::t!` with the check; [`check!`] is the check alone, for a framework's
//! own translation macro. Both read the messages of the crate being built, as
//! `[package.metadata.dxn_i18n]` in its `Cargo.toml` says:
//!
//! ```toml
//! [package.metadata.dxn_i18n]
//! messages = "locales"              # the directory, relative to Cargo.toml (the default)
//! default-locale = "en"             # the locale every id must be in (the default)
//! default-namespace = "translation" # a namespace whose ids are not prefixed (none by default)
//! ```
//!
//! The directory holds `<locale>.json` and `<locale>/<namespace>.json` resources, the JSON
//! [`dxn_i18n::Bundle::add_json`] reads; a namespace's ids are `<namespace>.<id>`, except the
//! default namespace's. An id must be in the default locale. The arguments are checked against
//! every locale's message: each one a translation reads is needed, and one that none reads is
//! an error.

mod catalog;
mod check;
mod input;

use proc_macro::TokenStream;

/// `dxn_i18n::t!(translator, id, name = value, …)`, with the id and the argument names checked
/// against the crate's messages when it builds.
///
/// ```rust,ignore
/// use dxn_i18n_macros::t;
///
/// // locales/en.json: { "cart": { "items": ".input {$count :number} .match $count one {{One item}} * {{{$count} items}}" } }
/// let text = t!(translator, "cart.items", count = 3);
/// let text = t!(translator, "cart.itmes", count = 3); // error: no message `cart.itmes`; did you mean `cart.items`?
/// let text = t!(translator, "cart.items");            // error: message `cart.items` needs the argument `$count`
/// ```
///
/// The id is a string literal; `dxn_i18n::t!` takes an id made at run time. The crate depends
/// on `dxn_i18n`, which the expansion calls.
#[proc_macro]
pub fn t(input: TokenStream) -> TokenStream {
    check::expand_t(input.into(), manifest_dir().as_deref()).into()
}

/// Checks a message id and argument names against the crate's messages, then expands to the
/// expression after `=>`: the check alone, for a framework's own translation macro.
///
/// ```rust,ignore
/// check!("cart.items", (count = 3), messages = "assets/locales" => my_framework::translate("cart.items", &args))
/// ```
///
/// The parentheses hold the call's arguments as `t!` takes them (`name = value`, `"name" =
/// value` or a lone `name`); only their names are read. `messages`, `default_locale` and
/// `default_namespace` before `=>` are the framework's defaults, which the crate's
/// `[package.metadata.dxn_i18n]` overrides.
#[proc_macro]
pub fn check(input: TokenStream) -> TokenStream {
    check::expand_check(input.into(), manifest_dir().as_deref()).into()
}

/// The directory of the crate being built, whose `Cargo.toml` and messages are read.
fn manifest_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("CARGO_MANIFEST_DIR").map(std::path::PathBuf::from)
}
