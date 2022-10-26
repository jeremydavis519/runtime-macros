extern crate custom_assert;

use custom_assert::custom_assert;

// This isn't marked with #[test] because we actually run this file as a doctest.
fn syntax_error() {
    custom_assert!(2 + 2 == );
}
