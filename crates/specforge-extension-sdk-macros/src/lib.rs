//! Attribute macro for the SpecForge extension SDK.
//!
//! ```ignore
//! use specforge_extension_sdk::prelude::*;
//!
//! #[specforge_extension_sdk::extension(
//!     name = "@you/greet",
//!     version = "0.1.0",
//!     short = "greet",
//!     description = "Friendly greetings"
//! )]
//! struct Greet;
//!
//! impl specforge_extension_sdk::Contributions for Greet {
//!     fn contribute(c: &mut specforge_extension_sdk::ContributionsBuilder) {
//!         // kinds, edges, rules...
//!     }
//! }
//! ```
//!
//! Generates a `specforge_extension_build()` function serving the author's
//! [`specforge_extension_sdk::Contributions`] impl; pair it with the SDK's
//! `component_guest!` macro for the wasip2 component exports. `version`
//! defaults to the crate's `CARGO_PKG_VERSION`. `short` (the name routing
//! the extension's commands, `specforge <short> <command>`; lowercase kebab
//! case, the name's last segment when absent) and `description` (what a
//! package registry shows) are optional.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// The `#[extension(...)]` attributes.
#[derive(Debug, Default, PartialEq)]
struct Attrs {
    name: Option<String>,
    version: Option<String>,
    short: Option<String>,
    description: Option<String>,
}

/// Whether `short` can route commands: lowercase kebab case
/// (`[a-z][a-z0-9-]*`), the shape the host checks too (E030).
fn is_valid_short(short: &str) -> bool {
    let mut chars = short.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn parse_attrs(ts: TokenStream2) -> syn::Result<Attrs> {
    let mut attrs = Attrs::default();
    let parser = syn::meta::parser(|meta| {
        let slot = if meta.path.is_ident("name") {
            &mut attrs.name
        } else if meta.path.is_ident("version") {
            &mut attrs.version
        } else if meta.path.is_ident("short") {
            &mut attrs.short
        } else if meta.path.is_ident("description") {
            &mut attrs.description
        } else {
            return Err(
                meta.error("unsupported property; expected name / version / short / description")
            );
        };
        meta.input.parse::<Token![=]>()?;
        let value = meta.input.parse::<LitStr>()?;
        if meta.path.is_ident("short") && !is_valid_short(&value.value()) {
            return Err(syn::Error::new(
                value.span(),
                format!(
                    "short = \"{}\" is not lowercase kebab case ([a-z][a-z0-9-]*): it names \
                     the extension's CLI subcommand and MCP tool prefix; put prose in \
                     `description`",
                    value.value()
                ),
            ));
        }
        *slot = Some(value.value());
        Ok(())
    });
    syn::parse::Parser::parse2(parser, ts)?;
    Ok(attrs)
}
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn};
use syn::{ItemStruct, LitStr, Token, parse_macro_input};

#[proc_macro_attribute]
pub fn extension(attr: TokenStream, item: TokenStream) -> TokenStream {
    let st = parse_macro_input!(item as ItemStruct);
    let ident = &st.ident;

    let Attrs {
        name,
        version,
        short,
        description,
    } = match parse_attrs(attr.into()) {
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

    let optional = |value: Option<String>| match value {
        Some(s) => quote! { Some(::std::string::String::from(#s)) },
        None => quote! { None },
    };
    let short_expr = optional(short);
    let description_expr = optional(description);

    let ts: TokenStream2 = quote! {
        #st

        fn specforge_extension_build() -> ::specforge_extension_sdk::ContributionsBuilder {
            let mut meta = ::specforge_extension_sdk::ExtensionMeta::new(#name, #version);
            meta.short = #short_expr;
            meta.description = #description_expr;
            let mut b = ::specforge_extension_sdk::ContributionsBuilder::new(meta);
            <#ident as ::specforge_extension_sdk::Contributions>::contribute(&mut b);
            b
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

/// Wrap a pass function for the component bridge.
///
/// **Deprecated** (ADR 0013): declare the pass with its handler instead,
/// `c.pass("condition_check", |p| { p.after("resolve").run(pass_condition_check); })`;
/// the function this attribute wraps (`fn(&PassInput) -> Vec<PassDiagnostic>`)
/// is already the handler `PassBuilder::run` takes. The attribute keeps
/// generating `specforge_dispatch_pass_<name>`, which decodes the
/// `PassInput` and encodes the diagnostics, for a guest that still routes
/// `__pass_<name>` through `component_guest!`'s `handler` (a pass declared
/// with `raw_category`).
///
/// ```ignore
/// #[compiler_pass(name = "condition_check", after = "resolve")]
/// fn pass_condition_check(input: &PassInput) -> Vec<PassDiagnostic> {
///     // ...
/// }
/// ```
///
#[proc_macro_attribute]
pub fn compiler_pass(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as CompilerPassArgs);
    let func = syn::parse_macro_input!(item as ItemFn);

    let fn_name = &func.sig.ident;
    let dispatch_ident = quote::format_ident!("specforge_dispatch_pass_{}", args.name);

    let expanded = quote::quote! {
        #func

        /// Wire helper: deserializes the host's `PassInput` snapshot, calls
        /// the pass function, and serializes the returned diagnostics. The
        /// guest's `component_guest!` handler routes `__pass_<name>` here.
        pub fn #dispatch_ident(input: &[u8]) -> Result<Vec<u8>, String> {
            let request: ::specforge_extension_sdk::PassInput =
                ::serde_json::from_slice(input)
                    .map_err(|e| format!("invalid pass request: {e}"))?;
            let findings = #fn_name(&request);
            ::serde_json::to_vec(&findings)
                .map_err(|e| format!("pass serialization failed: {e}"))
        }
    };
    expanded.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(
        behavior = "load_extension_declaration",
        verify = "a short name that is not lowercase kebab case is refused when the extension is built"
    )]
    fn a_short_name_that_is_not_kebab_case_is_refused() {
        for bad in [
            "Friendly greetings",
            "Greet",
            "greet_x",
            "-greet",
            "1greet",
            "",
        ] {
            let attr = quote! { name = "@sdk/greet", short = #bad };
            let error = parse_attrs(attr).expect_err(bad);
            assert!(
                error.to_string().contains("is not lowercase kebab case"),
                "{bad}: {error}"
            );
        }
        let attr = quote! {
            name = "@sdk/greet", version = "0.1.0", short = "greet-2",
            description = "Friendly greetings"
        };
        assert_eq!(
            parse_attrs(attr).unwrap(),
            Attrs {
                name: Some("@sdk/greet".to_string()),
                version: Some("0.1.0".to_string()),
                short: Some("greet-2".to_string()),
                description: Some("Friendly greetings".to_string()),
            }
        );
    }

    #[test]
    fn an_unknown_attribute_is_refused() {
        let error = parse_attrs(quote! { name = "@a/b", starter = "x" }).unwrap_err();
        assert!(
            error.to_string().contains("unsupported property"),
            "{error}"
        );
    }
}
