# runtime-macros

This crate offers a way to emulate the process of procedural macro expansion at run time.
It is intended for use with code coverage tools like [`cargo-tarpaulin`], which can't measure
the code coverage of anything that happens at compile time.

The `runtime-macros` crate works with `functionlike!`, `#[attributelike]`, and `#[derive(...)]`
procedural macros. There are no plans to support declarative macros.

[`cargo-tarpaulin`]: https://crates.io/crates/cargo-tarpaulin

To use `runtime-macros`, add a test case to your procedural macro crate that calls one of the
`emulate_*_macro_expansion` functions on a `.rs` file that calls the macro. Most likely, all the
files you'll want to use it on will be in your crate's `/tests` directory. Once you've completed
this step, any code coverage tool that works with your crate's test cases will be able to report on
how thoroughly you've tested the macro.

See the `/examples` directory in the [repository] for working examples.

[repository]: https://github.com/jeremydavis519/runtime-macros
