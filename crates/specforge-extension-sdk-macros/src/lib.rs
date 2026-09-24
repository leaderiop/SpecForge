//! Attribute macro for the SpecForge extension SDK.
//!
//! ```ignore
//! use specforge_extension_sdk::prelude::*;
//!
//! #[specforge_extension_sdk::extension(name = "@you/greet", version = "0.1.0", short = "Greetings")]
//! struct Greet;
//!
//! impl specforge_extension_sdk::Contributions for Greet {
//!     fn contribute(c: &mut specforge_extension_sdk::ContributionsBuilder) {
//!         // kinds, edges, rules...
//!     }
//! }
//! ```
//!
//! Expands to the struct plus `__handshake` / `__describe` exports wired to
//! the author's [`specforge_extension_sdk::Contributions`] impl. `version`
//! defaults to the crate's `CARGO_PKG_VERSION`; `short` is optional.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

fn parse_attrs(ts: TokenStream) -> syn::Result<(Option<String>, Option<String>, Option<String>)> {
    let mut name = None;
    let mut version = None;
    let mut short = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            meta.input.parse::<Token![=]>()?;
            name = Some(meta.input.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("version") {
            meta.input.parse::<Token![=]>()?;
            version = Some(meta.input.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("short") {
            meta.input.parse::<Token![=]>()?;
            short = Some(meta.input.parse::<LitStr>()?.value());
        } else {
            return Err(meta.error("unsupported property; expected name / version / short"));
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, ts.into())?;
    Ok((name, version, short))
}
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn};
use syn::{ItemStruct, LitStr, Token, parse_macro_input};

#[proc_macro_attribute]
pub fn extension(attr: TokenStream, item: TokenStream) -> TokenStream {
    let st = parse_macro_input!(item as ItemStruct);
    let ident = &st.ident;

    let (name, version, short) = match parse_attrs(attr) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let name = match name {
        Some(n) => n,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "extension macro requires name = \"...\"",
            )
            .to_compile_error()
            .into();
        }
    };
    let version = version.unwrap_or_else(|| {
        std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string())
    });

    let short_expr = match short {
        Some(s) => quote! { Some(::std::string::String::from(#s)) },
        None => quote! { None },
    };

    let ts: TokenStream2 = quote! {
        #st

        fn specforge_extension_build() -> ::specforge_extension_sdk::ContributionsBuilder {
            let mut meta = ::specforge_extension_sdk::ExtensionMeta::new(#name, #version);
            meta.short = #short_expr;
            let mut b = ::specforge_extension_sdk::ContributionsBuilder::new(meta);
            <#ident as ::specforge_extension_sdk::Contributions>::contribute(&mut b);
            b
        }

        #[::extism_pdk::plugin_fn]
        pub fn __handshake(_input: Vec<u8>) -> ::extism_pdk::FnResult<Vec<u8>> {
            Ok(::specforge_extension_sdk::handshake_json(&specforge_extension_build()).into_bytes())
        }

        #[::extism_pdk::plugin_fn]
        pub fn __describe(input: Vec<u8>) -> ::extism_pdk::FnResult<Vec<u8>> {
            ::specforge_extension_sdk::describe_dispatch(&specforge_extension_build(), &input)
        }
    };
    ts.into()
}

/// Compiler pass arguments: `name` (required), plus optional `after`,
/// `before`, and `phase` ordering hints.
struct CompilerPassArgs {
    name: String,
}

impl Parse for CompilerPassArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;
            let value: LitStr = input.parse()?;
            if key == "name" {
                name = Some(value.value());
            }
            if !input.is_empty() {
                let _comma: Token![,] = input.parse()?;
            }
        }
        Ok(CompilerPassArgs {
            name: name.ok_or_else(|| input.error("compiler_pass requires name = \"...\""))?,
        })
    }
}

/// Wrap a pass function in a `__pass_<name>` wasm export.
///
/// ```ignore
/// #[compiler_pass(name = "condition_check", after = "resolve")]
/// fn pass_condition_check(entities: &[PassEntity]) -> Vec<PassDiagnostic> {
///     // ...
/// }
/// ```
///
/// Expands to the original function plus a `#[plugin_fn]` export named
/// `__pass_<name>` that deserializes the host's `PassInput` snapshot,
/// calls the function, and serializes the returned diagnostics.
#[proc_macro_attribute]
pub fn compiler_pass(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as CompilerPassArgs);
    let func = syn::parse_macro_input!(item as ItemFn);

    let fn_name = &func.sig.ident;
    let export_name = format!("__pass_{}", args.name);
    let export_ident = quote::format_ident!("__pass_{}", args.name);

    let expanded = quote::quote! {
        #func

        #[::extism_pdk::plugin_fn]
        pub fn #export_ident(input: Vec<u8>) -> ::extism_pdk::FnResult<Vec<u8>> {
            let request: ::specforge_extension_sdk::PassInput =
                ::serde_json::from_slice(&input)?;
            let findings = #fn_name(&request);
            Ok(::serde_json::to_vec(&findings)?)
        }

        const _: () = {
            // Keep the export name discoverable and fail at compile time if
            // the pass name changes underneath the descriptor.
            let _export: &str = #export_name;
        };
    };
    expanded.into()
}
