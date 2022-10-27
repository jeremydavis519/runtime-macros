// Copyright (c) 2018-2022 Jeremy Davis (jeremydavis519@gmail.com)
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

extern crate proc_macro;
extern crate quote;
extern crate syn;

use {
    std::{
        fs,
        io::Read,
        panic::{self, AssertUnwindSafe}
    },
    quote::ToTokens,
    syn::{Meta, NestedMeta}
};

/// Searches the given Rust source code file for function-like macro calls and calls the functions
/// that define how to expand them.
///
/// Each time it finds one, this function calls the corresponding procedural macro function, passing
/// it the inner `TokenStream` just as if the macro were being expanded. The only effect is to
/// verify that the macro doesn't panic, as the expansion is not actually applied to the AST or the
/// source code.
///
/// Note that this parser only handles Rust's syntax, so it cannot resolve paths to see if they
/// are equivalent to the given one. The paths used to reference the macro must be exactly equal
/// to the one given in order to be expanded by this function. For example, if `macro_path` is
/// `"foo"` and the file provided calls the macro using `bar::foo!`, this function will not know
/// to expand it, and the macro's code coverage will be underestimated.
///
/// Also, this function uses `proc_macro2::TokenStream`, not the standard `proc_macro::TokenStream`.
/// The Rust compiler disallows using the `proc_macro` API for anything except defining a procedural
/// macro (i.e. we can't use it at runtime). You can convert between the two types using their
/// `into` methods, as shown below.
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
/// ```
/// # use runtime_macros::emulate_functionlike_macro_expansion;
///
/// # /*
/// #[proc_macro]
/// fn remove(ts: proc_macro::TokenStream) -> proc_macro::TokenStream {
///     // This stub just allows us to use `proc_macro2` instead of `proc_macro`.
///     remove_internal(ts.into()).into()
/// }
/// # */
///
/// fn remove_internal(_: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
///     // This macro just eats its input and replaces it with nothing.
///     proc_macro2::TokenStream::new()
/// }
///
/// # /*
/// #[test]
/// # */
/// fn macro_code_coverage() {
/// # /*
///     let file = std::fs::File::open("tests/tests.rs").unwrap();
/// # */
/// # let file = std::fs::File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
///     emulate_functionlike_macro_expansion(file, &[("remove", remove_internal)]).unwrap();
/// }
/// # macro_code_coverage();
/// ```
pub fn emulate_functionlike_macro_expansion<'a, F>(
        mut file: fs::File,
        macro_paths_and_proc_macro_fns: &[(&'a str, F)]
) -> Result<(), Error>
        where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    struct MacroVisitor<'a, F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream> {
        macro_paths_and_proc_macro_fns: AssertUnwindSafe<Vec<(syn::Path, &'a F)>>
    }
    impl<'a, 'ast, F> syn::visit::Visit<'ast> for MacroVisitor<'a, F>
            where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        fn visit_macro(&mut self, macro_item: &'ast syn::Macro) {
            for (path, proc_macro_fn) in self.macro_paths_and_proc_macro_fns.iter() {
                if macro_item.path == *path {
                    proc_macro_fn(macro_item.tokens.clone().into());
                }
            }
        }
    }

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| Error::IoError(e))?;

    let ast = AssertUnwindSafe(syn::parse_file(content.as_str()).map_err(|e| Error::ParseError(e))?);
    let macro_paths_and_proc_macro_fns = AssertUnwindSafe(
        macro_paths_and_proc_macro_fns.iter()
            .map(|(s, f)| Ok((syn::parse_str(s)?, f)))
            .collect::<Result<Vec<(syn::Path, &F)>, _>>()
            .map_err(|e| Error::ParseError(e))?
    );

    panic::catch_unwind(|| {
        syn::visit::visit_file(&mut MacroVisitor::<F> {
            macro_paths_and_proc_macro_fns
        }, &*ast);
    }).map_err(|_| Error::ParseError(syn::parse::Error::new(
        proc_macro2::Span::call_site().into(), "macro expansion panicked"
    )))?;

    Ok(())
}

