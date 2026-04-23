mod macro_impl;

use proc_macro::TokenStream;

#[proc_macro]
pub fn compile_exports_from(stream: TokenStream) -> TokenStream {
    macro_impl::compile_exports_from(stream.into()).into()
}
