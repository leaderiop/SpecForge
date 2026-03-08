use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitStr, Token};

struct TestAttr {
    entity_kind: String,
    entity_id: String,
    verify: Option<String>,
}

impl Parse for TestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind_ident: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let id_lit: LitStr = input.parse()?;

        let entity_kind = kind_ident.to_string();
        if !["behavior", "invariant", "event"].contains(&entity_kind.as_str()) {
            return Err(syn::Error::new(
                kind_ident.span(),
                format!("expected `behavior`, `invariant`, or `event`, found `{entity_kind}`"),
            ));
        }

        let mut verify = None;

        while !input.is_empty() {
            let _comma: Token![,] = input.parse()?;
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;
            let val: LitStr = input.parse()?;

            if key == "verify" {
                verify = Some(val.value());
            }
        }

        Ok(TestAttr {
            entity_kind,
            entity_id: id_lit.value(),
            verify,
        })
    }
}

#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as TestAttr);
    let input_fn = syn::parse_macro_input!(item as ItemFn);

    let entity_kind = &args.entity_kind;
    let entity_id = &args.entity_id;
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    let verify_expr = match &args.verify {
        Some(v) => quote! { Some(#v) },
        None => quote! { None },
    };

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
                #verify_expr,
            );
            #body
        }
    };

    output.into()
}
