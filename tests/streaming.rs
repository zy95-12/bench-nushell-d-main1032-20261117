use nu_test_support::nu;

#[test]
fn streaming_try_break_removes_handler() {
    let actual = nu!(
        r#"for x in [1] { try { break } catch { print 'stale handler' } }; error make {msg: 'outside try'}"#
    );
    assert!(
        actual.err.contains("outside try"),
        "out: {} err: {}",
        actual.out,
        actual.err
    );
    assert!(!actual.out.contains("stale handler"));
}

#[test]
fn streaming_try_continue_removes_handler() {
    let actual = nu!(
        r#"for x in [1 2] { if $x == 1 { try { continue } catch { print 'stale handler' } }; error make {msg: 'next iteration'} }"#
    );
    assert!(
        actual.err.contains("next iteration"),
        "out: {} err: {}",
        actual.out,
        actual.err
    );
    assert!(!actual.out.contains("stale handler"));
}

#[test]
fn streaming_loop_preserves_enclosing_handler() {
    let actual = nu!(
        r#"try { for x in [1] { try { break } }; error make {msg: 'outer'} } catch { 'caught' }"#
    );
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "caught");
}

#[test]
fn streaming_example_result_spans_are_source_spans() {
    let actual = nu!(
        r#"scope commands | where name == each | get 0.examples.0.result.0 | metadata | get span | to nuon"#
    );
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert!(
        !actual.out.contains("9223372036854775807"),
        "{}",
        actual.out
    );
    assert!(
        !actual.out.contains("18446744073709551615"),
        "{}",
        actual.out
    );
}

#[test]
fn streaming_namespace_help_retains_real_commands() {
    let actual = nu!(
        r#"let h = (detect); [( $h | str contains 'detect columns') ([1 2] | math sum) ((ansi --list | length) > 0)] | to nuon"#
    );
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "[true, 3, true]");
}

#[test]
fn streaming_table_configuration_roundtrips() {
    let actual = nu!(
        "$env.config.table.batch_duration = 20ms; $env.config.table.page_size = 2; [$env.config.table.batch_duration $env.config.table.page_size] | to nuon"
    );
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "[20000000ns, 2]");
}
