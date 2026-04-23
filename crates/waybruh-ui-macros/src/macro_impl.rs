use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use slint_interpreter::Compiler;
use spin_on::spin_on;
use std::{os::unix::ffi::OsStrExt, path::PathBuf};
use syn::{
    LitStr, Result as ParseResult, Token, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Bracket,
};

pub struct LitStrArray {
    pub source_span: Span,
    pub string_literals: Punctuated<LitStr, Token![,]>,
    #[allow(unused)]
    pub bracket: Bracket,
}

impl Parse for LitStrArray {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let source_span = input.span();
        let literals;

        Ok(Self {
            bracket: bracketed!(literals in input),
            string_literals: literals.parse_terminated(<LitStr as Parse>::parse, Token![,])?,
            source_span,
        })
    }
}

pub fn compile_exports_from(stream: TokenStream2) -> TokenStream2 {
    let path_array = syn::parse2::<LitStrArray>(stream).unwrap();
    let mut exports = String::new();

    for path in &path_array.string_literals {
        add_exports_from(&mut exports, path.value());
    }

    let result = LitStr::new(&exports, path_array.source_span);

    quote! { #result }
}

fn add_exports_from(exports: &mut String, path: String) {
    let compiler = Compiler::new();
    let path = PathBuf::from(path);
    let file_name = String::from_utf8_lossy(path.file_name().unwrap().as_bytes()).into_owned();

    let mut source = std::fs::read_to_string(&path).unwrap();
    source.push_str("\nexport component _Entry inherits Window {}");

    let result = spin_on(compiler.build_from_source(source, path));
    let component = result.component("_Entry").unwrap();

    exports.push_str("\nexport { ");

    let components = result
        .component_names()
        .filter(|&c| c != "_Entry")
        .map(str::to_owned);

    let items = component
        .globals()
        .chain(component.functions())
        .chain(components)
        .collect::<Vec<_>>();

    if let Some(first) = items.first() {
        exports.push_str(first);
    }

    for item in items.get(1..).unwrap_or_default() {
        exports.push_str(", ");
        exports.push_str(item);
    }

    exports.push_str(&format!(r#" }} from "waybruh/{file_name}";"#));
}
