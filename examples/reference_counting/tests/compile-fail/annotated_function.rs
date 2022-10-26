extern crate reference_counting;

use reference_counting::reference_counted;

#[reference_counted]
fn foo() {}