/// Searches the given Rust source code file for derive macro calls and calls the functions that
/// define how to expand them.
///
/// This function behaves just like [`emulate_functionlike_macro_expansion`], but with derive macros
/// like `#[derive(Foo)]` instead of function-like macros like `foo!()`. See that function's
/// documentation for details and an example of use.
///
/// [`emulate_functionlike_macro_expansion`]: fn.emulate_functionlike_macro_expansion.html
pub fn emulate_derive_macro_expansion<'a, F>(
        mut file: fs::File,
        macro_paths_and_proc_macro_fns: &[(&'a str, F)]
) -> Result<(), Error>
        where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    struct MacroVisitor<'a, F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream> {
        macro_paths_and_proc_macro_fns: AssertUnwindSafe<Vec<(syn::Path, &'a F)>>
    }
    impl<'a, 'ast, F> syn::visit::Visit<'ast> for MacroVisitor<'a, F>
            where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        fn visit_item(&mut self, item: &'ast syn::Item) {
            macro_rules! visit {
                ( $($ident:ident),* ) => {
                    match *item {
                        $(syn::Item::$ident(ref item) => {
                            for attr in item.attrs.iter() {
                                let meta = match attr.parse_meta() {
                                    Ok(Meta::List(list)) => list,
                                    _ => continue
                                };
                                let path_ident = match meta.path.get_ident() {
                                    Some(x) => x,
                                    None => continue
                                };
                                if path_ident.to_string() != "derive" {
                                    continue;
                                }
                                for nested_meta in meta.nested.iter() {
                                    let meta_path = match *nested_meta {
                                        NestedMeta::Meta(Meta::Path(ref path)) => path,
                                        _ => continue
                                    };
                                    for (path, proc_macro_fn) in self.macro_paths_and_proc_macro_fns.iter() {
                                        if meta_path == path {
                                            proc_macro_fn(/* attributes? */ item.to_token_stream());
                                        }
                                    }
                                }
                            }
                        },)*
                        _ => {}
                    }
                }
            }
            visit!(
                Const,
                Enum,
                ExternCrate,
                Fn,
                ForeignMod,
                Impl,
                Macro,
                Macro2,
                Mod,
                Static,
                Struct,
                Trait,
                TraitAlias,
                Type,
                Union,
                Use
            );
        }
    }

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| Error::IoError(e))?;

    let ast = AssertUnwindSafe(syn::parse_file(content.as_str()).map_err(|e| Error::ParseError(e))?);
    let macro_paths_and_proc_macro_fns = AssertUnwindSafe(
        macro_paths_and_proc_macro_fns.iter()
            .map(|(s, f)| Ok((syn::parse_str(s)?, f)))
            .collect::<Result<Vec<(syn::Path, &F)>, _>>()
            .map_err(|e| Error::ParseError(e))?
    );

    panic::catch_unwind(|| {
        syn::visit::visit_file(&mut MacroVisitor::<F> {
            macro_paths_and_proc_macro_fns
        }, &*ast);
    }).map_err(|_| Error::ParseError(syn::parse::Error::new(
        proc_macro2::Span::call_site().into(), "macro expansion panicked"
    )))?;

    Ok(())
}

/// Searches the given Rust source code file for attribute-like macro calls and calls the functions
/// that define how to expand them.
///
/// This function behaves just like [`emulate_functionlike_macro_expansion`], but with attribute-like
/// macros like `#[foo]` instead of function-like macros like `foo!()`. See that function's
/// documentation for details and an example of use.
///
/// [`emulate_functionlike_macro_expansion`]: fn.emulate_functionlike_macro_expansion.html
pub fn emulate_attributelike_macro_expansion<'a, F>(
        mut file: fs::File,
        macro_paths_and_proc_macro_fns: &[(&'a str, F)]
) -> Result<(), Error>
        where F: Fn(proc_macro2::TokenStream, proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    struct MacroVisitor<'a, F: Fn(proc_macro2::TokenStream, proc_macro2::TokenStream) -> proc_macro2::TokenStream> {
        macro_paths_and_proc_macro_fns: AssertUnwindSafe<Vec<(syn::Path, &'a F)>>
    }
    impl<'a, 'ast, F> syn::visit::Visit<'ast> for MacroVisitor<'a, F>
            where F: Fn(proc_macro2::TokenStream, proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        fn visit_item(&mut self, item: &'ast syn::Item) {
            macro_rules! visit {
                ( $($ident:ident),* ) => {
                    match *item {
                        $(syn::Item::$ident(ref item) => {
                            for attr in item.attrs.iter() {
                                for (path, proc_macro_fn) in self.macro_paths_and_proc_macro_fns.iter() {
                                    if attr.path == *path {
                                        proc_macro_fn(attr.tokens.clone().into(), item.to_token_stream());
                                    }
                                }
                            }
                        },)*
                        _ => {}
                    }
                }
            }
            visit!(
                Const,
                Enum,
                ExternCrate,
                Fn,
                ForeignMod,
                Impl,
                Macro,
                Macro2,
                Mod,
                Static,
                Struct,
                Trait,
                TraitAlias,
                Type,
                Union,
                Use
            );
        }
    }

    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| Error::IoError(e))?;

    let ast = AssertUnwindSafe(syn::parse_file(content.as_str()).map_err(|e| Error::ParseError(e))?);
    let macro_paths_and_proc_macro_fns = AssertUnwindSafe(
        macro_paths_and_proc_macro_fns.iter()
            .map(|(s, f)| Ok((syn::parse_str(s)?, f)))
            .collect::<Result<Vec<(syn::Path, &F)>, _>>()
            .map_err(|e| Error::ParseError(e))?
    );

    panic::catch_unwind(|| {
        syn::visit::visit_file(&mut MacroVisitor::<F> {
            macro_paths_and_proc_macro_fns
        }, &*ast);
    }).map_err(|_| Error::ParseError(syn::parse::Error::new(
        proc_macro2::Span::call_site().into(), "macro expansion panicked"
    )))?;

    Ok(())
}

/// The error type for `emulate_*_macro_expansion`. If anything goes wrong during the file loading
/// or macro expansion, this type describes it.
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
        // All the tests are in this one function so they'll run sequentially. Something about how
        // Tarpaulin works seems to dislike having two instances running in parallel.

        {
            // Function-like
            let mut config = Config::default();
            let test_dir = env::current_dir().unwrap().join("examples").join("custom_assert");
            config.manifest = test_dir.join("Cargo.toml");
            config.test_timeout = time::Duration::from_secs(60);
            let (_trace_map, return_code) = launch_tarpaulin(&config, &None).unwrap();
            assert_eq!(return_code, 0);
        }

        {
            // Attribute-like
            let mut config = Config::default();
            let test_dir = env::current_dir().unwrap().join("examples").join("reference_counting");
            config.manifest = test_dir.join("Cargo.toml");
            config.test_timeout = time::Duration::from_secs(60);
            let (_trace_map, return_code) = launch_tarpaulin(&config, &None).unwrap();
            assert_eq!(return_code, 0);
        }
    }
}
