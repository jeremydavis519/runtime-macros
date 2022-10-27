extern crate plain_old_data;

use {
    std::mem::size_of,
    plain_old_data::Pod
};

// We can have multiple derives on the same line, and runtime-macros will consider all of them.
#[derive(Debug, Pod, Clone, Copy, PartialEq)]
#[repr(C, packed)]
struct State {
    x: u32,
    y: u32,
    z: u8
}

#[test]
fn pod_struct() {
    let state = State {
        x: u32::from_be(0x12345678),
        y: u32::from_be(0x9abcdef0),
        z: 0x55
    };
    let bytes = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x55];
    assert_eq!(<[u8; size_of::<State>()]>::from(state), bytes);
    assert_eq!(state, State::from(bytes));
}
