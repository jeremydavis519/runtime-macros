// Copyright (c) 2018 Jeremy Davis (jeremydavis519@gmail.com)
// 
// Licensed under the Apache License, Version 2.0 (located at /LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0), or the MIT license
// (located at /LICENSE-MIT or http://opensource.org/licenses/MIT), at your
// option. The file may not be copied, modified, or distributed except
// according to those terms.
// 
// Unless required by applicable law or agreed to in writing, this software
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF
// ANY KIND, either express or implied. See the applicable license for the
// specific language governing permissions and limitations under that license.

//! This crate offers a way to emulate the process of procedural macro expansion at run time.
//! It is intended for use with code coverage tools like [`tarpaulin`], which can't measure
//! the code coverage of anything that happens at compile time.
//! 
//! Currently, `runtime-macros` only works with `functionlike!` procedural macros. Custom
//! derive may be supported in the future if there's demand.
//! 
//! [`tarpaulin`]: https://crates.io/crates/cargo-tarpaulin
//! 
//! To use it, add a test case to your procedural macro crate that calls `emulate_macro_expansion`
//! on a `.rs` file that calls the macro. Most likely, all the files you'll want to use it on will
//! be in your `/tests` directory. Once you've completed this step, any code coverage tool that
//! works with your crate's test cases will be able to report on how thoroughly you've tested the
//! macro.
//! 
//! See the `/examples` directory in the [repository] for working examples.
//! 
//! [repository]: https://github.com/jeremydavis519/runtime-macros

use std::fs;
use std::io::Read;
use std::panic::{self, AssertUnwindSafe};

/// Parses the given Rust source file, finding functionlike macro expansions using `macro_path`.
/// Each time it finds one, it calls `proc_macro_fn`, passing it the inner `TokenStream` just as
/// if the macro were being expanded. The only effect is to verify that the macro doesn't panic,
/// as the expansion is not actually applied to the AST or the source code.
/// 
/// Note that this parser only handles Rust's syntax, so it cannot resolve paths to see if they
/// are equivalent to the given one. The paths used to reference the macro must be exactly equal
/// to the one given in order to be expanded by this function. For example, if `macro_path` is
/// `"foo"` and the file provided calls the macro using `bar::foo!`, this function will not know
/// to expand it, and the macro's code coverage will be underestimated.
/// 
/// Also, this function uses `proc_macro2::TokenStream`, not the standard but partly unstable
/// `proc_macro::TokenStream`. You can convert between them using their `into` methods, as shown
/// below.
///
/// # Returns
///
/// `Ok` on success, or an instance of [`Error`] indicating any error that occurred when trying to
/// read or parse the file.
///
/// [`Error`]: enum.Error.html
/// 
/// # Example
/// 
/// ```ignore
/// # // This example doesn't compile because procedural macros can only be made in crates with
/// # // type "proc-macro".
/// # #![cfg(feature = "proc-macro")]
/// # extern crate proc_macro;
/// # extern crate proc_macro2;
/// #[proc_macro]
/// fn remove(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
///     // This macro just eats its input and replaces it with nothing.
///     proc_macro::TokenStream::empty()
/// }
/// 
/// extern crate syn;
/// 
/// #[test]
/// fn macro_code_coverage() {
///     let file = std::fs::File::open("tests/tests.rs");
///     emulate_macro_expansion(file, "remove", |ts| remove(ts.into()).into());
/// }
/// ```
pub fn emulate_macro_expansion_fallible<F>(mut file: fs::File, macro_path: &str, proc_macro_fn: F)
        -> Result<(), Error>
        where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    struct MacroVisitor<F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream> {
        macro_path: syn::Path,
        proc_macro_fn: AssertUnwindSafe<F>
    }
    impl<'ast, F> syn::visit::Visit<'ast> for MacroVisitor<F>
            where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        fn visit_macro(&mut self, macro_item: &'ast syn::Macro) {
            if macro_item.path == self.macro_path {
                (*self.proc_macro_fn)(macro_item.tokens.clone());
            }
        }
    }
    
    let proc_macro_fn = AssertUnwindSafe(proc_macro_fn);

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| Error::IoError(e))?;
    
    let ast = AssertUnwindSafe(syn::parse_file(content.as_str()).map_err(|e| Error::ParseError(e))?);
    let macro_path: syn::Path = syn::parse_str(macro_path).map_err(|e| Error::ParseError(e))?;

    panic::catch_unwind(|| {
        syn::visit::visit_file(&mut MacroVisitor::<F> {
            macro_path,
            proc_macro_fn
        }, &*ast);
    }).map_err(|_| Error::ParseError(syn::parse::Error::new(proc_macro2::Span::call_site(), "macro expansion panicked")))?;

    Ok(())
}

