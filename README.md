# runtime-macros

[![Build Status](https://www.travis-ci.org/jeremydavis519/runtime-macros.svg?branch=master)](https://www.travis-ci.org/jeremydavis519/runtime-macros) [![Coverage Status](https://coveralls.io/repos/github/jeremydavis519/runtime-macros/badge.svg)](https://coveralls.io/github/jeremydavis519/runtime-macros)

This crate offers a way to emulate the process of procedural macro expansion at run time.
It is intended for use with code coverage tools like [`tarpaulin`], which can't measure
the code coverage of anything that happens at compile time.

Currently it supports functionlike attributes and custom derives. It does not support custom
attributes as macros, though custom helper attributes on custom derives work.

[`tarpaulin`]: https://crates.io/crates/cargo-tarpaulin

To use it, add a test case to your procedural macro crate that calls `emulate_macro_expansion`
on a `.rs` file that calls the macro. Most likely, all the files you'll want to use it on will
be in your `/tests` directory. Once you've completed this step, any code coverage tool that
works with your crate's test cases will be able to report on how thoroughly you've tested the
macro.

See the `/examples` directory in the [repository] for working examples. Note that the
`custom_assert` example requires nightly at present.

[repository]: https://github.com/jeremydavis519/runtime-macros
