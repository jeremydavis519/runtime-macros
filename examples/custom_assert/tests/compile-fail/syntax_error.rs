#![feature(proc_macro_hygiene)]

extern crate custom_assert;

use custom_assert::custom_assert;

#[test]
fn syntax_error() {
    custom_assert!(2 + 2 == );
}