fn uses_derive(attrs: &[syn::Attribute], derive_name: &syn::Path) -> Result<bool, Error> {
    for attr in attrs {
        if attr.path.is_ident("derive") {
            let meta = attr.parse_meta().map_err(|e| Error::ParseError(e))?;
            if let syn::Meta::List(ml) = meta {
                let uses_derive = ml.nested.iter().any(|nested_meta| {
                    *nested_meta == syn::NestedMeta::Meta(syn::Meta::Path(derive_name.clone()))
                });
                if uses_derive {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}


/// Parses the given Rust source file, finding custom drives macro expansions using `macro_path`.
/// Each time it finds one, it calls `derive_fn`, passing it a `syn::DeriveInput`. 
/// 
/// Note that this parser only handles Rust's syntax, so it cannot resolve paths to see if they
/// are equivalent to the given one. The paths used to reference the macro must be exactly equal
/// to the one given in order to be expanded by this function. For example, if `macro_path` is
/// `"foo"` and the file provided calls the macro using `bar::foo!`, this function will not know
/// to expand it, and the macro's code coverage will be underestimated.
/// 
/// This function follows the standard syn pattern of implementing most of the logic using the
/// `proc_macro2` types, leaving only those methods that can only exist for `proc_macro=true`
/// crates, such as types from `proc_macro` or `syn::parse_macro_input` in the outer function.
/// This allows use of the inner function in tests which is needed to expand it here.
///
/// # Returns
///
/// `Ok` on success, or an instance of [`Error`] indicating any error that occurred when trying to
/// read or parse the file.
///
/// [`Error`]: enum.Error.html
/// 
/// # Example
/// 
/// ```ignore
/// # // This example doesn't compile because procedural macros can only be made in crates with
/// # // type "proc-macro".
/// # #![cfg(feature = "proc-macro")]
/// # extern crate proc_macro;
/// 
/// use quote::quote;
/// use syn::parse_macro_input;
/// 
/// #[proc_macro_derive(Hello)]
/// fn hello(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
///     hello_internal(parse_macro_input!(input as DeriveInput)).into()
/// }
/// 
/// fn hello_internal(input: syn::DeriveInput) -> proc_macro2::TokenStream {
///     let ident = input.ident;
///     quote! {
///         impl #ident {
///             fn hello_world() -> String {
///                 String::from("Hello World")
///             }
///         }
///     }
/// }
/// 
/// #[test]
/// fn macro_code_coverage() {
///     let file = std::fs::File::open("tests/tests.rs");
///     emulate_derive_expansion_fallible(file, "Hello", hello_internal);
/// }
/// ```
pub fn emulate_derive_expansion_fallible<F>(mut file: fs::File, macro_path: &str, derive_fn: F)
        -> Result<(), Error>
        where F: Fn(syn::DeriveInput) -> proc_macro2::TokenStream {
    struct MacroVisitor<F: Fn(syn::DeriveInput) -> proc_macro2::TokenStream> {
        macro_path: syn::Path,
        derive_fn: AssertUnwindSafe<F>
    }
    impl<'ast, F> syn::visit::Visit<'ast> for MacroVisitor<F>
            where F: Fn(syn::DeriveInput) -> proc_macro2::TokenStream {
        fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
            match uses_derive(&node.attrs, &self.macro_path) {
                Ok(uses) => {
                    if uses {
                        (*self.derive_fn)(node.clone().into());
                    }
                },
                Err(e) => panic!("Failed expanding derive macro for {:?}: {}", self.macro_path, e),
            }
        }
        
        fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
            match uses_derive(&node.attrs, &self.macro_path) {
                Ok(uses) => {
                    if uses {
                        (*self.derive_fn)(node.clone().into());
                    }
                },
                Err(e) => panic!("Failed expanding derive macro for {:?}: {}", self.macro_path, e),
            }
        }
    }
    
    let derive_fn = AssertUnwindSafe(derive_fn);

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| Error::IoError(e))?;
    
    let ast = AssertUnwindSafe(syn::parse_file(content.as_str()).map_err(|e| Error::ParseError(e))?);
    let macro_path: syn::Path = syn::parse_str(macro_path).map_err(|e| Error::ParseError(e))?;

    panic::catch_unwind(|| {
        syn::visit::visit_file(&mut MacroVisitor::<F> {
            macro_path,
            derive_fn
        }, &*ast);
    }).map_err(|_| Error::ParseError(syn::parse::Error::new(proc_macro2::Span::call_site(), "macro expansion panicked")))?;

    Ok(())
}

/// This type is like [`emulate_macro_expansion_fallible`] but automatically unwraps any errors it
/// encounters. As such, it's deprecated due to being less flexible.
///
/// [`emulate_macro_expansion_fallible`]: fn.emulate_macro_expansion_fallible.html
#[deprecated]
pub fn emulate_macro_expansion<F>(file: fs::File, macro_path: &str, proc_macro_fn: F)
        where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    emulate_macro_expansion_fallible(file, macro_path, proc_macro_fn).unwrap()
}

/// The error type for [`emulate_macro_expansion_fallible`]. If anything goes wrong during the file
/// loading or macro expansion, this type describes it.
///
/// [`emulate_macro_expansion_fallible`]: fn.emulate_macro_expansion_fallible.html
#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ParseError(syn::parse::Error)
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::IoError(e) => e.fmt(f),
            Error::ParseError(e) => e.fmt(f)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error+'static)> {
        match self {
            Error::IoError(e) => e.source(),
            Error::ParseError(e) => e.source()
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate cargo_tarpaulin;
    use self::cargo_tarpaulin::launch_tarpaulin;
    use self::cargo_tarpaulin::config::Config;
    use std::{env, time};

    #[test]
    fn proc_macro_coverage() {
        let mut config = Config::default();
        let test_dir = env::current_dir().unwrap().join("examples").join("custom_assert");
        config.manifest = test_dir.join("Cargo.toml");
        config.test_timeout = time::Duration::from_secs(60);
        let (_trace_map, return_code) = launch_tarpaulin(&config).unwrap();
        assert_eq!(return_code, 0);
    }
}
