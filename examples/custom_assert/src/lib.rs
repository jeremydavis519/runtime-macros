#![feature(proc_macro)]

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro2::Span;
use quote::{Tokens, ToTokens};
use syn::{Expr, Ident};
use syn::buffer::Cursor;
use syn::synom::{PResult, Synom};
use syn::punctuated::Punctuated;
use syn::token::{Bang, Brace, Comma, Paren, Semi};

/// Used exactly like the built-in `assert!` macro.
#[proc_macro]
pub fn custom_assert(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
    custom_assert_internal(ts.into()).into()
}

fn custom_assert_internal(ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let ast: CustomAssert = syn::parse2(ts).unwrap();
    let mut tokens: Tokens = Tokens::new();
    ast.to_tokens(&mut tokens);
    tokens.into()
}

struct CustomAssert {
    expr: Expr,
    message: Punctuated<Expr, Comma>
}

impl Synom for CustomAssert {
    fn parse(mut cursor: Cursor) -> PResult<Self> {
        let expr = Expr::parse(cursor)?; // Required expression
        cursor = expr.1;
        if let Ok((_, mut cursor)) = Comma::parse(cursor) { // Optional message
            let message = Punctuated::parse_separated_nonempty(cursor)?;
            cursor = message.1;
            Ok((CustomAssert { expr: expr.0, message: message.0 }, cursor))
        } else {
            Ok((CustomAssert { expr: expr.0, message: Punctuated::new() }, cursor))
        }
    }
}

impl ToTokens for CustomAssert {
    fn to_tokens(&self, tokens: &mut Tokens) {
        // Equivalent to quote!(if !(#expr) { panic!(#message); }), but avoiding that macro
        // means we can count the lines used.
        let span = Span::call_site();

        Ident::new("if", span).to_tokens(tokens);
        Bang::new(span).to_tokens(tokens);

        Paren(span).surround(tokens, |ref mut expr_tokens| {
            self.expr.to_tokens(expr_tokens);
        });

        Brace(span).surround(tokens, |ref mut block_tokens| {
            Ident::new("panic", span).to_tokens(block_tokens);
            Bang::new(span).to_tokens(block_tokens);

            Paren(span).surround(block_tokens, |ref mut message_tokens| {
                self.message.to_tokens(message_tokens);
            });

            Semi::new(span).to_tokens(block_tokens);
        });
    }
}

#[cfg(test)]
mod tests {
    extern crate runtime_macros;
    use self::runtime_macros::emulate_macro_expansion;
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
        emulate_macro_expansion(file, "custom_assert", custom_assert_internal);
    }
}
