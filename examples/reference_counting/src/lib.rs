extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate syn;

use {
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::ToTokens,
    syn::{
        parse::{self, Parse, ParseStream}, punctuated::Punctuated, token::Pub,
    }
};

/// An attribute that adds a reference count field to the annotated struct or enum. Inserts a
/// compile error if the annotated item is anything else (e.g. a module or a function definition).
/// Also inserts an error if any arguments are provided to the attribute, as none are supported.
///
/// In the case of an enum, the reference count is added to every variant. For tuple structs and
/// tuple enum variants, the reference count is unnamed and is simply appended to the tuple. Any
/// unit variants (variants with no fields) are converted into variants with named fields.
///
/// In structs and variants with named fields, the reference count field is called `reference_count`.
///
/// This function has to be a stub whether proc_macro2 is used or not because Rust complains if we
/// try to use a `#[proc_macro]` function as a regular function outside of a procedural macro
/// context (e.g. in a test). The real logic begins in `reference_counted_internal`.
#[proc_macro_attribute]
pub fn reference_counted(attr: TokenStream, item: TokenStream) -> TokenStream {
    reference_counted_internal(attr.into(), item.into()).into()
}

fn reference_counted_internal(attr: proc_macro2::TokenStream, item: proc_macro2::TokenStream)
        -> proc_macro2::TokenStream {
    if !attr.is_empty() {
        return quote!(::core::compile_error!("`#[reference_counted]` does not accept any arguments."))
    }

    // Parse the annotated item.
    let ast: RefCounted = match syn::parse2(item) {
        Ok(parsed) => parsed,
        Err(_) => return quote!(
            ::core::compile_error!("`#[reference_counted]` must be applied to a struct or an enum.")
        )
    };

    // Return the macro's expanded form (the main logic is in `RefCounted::to_tokens`).
    let mut ts = proc_macro2::TokenStream::new();
    ast.to_tokens(&mut ts);
    ts
}

enum RefCounted {
    Struct(syn::ItemStruct),
    Enum(syn::ItemEnum)
}

impl Parse for RefCounted {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        // Is this a struct?
        if let Ok(item) = input.call(syn::ItemStruct::parse) {
            return Ok(Self::Struct(item));
        }

        // Is this an enum?
        let item = input.call(syn::ItemEnum::parse)?;
        Ok(Self::Enum(item))
    }
}

impl ToTokens for RefCounted {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match *self {
            Self::Struct(ref item) => {
                // Struct: Add a single reference count field.
                let mut item = item.clone();
                apply_refcount(&mut item.fields);
                item.to_tokens(tokens);
            },
            Self::Enum(ref item) => {
                // Enum: Add a reference count field to each variant.
                let mut item = item.clone();
                for variant in item.variants.iter_mut() {
                    apply_refcount(&mut variant.fields);
                }
                item.to_tokens(tokens);
            }
        }
    }
}

// Adds a reference count to the given set of fields.
fn apply_refcount(fields: &mut syn::Fields) {
    match *fields {
        syn::Fields::Named(ref mut fields) => {
            // Named fields: Add a public named reference count.
            fields.named.push(syn::Field {
                attrs: Vec::new(),
                vis: syn::Visibility::Public(Default::default()),
                mutability: syn::FieldMutability::None,
                ident: Some(syn::Ident::new_raw("reference_count", Span::call_site())),
                colon_token: Some(Default::default()),
                ty: syn::Type::Verbatim(quote!(usize)),
            });
        },
        syn::Fields::Unnamed(ref mut fields) => {
            // Unnamed fields: Add a public unnamed reference count to the end.
            fields.unnamed.push(syn::Field {
                attrs: Vec::new(),
                vis: syn::Visibility::Public(Pub::default()),
                mutability: syn::FieldMutability::None,
                ident: None,
                colon_token: None,
                ty: syn::Type::Verbatim(quote!(usize))
            });
        },
        syn::Fields::Unit => {
            // No fields: Convert to named fields with a named reference count.
            let mut named_fields = Punctuated::new();
            named_fields.push(syn::Field {
                attrs: Vec::new(),
                vis: syn::Visibility::Public(Pub::default()),
                mutability: syn::FieldMutability::None,
                ident: Some(syn::Ident::new_raw("reference_count", Span::call_site())),
                colon_token: Some(Default::default()),
                ty: syn::Type::Verbatim(quote!(usize))
            });
            *fields = syn::Fields::Named(
                syn::FieldsNamed {
                    brace_token: Default::default(),
                    named: named_fields
                }
            );
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate runtime_macros;
    use self::runtime_macros::emulate_attributelike_macro_expansion;
    use super::reference_counted_internal;
    use std::{env, fs};

    #[test]
    fn code_coverage() {
        // This code doesn't check much. Instead, it does macro expansion at run time to let
        // tarpaulin measure code coverage for the macro.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("tests.rs");
        let file = fs::File::open(path).unwrap();
        emulate_attributelike_macro_expansion(file, &[("reference_counted", reference_counted_internal)]).unwrap();
    }

    #[test]
    fn given_args() {
        // This code makes sure that the given file doesn't compile.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("compile-fail");
        path.push("given_args.rs");
        let file = fs::File::open(path).unwrap();
        emulate_attributelike_macro_expansion(file, &[("reference_counted", reference_counted_internal)]).unwrap();
    }

    #[test]
    fn annotated_function() {
        // This code makes sure that the given file doesn't compile.
        let mut path = env::current_dir().unwrap();
        path.push("tests");
        path.push("compile-fail");
        path.push("annotated_function.rs");
        let file = fs::File::open(path).unwrap();
            emulate_attributelike_macro_expansion(file, &[("reference_counted", reference_counted_internal)]).unwrap();
    }
}

#[cfg(doctest)]
mod doctests {
    //! Rust doesn't provide a standard way to test for failure to compile, but Rustdoc does. So tests like
    //! that can be put here.
    //!
    //! ```
    //! // Confirm that the file exists.
    //! include_bytes!("../tests/compile-fail/given_args.rs");
    //! ```
    //! ```compile_fail
    //! // Including the file as code is enough to cause a compilation failure.
    //! include!("../tests/compile-fail/given_args.rs");
    //! fn main() {}
    //! ```
    //!
    //! ```
    //! // Confirm that the file exists.
    //! include_bytes!("../tests/compile-fail/annotated_function.rs");
    //! ```
    //! ```compile_fail
    //! // Including the file as code is enough to cause a compilation failure.
    //! include!("../tests/compile-fail/annotated_function.rs");
    //! fn main() {}
    //! ```
}
