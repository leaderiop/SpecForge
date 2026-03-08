use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitStr, Token};

struct TestAttr {
    entity_kind: String,
    entity_id: String,
}

impl Parse for TestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind_ident: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let id_lit: LitStr = input.parse()?;

        let entity_kind = kind_ident.to_string();
        // Accept "behavior", "invariant", "event", and "verify" (which is skipped)
        if !["behavior", "invariant", "event", "verify"].contains(&entity_kind.as_str()) {
            return Err(syn::Error::new(
                kind_ident.span(),
                format!("expected `behavior`, `invariant`, or `event`, found `{entity_kind}`"),
            ));
        }

        // Skip the "verify" key — it's metadata, not a guard
        // Also skip any trailing comma + additional args
        while !input.is_empty() {
            let _comma: Token![,] = input.parse()?;
            let _key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;
            let _val: LitStr = input.parse()?;
        }

        Ok(TestAttr {
            entity_kind,
            entity_id: id_lit.value(),
        })
    }
}

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as TestAttr);
    let input_fn = syn::parse_macro_input!(item as ItemFn);

    // "verify" attribute is metadata-only — no guard injected
    if args.entity_kind == "verify" {
        return quote! { #input_fn }.into();
    }

    let entity_kind = &args.entity_kind;
    let entity_id = &args.entity_id;
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let body = &input_fn.block;

    let output = quote! {
        #(#attrs)*
        #vis #sig {
            let __specforge_guard = ::specforge_test::__private::TestGuard::new(
                #entity_kind,
                #entity_id,
                module_path!(),
                #fn_name_str,
                file!(),
                line!(),
            );
            #body
        }
    };

    output.into()
}
