extern crate proc_macro;

use cached::proc_macro::cached;
use proc_macro::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::{parse_macro_input, LitStr};

#[cached]
fn check_assets_path_exists(path: String) -> bool {
    let mut path_buf = PathBuf::from("assets");
    path_buf.push(path);
    path_buf.exists()
}

#[proc_macro]
pub fn assert_assets_path(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path_str = path_lit.value();

    if !check_assets_path_exists(path_str.clone()) {
        panic!("File does not exist: {}", path_str.clone());
    }

    let expanded = quote! {
        #path_str
    };

    TokenStream::from(expanded)
}
