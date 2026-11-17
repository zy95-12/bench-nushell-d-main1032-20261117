use nu_test_support::nu;
#[test]
fn predictable_table_delimiters_and_escaping() {
    let actual = nu!(r#"[{"a|b": "x|y\nz"}] | to md"#);
    assert_eq!(actual.out, "|a\\|b||---||x\\|y<br>z|");
}
#[test]
fn predictable_optional_markdown_escaping() {
    let actual = nu!(r#"[[a]; ["*x* <b>"]] | to md --escape"#);
    assert_eq!(actual.out, "|a||---||\\*x\\* &lt;b&gt;|");
}

#[test]
fn predictable_adjacent_record_tables_have_blank_line() {
    let actual = nu!(r#"[{a: 1} {b: 2}] | to md --per-element | to json -r"#);
    assert_eq!(actual.out, r#""|a|\n|---|\n|1|\n\n|b|\n|---|\n|2|""#);
}

#[test]
fn predictable_grouped_mixed_schema_tables_have_blank_line() {
    let actual = nu!(r#"[{a: 1} {a: 2} {b: 3} {b: 4}] | to md --per-element | to json -r"#);
    assert_eq!(
        actual.out,
        r#""|a|\n|---|\n|1|\n|2|\n\n|b|\n|---|\n|3|\n|4|""#
    );
}

#[test]
fn predictable_heading_blockquote_and_table_boundaries() {
    let actual =
        nu!(r#"[{H1: Title} {blockquote: Quote} {a: 1}] | to md --per-element | to json -r"#);
    assert_eq!(actual.out, r##""# Title\n> Quote\n\n|a|\n|---|\n|1|""##);
    let list = nu!(r#"[1 2 3] | to md --per-element | to json -r"#);
    assert_eq!(list.out, r#""1\n2\n3""#);
}
