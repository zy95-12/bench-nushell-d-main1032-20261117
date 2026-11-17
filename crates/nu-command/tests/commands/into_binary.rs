use nu_test_support::nu;
#[test]
fn predictable_endian_and_compact() {
    let actual = nu!(
        r#"[(258 | into binary --endian big | encode hex) (258 | into binary -e little | encode hex) (258 | into binary -e big -c | encode hex) (258 | into binary -e little -c | encode hex)] | to nuon"#
    );
    assert_eq!(
        actual.out,
        r#"["0000000000000102", "0201000000000000", "0102", "0201"]"#
    );
}
#[test]
fn predictable_endian_rejects_unknown() {
    assert!(!nu!("1 | into binary --endian sideways").err.is_empty());
}

#[test]
fn predictable_endian_numeric_types_and_paths() {
    let actual = nu!(
        r#"[(true | into binary -e big | encode hex) (1.0 | into binary -e big | encode hex) (1ns | into binary -e big | encode hex) (1b | into binary -e big | encode hex) ({n: 258} | into binary -e big n | get n | encode hex)] | str join ','"#
    );
    assert_eq!(
        actual.out,
        "0000000000000001,3FF0000000000000,0000000000000001,0000000000000001,0000000000000102"
    );
}
