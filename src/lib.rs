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

extern crate proc_macro2;
extern crate syn;

use std::fs;
use std::io::Read;

/// Parses the given Rust source code file, searching for macro expansions that use `macro_path`.
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
/// Also, this function uses `proc_macro2::TokenStream`, not the standard but unstable
/// `proc_macro::TokenStream`. You can convert between them using their `into` methods, as shown
/// below.
/// 
/// # Example
/// 
/// ```ignore
/// # // This example doesn't compile because procedural macros can only be made in crates with
/// # // type "proc-macro".
/// # #![cfg(feature = "proc-macro")]
/// # #![feature(proc_macro)]
/// # extern crate proc_macro;
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
pub fn emulate_macro_expansion<F>(mut file: fs::File, macro_path: &str, proc_macro_fn: F)
        where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    struct MacroVisitor<'a, F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream> {
        macro_path: &'a str,
        proc_macro_fn: F
    }
    impl<'a, 'ast, F> syn::visit::Visit<'ast> for MacroVisitor<'a, F>
            where F: Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        fn visit_macro(&mut self, macro_item: &'ast syn::Macro) {
            if macro_item.path == syn::parse_str::<syn::Path>(self.macro_path).unwrap() {
                (self.proc_macro_fn)(macro_item.tts.clone().into());
            }
        }
    }
    
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    
    let ast = syn::parse_file(content.as_str()).unwrap();
    syn::visit::visit_file(&mut MacroVisitor::<F> { macro_path, proc_macro_fn }, &ast);
}

#[cfg(test)]
mod tests {
    extern crate cargo_tarpaulin;
    use self::cargo_tarpaulin::launch_tarpaulin;
    use self::cargo_tarpaulin::config::Config;
    use self::cargo_tarpaulin::traces::CoverageStat;
    use std::{env, time};

    #[test]
    fn proc_macro_coverage() {
        let mut config = Config::default();
        config.test_timeout = time::Duration::from_secs(60);
        let mut test_dir = env::current_dir().unwrap();
        test_dir.push("examples");
        test_dir.push("custom_assert");
        config.manifest = test_dir.join("Cargo.toml");
        let res = launch_tarpaulin(&config).unwrap();
        let lib_file = test_dir.join("src/lib.rs");
        let lib_hits = res.covered_in_path(&lib_file);
        let lib_lines = res.coverable_in_path(&lib_file);
        assert_eq!(lib_hits, 28);
        assert_eq!(lib_lines, 36);

        // Make sure Tarpaulin actually hits the lines in the macro's code.
        let should_hit_once = &[22, 23, 24, 25, 26, 35, 36, 37, 38, 43, 49, 52,
            54, 55, 58, 62, 63, 66, 69, 83, 84, 85, 86, 87];
        for trace in res.get_child_traces(&lib_file) {
            if should_hit_once.contains(&trace.line) {
                assert_eq!(CoverageStat::Line(1), trace.stats);
            }
        }
    }
}
