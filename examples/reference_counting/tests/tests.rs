extern crate reference_counting;

use {
    std::sync::Mutex,
    reference_counting::reference_counted
};

#[reference_counted]
#[derive(Default)]
#[repr(C)]
struct SharedState {
    values: Vec<String>
}

static STATE: Mutex<SharedState> = Mutex::new(SharedState {
    values: Vec::new(),
    reference_count: 0
});

#[test]
fn ref_counted_struct() {
    let mut state = STATE.lock().unwrap();
    state.reference_count += 1;
    assert_eq!(state.reference_count, 1);
    state.reference_count -= 1;
}

// No test of reference_counted on an enum! That part of the macro expansion won't be covered.
