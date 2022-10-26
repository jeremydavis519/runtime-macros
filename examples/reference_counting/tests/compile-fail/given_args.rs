extern crate reference_counting;

use reference_counting::reference_counted;

#[reference_counted(unrecognized_argument)]
struct SharedState {
    values: Vec<String>
}
