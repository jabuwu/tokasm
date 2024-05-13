mod main_fn;

#[proc_macro_attribute]
pub fn main(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match main_fn::proc_macro(attr.into(), item.into()) {
        Ok(token) => token.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
