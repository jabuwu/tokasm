use proc_macro2::TokenStream;
use syn::{braced, parenthesized, parse::Parse, parse2, token, Error, Ident};
use quote::quote;

#[allow(dead_code)]
pub struct AsyncFnMain {
    async_token: token::Async,
    fn_token: token::Fn,
    main: Ident,
    paren_token: token::Paren,
    brace_token: token::Brace,
    body: TokenStream,
}

impl Parse for AsyncFnMain {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _paren_content;
        let body_content;
        Ok(Self {
            async_token: input.parse()?,
            fn_token: input.parse()?,
            main: input.parse()?,
            paren_token: parenthesized!(_paren_content in input),
            brace_token: braced!(body_content in input),
            body: body_content.parse()?,
        })
    }
}

pub fn proc_macro(_attr: TokenStream, item: TokenStream) -> Result<TokenStream, Error> {
    let AsyncFnMain { body, .. } = parse2(item)?;
    Ok(quote! {
        fn main() {
            let body = async {
                #body
            };
            tokasm::spawn(body);
            #[cfg(not(target_arch = "wasm32"))]
            tokasm::wait_until_finished();
        }
    })
}
