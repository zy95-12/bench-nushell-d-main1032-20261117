#[test]
fn value_module_reexports_multilife() {
    use nu_protocol::value::MultiLife;

    let outer: MultiLife<'static, '_, str> = MultiLife::Out("outer");
    let text = String::from("local");
    let local: MultiLife<'static, '_, str> = MultiLife::Local(&text);
    assert_eq!(&*outer, "outer");
    assert_eq!(&*local, "local");
}
