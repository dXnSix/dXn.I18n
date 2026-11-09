//! Checking a call against the catalog, and what the macros expand to.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::LitStr;

use crate::catalog::{self, Catalog, Settings, same_locale};
use crate::input::{ArgName, CheckInput, Default, TInput};

/// `t!`: the check, then `dxn_i18n::t!` with the call's own tokens.
pub(crate) fn expand_t(input: TokenStream, manifest_dir: Option<&Path>) -> TokenStream {
    let parsed: TInput = match syn::parse2(input.clone()) {
        Ok(parsed) => parsed,
        Err(error) => return compile_errors(error),
    };
    let expansion = quote! { ::dxn_i18n::t!(#input) };
    finish(&parsed.id, &parsed.args, &[], manifest_dir, expansion)
}

/// `check!`: the check, then the framework's expansion.
pub(crate) fn expand_check(input: TokenStream, manifest_dir: Option<&Path>) -> TokenStream {
    let parsed: CheckInput = match syn::parse2(input) {
        Ok(parsed) => parsed,
        Err(error) => return compile_errors(error),
    };
    finish(
        &parsed.id,
        &parsed.args,
        &parsed.defaults,
        manifest_dir,
        parsed.expansion,
    )
}

/// The expansion in a block that includes every message file, so the crate rebuilds when one
/// changes; or the errors.
fn finish(
    id: &LitStr,
    args: &[ArgName],
    defaults: &[Default],
    manifest_dir: Option<&Path>,
    expansion: TokenStream,
) -> TokenStream {
    match run(id, args, defaults, manifest_dir) {
        Ok(files) => {
            let includes = files.iter().map(|file| {
                let path = LitStr::new(&file.to_string_lossy(), Span::call_site());
                quote! { const _: &[u8] = ::core::include_bytes!(#path); }
            });
            quote! { { #(#includes)* #expansion } }
        }
        Err(errors) => compile_errors(errors),
    }
}

/// One `compile_error!` per error, as the statements of a block, so several of them are still
/// one expression.
fn compile_errors(errors: syn::Error) -> TokenStream {
    let errors = errors.into_iter().map(|error| error.to_compile_error());
    quote! { { #(#errors);* } }
}

fn run(
    id: &LitStr,
    args: &[ArgName],
    defaults: &[Default],
    manifest_dir: Option<&Path>,
) -> Result<Vec<PathBuf>, syn::Error> {
    let span = id.span();
    let manifest_dir = manifest_dir.ok_or_else(|| {
        syn::Error::new(
            span,
            "CARGO_MANIFEST_DIR is not set; checked messages are read beside the Cargo.toml of \
             the crate being built",
        )
    })?;
    let mut settings = Settings::default();
    for default in defaults {
        let value = default.value.value();
        match default.key.to_string().as_str() {
            "messages" => settings.messages = value,
            "default_locale" => settings.default_locale = value,
            "default_namespace" => settings.default_namespace = Some(value),
            other => {
                return Err(syn::Error::new(
                    default.key.span(),
                    format!(
                        "`{other}` is no setting; the settings are `messages`, \
                         `default_locale` and `default_namespace`"
                    ),
                ));
            }
        }
    }
    let settings = catalog::settings(manifest_dir, settings)
        .map_err(|message| syn::Error::new(span, message))?;
    let catalog =
        catalog::load(manifest_dir, &settings).map_err(|message| syn::Error::new(span, message))?;
    let mut errors = check(&catalog, &settings, &id.value(), span, args).into_iter();
    match errors.next() {
        None => Ok(catalog.files.clone()),
        Some(mut first) => {
            first.extend(errors);
            Err(first)
        }
    }
}

/// What is wrong with calling message `id` with `args`.
fn check(
    catalog: &Catalog,
    settings: &Settings,
    id: &str,
    span: Span,
    args: &[ArgName],
) -> Vec<syn::Error> {
    let error = |span: Span, message: String| syn::Error::new(span, message);
    let Some((default_tag, default)) = catalog
        .locales
        .iter()
        .find(|(tag, _)| same_locale(tag, &settings.default_locale))
    else {
        return vec![error(
            span,
            format!(
                "`{}` has no messages for the default locale `{}`; set `default-locale` in \
                 `[package.metadata.dxn_i18n]`",
                settings.messages, settings.default_locale
            ),
        )];
    };
    if !default.contains_key(id) {
        let mut message = format!(
            "no message `{id}` in the default locale `{default_tag}` (`{}`)",
            settings.messages
        );
        if let Some(near) = closest(id, default.keys()) {
            message.push_str(&format!("; did you mean `{near}`?"));
        }
        return vec![error(span, message)];
    }

    let mut errors = Vec::new();
    // Each argument a translation reads, and the locales that read it.
    let mut readers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut translations = 0;
    for (tag, messages) in &catalog.locales {
        let Some(entry) = messages.get(id) else {
            continue;
        };
        translations += 1;
        match &entry.arguments {
            Ok(arguments) => {
                for argument in arguments {
                    readers.entry(argument).or_default().push(tag);
                }
            }
            Err(problem) => errors.push(error(
                span,
                format!("message `{id}` in `{}` {problem}", entry.file),
            )),
        }
    }

    let mut given: BTreeMap<&str, Span> = BTreeMap::new();
    for arg in args {
        if given.insert(&arg.name, arg.span).is_some() {
            errors.push(error(
                arg.span,
                format!("the argument `${}` is given twice", arg.name),
            ));
        } else if !readers.contains_key(arg.name.as_str()) {
            let takes = if readers.is_empty() {
                "it takes no arguments".to_owned()
            } else {
                format!(
                    "it takes {}",
                    list(readers.keys().map(|name| format!("`${name}`")))
                )
            };
            errors.push(error(
                arg.span,
                format!("message `{id}` has no argument `${}`; {takes}", arg.name),
            ));
        }
    }
    for (argument, locales) in &readers {
        if given.contains_key(argument) {
            continue;
        }
        let mut message = format!("message `{id}` needs the argument `${argument}`");
        if locales.len() < translations {
            message.push_str(&format!(
                ", which its {} translation reads",
                list(locales.iter().map(|tag| format!("`{tag}`")))
            ));
        }
        errors.push(error(span, message));
    }
    errors
}

/// `a`, `a and b`, `a, b and c`.
fn list(items: impl Iterator<Item = String>) -> String {
    let items: Vec<String> = items.collect();
    match items.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// The id nearest `id` by edit distance, if one is near enough to be a typo of it.
fn closest<'a>(id: &str, ids: impl Iterator<Item = &'a String>) -> Option<&'a str> {
    let limit = (id.chars().count() / 3).max(1);
    ids.map(|candidate| (distance(id, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate.as_str())
}

/// Levenshtein distance, in characters.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, a) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, b) in b.iter().enumerate() {
            let substitution = previous + usize::from(a != *b);
            previous = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(previous + 1);
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests;
