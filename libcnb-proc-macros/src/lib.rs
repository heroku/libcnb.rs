// Enable rustc and Clippy lints that are disabled by default.
// https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unused-crate-dependencies
#![warn(unused_crate_dependencies)]
// https://rust-lang.github.io/rust-clippy/stable/index.html
#![warn(clippy::pedantic)]

use cargo_metadata::{Metadata, MetadataCommand};
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
        Err(_) => {
            quote! {
                compile_error!(concat!("Could not compile regular expression: ", #(verify_regex_input.regex)));
            }
        }
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
pub fn path_to_packaged_crate_binary(input: TokenStream) -> TokenStream {
    let bin_target_name = parse_macro_input!(input as syn::LitStr).value();

    let cargo_metadata = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok()
        .map(|cargo_manifest_dir| {
            MetadataCommand::new()
                .manifest_path(cargo_manifest_dir.join("Cargo.toml"))
                .exec()
        })
        .transpose();

    let token_stream = if let Ok(Some(cargo_metadata)) = cargo_metadata {
        if let Some(root_package) = cargo_metadata.root_package() {
            let valid_target = root_package
                .targets
                .iter()
                .any(|target| target.name == bin_target_name);

            if valid_target {
                quote! {{
                    ::std::env::var("CNB_BUILDPACK_DIR")
                        .map(::std::path::PathBuf::from)
                        .expect("Could not read CNB_BUILDPACK_DIR environment variable")
                        .join(".libcnb-cargo")
                        .join("additional-bin")
                        .join(#bin_target_name)
                }}
            } else {
                quote! {
                    compile_error!(concat!("Given target name '", #bin_target_name ,"' is not a valid crate binary target!"))
                }
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
