extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::{path::PathBuf, sync::OnceLock};
use syn::{parse_macro_input, LitStr};

fn resolve_assets_root() -> PathBuf {
    static ASSETS_ROOT: OnceLock<PathBuf> = OnceLock::new();

    ASSETS_ROOT
        .get_or_init(|| {
            let mut dir = std::env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    std::env::current_dir().expect("unable to determine current dir")
                });

            loop {
                let candidate = dir.join("assets");
                if candidate.exists() {
                    return candidate;
                }

                if !dir.pop() {
                    panic!(
                        "Unable to locate an `assets` directory relative to `{}`",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| dir.display().to_string())
                    );
                }
            }
        })
        .clone()
}

fn check_assets_path_exists(path: &str) -> bool {
    let mut path_buf = resolve_assets_root();
    path_buf.push(path);
    path_buf.exists()
}

// TODO should check for a file, and reject folders. Could add a different function for folders
#[proc_macro]
pub fn assert_assets_path(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let path_str = path_lit.value();

    if !check_assets_path_exists(&path_str) {
        panic!("File does not exist: {}", path_str);
    }

    let expanded = quote! {
        #path_str
    };

    TokenStream::from(expanded)
}
