use custom_derive::HelloWorld;

#[test]
fn syntax_error() {
    #[derive("Name = Missing" =, HelloWorld)]
    struct MyStruct;

    assert_eq!(MyStruct::hello_world(), "Hello World");
}
