use custom_derive::HelloWorld;


#[test]
fn derive_hello_world() {
    #[derive(HelloWorld)]
    struct MyStruct;

    assert_eq!(MyStruct::hello_world(), "Hello World");
}

#[test]
fn derive_multiple() {
    #[derive(Debug, HelloWorld)]
    struct MyStruct;

    assert_eq!(MyStruct::hello_world(), "Hello World");
}
