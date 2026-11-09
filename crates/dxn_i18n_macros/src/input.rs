//! What the macros are called with.

use proc_macro2::{Span, TokenStream};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Ident, Lit, LitStr, Token};
use unicode_normalization::UnicodeNormalization;

/// An argument's name, in Unicode NFC (as `Args` compares names), and where the call names it.
pub(crate) struct ArgName {
    pub name: String,
    pub span: Span,
}

/// `name = value`, `"name" = value` or a lone `name`; only the name is kept.
struct Arg(ArgName);

impl Parse for Arg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (name, span, needs_value) = if input.peek(LitStr) {
            let literal: LitStr = input.parse()?;
            (literal.value(), literal.span(), true)
        } else {
            let ident = Ident::parse_any(input)?;
            (ident.unraw().to_string(), ident.span(), false)
        };
        if needs_value || input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse::<Expr>()?;
        }
        Ok(Self(ArgName {
            name: name.nfc().collect(),
            span,
        }))
    }
}

fn args(input: ParseStream) -> syn::Result<Vec<ArgName>> {
    Ok(Punctuated::<Arg, Token![,]>::parse_terminated(input)?
        .into_iter()
        .map(|Arg(name)| name)
        .collect())
}

/// The id as a string literal, through the invisible groups a `macro_rules!` fragment
/// forwards it in.
fn literal_id(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(literal),
            ..
        }) => Some(literal),
        Expr::Group(group) => literal_id(&group.expr),
        Expr::Paren(paren) => literal_id(&paren.expr),
        _ => None,
    }
}

fn id(input: ParseStream) -> syn::Result<LitStr> {
    let expr: Expr = input.parse()?;
    literal_id(&expr).cloned().ok_or_else(|| {
        syn::Error::new_spanned(
            &expr,
            "a checked message id is a string literal; `dxn_i18n::t!` takes an id made at run time",
        )
    })
}

/// `t!(translator, id, args…)`.
pub(crate) struct TInput {
    pub id: LitStr,
    pub args: Vec<ArgName>,
}

impl Parse for TInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Expr>()?;
        input.parse::<Token![,]>()?;
        let id = id(input)?;
        let args = if input.is_empty() {
            Vec::new()
        } else {
            input.parse::<Token![,]>()?;
            args(input)?
        };
        Ok(Self { id, args })
    }
}

/// A framework's default for where the messages are, before the crate's metadata.
pub(crate) struct Default {
    pub key: Ident,
    pub value: LitStr,
}

/// `check!(id, (args…), key = "value", … => expansion)`.
pub(crate) struct CheckInput {
    pub id: LitStr,
    pub args: Vec<ArgName>,
    pub defaults: Vec<Default>,
    pub expansion: TokenStream,
}

impl Parse for CheckInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let id = id(input)?;
        input.parse::<Token![,]>()?;
        let content;
        syn::parenthesized!(content in input);
        let args = args(&content)?;
        let mut defaults = Vec::new();
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.peek(Token![=>]) {
                break;
            }
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            defaults.push(Default {
                key,
                value: input.parse()?,
            });
        }
        input.parse::<Token![=>]>()?;
        Ok(Self {
            id,
            args,
            defaults,
            expansion: input.parse()?,
        })
    }
}
