extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use {
    proc_macro::TokenStream,
    quote::{ToTokens, TokenStreamExt},
    syn::{
        Expr,
        parse::{self, Parse, ParseStream},
        punctuated::Punctuated
    }
};

/// Used exactly like the built-in `assert!` macro. This function has to be a stub whether
/// proc_macro2 is used or not because Rust complains if we try to use a `#[proc_macro]` function
/// as a regular function outside of a procedural macro context (e.g. in a test). The real logic
/// begins in `custom_assert_internal`.
#[proc_macro]
pub fn custom_assert(ts: TokenStream) -> TokenStream {
    custom_assert_internal(ts.into()).into()
}

fn custom_assert_internal(ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // Parse the arguments to the macro.
    let ast: CustomAssert = syn::parse2(ts).unwrap();

    // Return the macro's expanded form (the main logic is in `CustomAssert::to_tokens`).
    let mut ts = proc_macro2::TokenStream::new();
    ast.to_tokens(&mut ts);
    ts
}

struct CustomAssert {
    expr: Expr,
    message: Punctuated<Expr, Token![,]>
}

impl Parse for CustomAssert {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let expr = input.call(Expr::parse)?; // Required expression
        if input.parse::<Token![,]>().is_ok() { // Optional message
            let message = input.call(Punctuated::parse_separated_nonempty)?;
            Ok(CustomAssert { expr, message })
        } else {
            Ok(CustomAssert { expr, message: Punctuated::new() })
        }
    }
}

impl ToTokens for CustomAssert {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.expr;
        let message = &self.message;
        tokens.append_all(quote!(if !(#expr) { panic!(#message); }));
    }
}

#[cfg(test)]
mod tests {
    extern crate runtime_macros;
    use self::runtime_macros::emulate_functionlike_macro_expansion;
    use super::custom_assert_internal;
    use std::{env, fs};

    #[test]
    fn code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("tests.rs");
        let file = fs::File::open(path).unwrap();
        emulate_functionlike_macro_expansion(file, &[("custom_assert", custom_assert_internal)]).unwrap();
    }

    #[test]
    fn syntax_error() {
        // This code makes sure that the given file doesn't compile.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("compile-fail");
        path.push("syntax_error.rs");
        let file = fs::File::open(path).unwrap();
        assert!(emulate_functionlike_macro_expansion(file, &[("custom_assert", custom_assert_internal)]).is_err());
    }
}
