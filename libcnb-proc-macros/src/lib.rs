#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]

use proc_macro::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input;
use syn::Token;

/// Compiles the given regex using the `fancy_regex` crate and tries to match the given value. If
/// the value matches the regex, the macro will expand to the first expression. Otherwise it will
/// expand to the second expression.
///
/// It is designed to be used within other macros to produce compile time errors when the regex
/// doesn't match but it might work for other use-cases as well.
///
/// ```no_run
/// libcnb_proc_macros::verify_regex!("^A-Z+$", "foobar", println!("It did match!"), println!("It did not match!"));
/// ```
#[proc_macro]
pub fn verify_regex(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as VerifyRegexInput);

    let token_stream = match fancy_regex::Regex::new(&input.regex.value()) {
        Ok(regex) => {
            let regex_matches = regex.is_match(&input.value.value()).unwrap_or(false);

            let expression = if regex_matches {
                input.expression_when_matched
            } else {
                input.expression_when_unmatched
            };

            quote! { #expression }
        }
        Err(err) => syn::Error::new(
            input.regex.span(),
            format!("Could not compile regular expression: {err}"),
        )
        .to_compile_error(),
    };

    token_stream.into()
}

struct VerifyRegexInput {
    regex: syn::LitStr,
    value: syn::LitStr,
    expression_when_matched: syn::Expr,
    expression_when_unmatched: syn::Expr,
}

impl Parse for VerifyRegexInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let regex: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let value: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let expression_when_matched: syn::Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let expression_when_unmatched: syn::Expr = input.parse()?;

        Ok(Self {
            regex,
            value,
            expression_when_matched,
            expression_when_unmatched,
        })
    }
}

#[proc_macro]
pub fn verify_bin_target_exists(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as VerifyBinTargetExistsInput);

    let cargo_metadata = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok()
        .map(|cargo_manifest_dir| {
            cargo_metadata::MetadataCommand::new()
                .manifest_path(cargo_manifest_dir.join("Cargo.toml"))
                .exec()
        })
        .transpose();

    let token_stream = if let Ok(Some(cargo_metadata)) = cargo_metadata {
        if let Some(root_package) = cargo_metadata.root_package() {
            let valid_target = root_package
                .targets
                .iter()
                .any(|target| target.name == input.target_name.value());

            let expression = if valid_target {
                input.expression_when_matched
            } else {
                input.expression_when_unmatched
            };

            quote! {
                #expression
            }
        } else {
            quote! {
                compile_error!("Cannot read root package for this crate!")
            }
        }
    } else {
        quote! {
            compile_error!("Cannot read Cargo metadata!")
        }
    };

    token_stream.into()
}

struct VerifyBinTargetExistsInput {
    target_name: syn::LitStr,
    expression_when_matched: syn::Expr,
    expression_when_unmatched: syn::Expr,
}

impl Parse for VerifyBinTargetExistsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let target_name: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let expression_when_matched: syn::Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let expression_when_unmatched: syn::Expr = input.parse()?;

        Ok(Self {
            target_name,
            expression_when_matched,
            expression_when_unmatched,
        })
    }
}
