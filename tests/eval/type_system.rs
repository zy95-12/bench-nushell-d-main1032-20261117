use nu_test_support::nu;

#[test]
fn null_spreads_are_empty_in_literals_and_constants() {
    for command in [
        "let n = null; [1 ...$n 2] | to nuon",
        "const n = null; const xs = [1 ...$n 2]; $xs | to nuon",
    ] {
        let out = nu!(command);
        assert!(out.status.success(), "{}", out.err);
        assert_eq!(out.out, "[1, 2]");
    }
    for command in [
        "let n = null; {a: 1 ...$n} | to nuon",
        "const n = null; const r = {a: 1 ...$n}; $r | to nuon",
    ] {
        let out = nu!(command);
        assert!(out.status.success(), "{}", out.err);
        assert_eq!(out.out, "{a: 1}");
    }
}

#[test]
fn table_headers_can_be_variables() {
    let out = nu!("let h = 'name'; [[$h]; [42]] | get name.0");
    assert!(out.status.success(), "{}", out.err);
    assert_eq!(out.out, "42");
}

#[test]
fn range_loop_variable_is_numeric() {
    let out = nu!("for i in 1..3 { $i ++ 'x' }");
    assert!(
        out.err.contains("operator") && !out.status.success(),
        "{}",
        out.err
    );
    assert!(out.err.contains("int"), "{}", out.err);
}

#[test]
fn annotated_variables_are_checked_at_runtime_when_enabled() {
    for command in [
        "def dynamic []: nothing -> any { 'wrong' }; let x: int = dynamic; $x",
        "def dynamic []: nothing -> any { 'wrong' }; mut x: int = 1; $x = dynamic; $x",
        "def dynamic []: nothing -> any { 'wrong' }; mut x: record<a: int> = {a: 1}; $x.a = dynamic; $x",
    ] {
        let out = nu!(experimental: vec!["enforce-runtime-annotations".into()], command);
        assert!(!out.status.success(), "{}", out.out);
        assert!(out.err.contains("cant_convert"), "{}", out.err);
        assert!(out.err.contains("int"), "{}", out.err);
    }
}

#[test]
fn ordinary_mode_preserves_dynamic_assignment() {
    let out = nu!("def dynamic []: nothing -> any { 'wrong' }; let x: int = dynamic; $x");
    assert!(out.status.success(), "{}", out.err);
    assert_eq!(out.out, "wrong");
}

#[test]
fn explicit_option_disable_and_multiple_options_work() {
    let command = "def dynamic []: nothing -> any { 'wrong' }; let x: int = dynamic; $x";
    let out = nu!(experimental: vec!["enforce-runtime-annotations=false".into()], command);
    assert!(out.status.success(), "{}", out.err);
    assert_eq!(out.out, "wrong");
    let out = nu!(experimental: vec!["reorder-cell-paths=false".into(), "enforce-runtime-annotations".into()], command);
    assert!(out.err.contains("cant_convert"), "{}", out.err);
}

#[test]
fn runtime_enforcement_accepts_compatible_values_and_inferred_changes() {
    for command in [
        "def dynamic []: nothing -> any { 2 }; mut x: int = dynamic; $x = dynamic; $x",
        "def dynamic []: nothing -> any { 'ok' }; mut x = 1; $x = dynamic; $x",
        "def dynamic []: nothing -> any { 2.5 }; let x: number = dynamic; $x",
        "def dynamic []: nothing -> any { [1 2] }; let x: list<int> = dynamic; $x | length",
        "let g: glob = '*.txt'; let s: string = $g; $s",
    ] {
        let out = nu!(experimental: vec!["enforce-runtime-annotations".into()], command);
        assert!(out.status.success(), "{command}: {}", out.err);
    }
}

#[test]
fn runtime_enforcement_checks_list_elements_and_reports_value_span() {
    let out = nu!(experimental: vec!["enforce-runtime-annotations".into()],
        "def dynamic []: nothing -> any { [1 'bad'] }; let x: list<int> = dynamic; $x");
    assert!(out.err.contains("cant_convert"), "{}", out.err);
    assert!(out.err.contains("columns 37 to 41"), "{}", out.err);
}

#[test]
fn table_variable_headers_reject_non_string_types() {
    for value in ["true", "1"] {
        let out = nu!(format!("let cols = {value}; [[$cols]; [1]]"));
        assert!(!out.status.success(), "{value} header was accepted");
        assert!(out.err.contains("must be a string"), "{}", out.err);
    }
}

#[test]
fn table_any_headers_require_runtime_strings() {
    for value in ["1", "true", "[]"] {
        let out = nu!(format!(
            "def dynamic []: nothing -> any {{ {value} }}; let cols = (dynamic); [[$cols]; [1]]"
        ));
        assert!(!out.status.success(), "Any {value} header was accepted");
        assert!(
            out.err.contains("can't convert") && out.err.contains("nu::shell::cant_convert"),
            "{}",
            out.err
        );
    }
}

#[test]
fn table_string_compatible_headers_still_work() {
    for source in [
        "let cols = 'name'; [[$cols]; [1]] | get name.0",
        "def dynamic []: nothing -> any { 'name' }; let cols = (dynamic); [[$cols]; [1]] | get name.0",
        "let cols = ('name' | into glob); [[$cols]; [1]] | get name.0",
        "[['42']; [1]] | get '42'.0",
        "let key = 42; {$key: 1} | get '42'",
    ] {
        let out = nu!(source);
        assert!(out.status.success(), "{source}: {}", out.err);
        assert_eq!(out.out, "1", "{source}");
    }
}
