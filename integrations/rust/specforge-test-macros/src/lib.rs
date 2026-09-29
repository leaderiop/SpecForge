//! `#[specforge_test(behavior = "id", verify = "...")]` — links a test to
//! the spec entity (and optionally the `verify` obligation) it proves, and
//! registers it as a test (ADR 0002): no separate `#[test]` is needed.
//!
//! The attribute defers registration when another attribute on the same
//! function registers the test: a stacked specforge attribute below it, or
//! a runner attribute such as `#[tokio::test]`. A plain `#[test]` below it
//! is a compile error, because it would register the test twice.

use proc_macro::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, ItemFn, LitStr, Token};

struct TestAttr {
    entity_kind: String,
    entity_id: String,
    verify: Option<String>,
    /// Record the guard without registering a test: for functions called
    /// directly, like the macro's own self-tests.
    guard_only: bool,
}

impl Parse for TestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Entity kinds can be Rust keywords (`type`), so accept any ident.
        let kind_ident = Ident::parse_any(input)?;
        let _eq: Token![=] = input.parse()?;
        let id_lit: LitStr = input.parse()?;

        let entity_kind = kind_ident.unraw().to_string();

        let mut verify = None;
        let mut guard_only = false;

        while !input.is_empty() {
            let _comma: Token![,] = input.parse()?;
            if input.is_empty() {
                break;
            }
            let key: Ident = input.parse()?;
            if key == "__guard_only" {
                guard_only = true;
                continue;
            }
            if key != "verify" {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown argument `{key}`: expected `verify = \"...\"`"),
                ));
            }
            let _eq: Token![=] = input.parse()?;
            let val: LitStr = input.parse()?;
            verify = Some(val.value());
        }

        Ok(TestAttr {
            entity_kind,
            entity_id: id_lit.value(),
            verify,
            guard_only,
        })
    }
}

/// How a remaining attribute on the function relates to test registration.
enum Registration {
    /// Another specforge attribute or a runner attribute registers it.
    Deferred,
    /// A plain `#[test]` below this attribute would register it twice.
    PlainTest,
    Unrelated,
}

fn registration(attr: &Attribute) -> Registration {
    let segments: Vec<String> = attr
        .path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    let names: Vec<&str> = segments.iter().map(String::as_str).collect();
    match names.as_slice() {
        ["test"] => Registration::PlainTest,
        // The names this attribute goes by in practice.
        ["spec"] | ["specforge_test"] => Registration::Deferred,
        [.., "specforge_test_macros" | "specforge_test" | "specforge", "test"] => {
            Registration::Deferred
        }
        // Runner attributes that register the test themselves
        // (`tokio::test`, `async_std::test`, `rstest`, `test_case`, ...).
        [.., "test"] | [.., "rstest"] | [.., "test_case"] => Registration::Deferred,
        _ => Registration::Unrelated,
    }
}

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as TestAttr);
    let input_fn = syn::parse_macro_input!(item as ItemFn);

    let mut deferred = args.guard_only;
    for attr in &input_fn.attrs {
        match registration(attr) {
            Registration::PlainTest => {
                return syn::Error::new_spanned(
                    attr,
                    "remove this #[test]: #[specforge_test] registers the test itself",
                )
                .to_compile_error()
                .into();
            }
            Registration::Deferred => deferred = true,
            Registration::Unrelated => {}
        }
    }
    let register = if deferred {
        quote! {}
    } else {
        quote! { #[::core::prelude::v1::test] }
    };

    let entity_kind = &args.entity_kind;
    let entity_id = &args.entity_id;
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    let verify_expr = match &args.verify {
        Some(v) => quote! { Some(#v) },
        None => quote! { None },
    };
    let once = if args.guard_only {
        quote! {}
    } else {
        quote! {
            ::specforge_test::__private::assert_registered_once(
                module_path!(),
                #fn_name_str,
                #entity_id,
                #verify_expr,
            );
        }
    };

    // C11-04: #[should_panic] makes a panic the success path. #[ignore]
    // stays on the generated test, so libtest reports it as ignored, and
    // `cargo test -- --ignored` runs the real body and records its result.
    let expect_panic = input_fn
        .attrs
        .iter()
        .any(|a| a.path().is_ident("should_panic"));

    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    // Splice the body's statements rather than nesting its block, so a
    // single-expression body doesn't trip `unused_braces` in user code.
    let body = &input_fn.block.stmts;

    let output = quote! {
        #register
        #(#attrs)*
        #vis #sig {
            #once
            let __specforge_guard = ::specforge_test::__private::TestGuard::with_expectations(
                #entity_kind,
                #entity_id,
                module_path!(),
                #fn_name_str,
                file!(),
                line!(),
                #verify_expr,
                #expect_panic,
            );
            #(#body)*
        }
    };

    output.into()
}
