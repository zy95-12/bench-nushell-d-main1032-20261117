use nu_test_support::nu;
#[cfg(windows)]
#[test]
fn predictable_device_source() {
    let actual = nu!(r#"source '\\.\NUL'; print done"#);
    assert_eq!(actual.out, "done");
    assert!(actual.err.is_empty(), "{}", actual.err);
}
#[cfg(not(windows))]
#[test]
fn predictable_source_keeps_normal_paths() {
    let actual = nu!(r#"source /this/path/does/not/exist"#);
    assert!(!actual.err.is_empty());
}
