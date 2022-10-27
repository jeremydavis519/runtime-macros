extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate syn;

use {
    proc_macro::TokenStream,
    quote::ToTokens,
    syn::{
        ItemStruct,
        parse::{self, Parse, ParseStream}
    }
};

/// A derive macro that implements `From<T>` for `[u8; N]` and `From<[u8; N]>` for `T`, where `T` is
/// the type the derive attribute is annotating and `N` is the size of `T`, in bytes. The idea is
/// that such a type is "plain old data", which can be directly converted to and from arrays of
/// bytes because it has no invalid states.
///
/// (Please don't use this in production. There are much better implementations based on traits.
/// This is just an example of how to measure a derive macro's test coverage.)
///
/// This macro is only implemented for structs. Trying to use it on anything else (e.g. an enum)
/// will result in a compilation error.
///
/// This function has to be a stub whether proc_macro2 is used or not because Rust complains if we
/// try to use a `#[proc_macro]` function as a regular function outside of a procedural macro
/// context (e.g. in a test). The real logic begins in `pod_internal`.
#[proc_macro_derive(Pod)]
pub fn pod(item: TokenStream) -> TokenStream {
    pod_internal(item.into()).into()
}

fn pod_internal(item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // Parse the annotated item.
    let ast: Pod = match syn::parse2(item) {
        Ok(parsed) => parsed,
        Err(e) => return e.into_compile_error()
    };

    // Return the macro's expanded form (the main logic is in `Pod::to_tokens`).
    let mut ts = proc_macro2::TokenStream::new();
    ast.to_tokens(&mut ts);
    ts
}

struct Pod {
    item: ItemStruct
}

impl Parse for Pod {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(Self { item: input.call(ItemStruct::parse)? })
    }
}

impl ToTokens for Pod {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = self.item.ident.clone();
        tokens.extend(quote!(
            const N: usize = ::std::mem::size_of::<#ident>();
            impl From<#ident> for [u8; N] {
                fn from(value: #ident) -> Self {
                    unsafe { ::std::mem::transmute(value) }
                }
            }
            impl From<[u8; N]> for #ident {
                fn from(value: [u8; N]) -> Self {
                    unsafe { ::std::mem::transmute(value) }
                }
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    extern crate runtime_macros;
    use self::runtime_macros::emulate_derive_macro_expansion;
    use super::pod_internal;
    use std::{env, fs};

    #[test]
    fn code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("tests.rs");
        let file = fs::File::open(path).unwrap();
        emulate_derive_macro_expansion(file, &[("Pod", pod_internal)]).unwrap();
    }
}

// No tests of invalid uses of the macro (such as applying it to an enum)! Those paths won't be covered.
