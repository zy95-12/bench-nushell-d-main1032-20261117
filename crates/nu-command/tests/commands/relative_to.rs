use nu_test_support::nu;
#[test]
fn predictable_component_boundaries() {
    assert_eq!(
        nu!(r#""/foo/bar/Leaf" | path relative-to "/foo/bar""#).out,
        "Leaf"
    );
    assert!(
        !nu!(r#""/foo/barley" | path relative-to "/foo/bar""#)
            .err
            .is_empty()
    );
}
#[test]
fn predictable_platform_casing() {
    let actual = nu!(r#""parent/Child/Leaf" | path relative-to "PARENT/child""#);
    if cfg!(any(windows, target_os = "macos")) {
        assert_eq!(actual.out, "Leaf");
    } else {
        assert!(!actual.err.is_empty());
    }
}
