extern crate proc_macro;

use quote::quote;
use syn::parse_macro_input;


#[proc_macro_derive(HelloWorld)]
pub fn derive_hello_world(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_hello_world_internal(parse_macro_input!(input as syn::DeriveInput)).into()
}

fn derive_hello_world_internal(input: syn::DeriveInput) -> proc_macro2::TokenStream {
    let ident = input.ident;
    quote! {
        impl #ident {
            fn hello_world() -> String {
                String::from("Hello World")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use runtime_macros::emulate_derive_expansion_fallible;
    use super::derive_hello_world_internal;
    use std::{env, fs};

    #[test]
    fn derive_code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("tests.rs");
        let file = fs::File::open(path).unwrap();
        emulate_derive_expansion_fallible(file, "HelloWorld", derive_hello_world_internal).unwrap();
    }

    #[test]
    fn syntax_error() {
        // This code makes sure that the given file doesn't compile.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("compile-fail");
        path.push("invalid_derive.rs");
        let file = fs::File::open(path).unwrap();
        assert!(emulate_derive_expansion_fallible(file, "HelloWorld", derive_hello_world_internal).is_err());
    }
}
