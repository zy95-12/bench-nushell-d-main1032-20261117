use nu_test_support::nu;
#[test]
fn predictable_nuon_equals() {
    let actual = nu!(r#"[["a=b"] { "a=b": "x=y" }] | to nuon"#);
    assert_eq!(actual.out, r#"[["a=b"], {"a=b": "x=y"}]"#);
}
#[test]
fn predictable_cellpath_string() {
    let actual = nu!(r#"$.foo.2.bar? | into string"#);
    assert_eq!(actual.out, "$.foo.2.bar?");
}
#[cfg(feature = "sqlite")]
#[test]
fn predictable_sqlite_json_roundtrip() {
    use nu_test_support::playground::Playground;
    Playground::setup("predictable_json", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#"let db = ($env.PWD | path join test.db); [{data: {items: [1 {x: true} null]}}] | into sqlite $db; open $db | query db 'select data from main' | get 0.data.items.1.x"#);
        assert_eq!(actual.out, "true", "{}", actual.err);
    });
}
#[cfg(feature = "sqlite")]
#[test]
fn predictable_sqlite_jsonb_roundtrip() {
    use nu_test_support::playground::Playground;
    Playground::setup("predictable_jsonb", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#"let db = ($env.PWD | path join test.db); {seed: 1} | into sqlite $db -t seed; open $db | query db 'create table test (data JSONB, ordinary TEXT, bytes BLOB)'; [{data: {items: [1 2]} ordinary: '{"x":1}' bytes: 0x[0102]}] | into sqlite $db -t test; let row = (open $db | query db 'select data, ordinary, bytes, typeof(data) as kind from test' | get 0); if $row.kind != blob { error make {msg: 'JSONB storage must use a blob'} }; if $row.ordinary != '{"x":1}' { error make {msg: 'ordinary text changed'} }; if ($row.bytes | encode hex) != '0102' { error make {msg: 'ordinary blob changed'} }; $row.data.items | to nuon"#);
        assert_eq!(actual.out, "[1, 2]", "{}", actual.err);
    });
}

#[test]
fn predictable_cellpath_interpolation() {
    let actual = nu!(r#"$"path=($.foo.2.bar?)""#);
    assert_eq!(actual.out, "path=$.foo.2.bar?", "{}", actual.err);
}
