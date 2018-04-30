#![feature(proc_macro, proc_macro_non_items)]

extern crate custom_assert;

use custom_assert::custom_assert;

#[test]
fn assert_no_message() {
    custom_assert!(2 + 2 == 4);
}

// No test of custom_assert with a message! That part of the macro expansion won't be covered.
