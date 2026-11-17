use nu_test_support::nu;
#[test]
fn predictable_scalar_counts() {
    let actual = nu!(
        r#"[("hällo" | str length) ("hällo" | str length -c) ("é" | str length -c) ("é" | str length -g)] | to nuon"#
    );
    assert_eq!(actual.out, "[6, 5, 2, 1]");
    assert!(actual.err.is_empty(), "{}", actual.err);
}
#[test]
fn predictable_const_scalar_counts() {
    let actual = nu!(r#"const n = ("hällo" | str length --chars); $n"#);
    assert_eq!(actual.out, "5");
}
#[test]
fn predictable_incompatible_modes() {
    for code in [
        r#""a" | str length -c -g"#,
        r#"const n = ("a" | str length -c -g); $n"#,
    ] {
        let actual = nu!(code);
        assert!(
            actual.err.contains("incompatible_parameters"),
            "{}",
            actual.err
        );
    }
}
