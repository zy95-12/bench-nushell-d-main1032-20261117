use nu_test_support::{nu, pipeline};

#[cfg(feature = "sqlite")]
#[test]
fn can_query_single_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | query db "select * from strings"
            | where x =~ ell
            | length
        "#
    ));

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn invalid_sql_fails() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | query db "select *asdfasdf"
        "#
    ));

    assert!(actual.err.contains("syntax error"));
}

#[cfg(feature = "sqlite")]
#[test]
fn invalid_input_fails() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            "foo" | query db "select * from asdf"
        "#
    ));

    assert!(actual.err.contains("can't convert string"));
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_storage_query_roundtrip() {
    nu_test_support::playground::Playground::setup("sqlite_storage_query_roundtrip", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), pipeline(r#"
            [{name: "O'Brien", amount: 42}, {name: "λ", amount: -7}]
            | into sqlite ($env.PWD | path join roundtrip.db) --table-name records;
            open roundtrip.db
            | query db "select name, amount from records order by amount desc"
            | to json --raw
        "#));
        assert!(actual.status.success(), "{}", actual.err);
        assert_eq!(
            actual.out,
            r#"[{"name":"O'Brien","amount":42},{"name":"λ","amount":-7}]"#
        );
    });
}
