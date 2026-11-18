use nu_test_support::nu;

#[test]
fn each_works_separately() {
    let actual = nu!("echo [1 2 3] | each { |it| echo $it 10 | math sum } | to json -r");

    assert_eq!(actual.out, "[11,12,13]");
}

#[test]
fn each_group_works() {
    let actual = nu!("echo [1 2 3 4 5 6] | chunks 3 | to json --raw");

    assert_eq!(actual.out, "[[1,2,3],[4,5,6]]");
}

#[test]
fn each_window() {
    let actual = nu!("echo [1 2 3 4] | window 3 | to json --raw");

    assert_eq!(actual.out, "[[1,2,3],[2,3,4]]");
}

#[test]
fn each_window_stride() {
    let actual = nu!("echo [1 2 3 4 5 6] | window 3 -s 2 | to json --raw");

    assert_eq!(actual.out, "[[1,2,3],[3,4,5]]");
}

#[test]
fn each_no_args_in_block() {
    let actual = nu!("echo [[foo bar]; [a b] [c d] [e f]] | each {|i| $i | to json -r } | get 1");

    assert_eq!(actual.out, r#"{"foo":"c","bar":"d"}"#);
}

#[test]
fn each_implicit_it_in_block() {
    let actual = nu!(
        "echo [[foo bar]; [a b] [c d] [e f]] | each { |it| nu --testbin cococo $it.foo } | str join"
    );

    assert_eq!(actual.out, "ace");
}

#[test]
fn each_uses_enumerate_index() {
    let actual = nu!("[7 8 9 10] | enumerate | each {|el| $el.index } | to nuon");

    assert_eq!(actual.out, "[0, 1, 2, 3]");
}

#[test]
fn each_while_uses_enumerate_index() {
    let actual = nu!("[7 8 9 10] | enumerate | each while {|el| $el.index } | to nuon");

    assert_eq!(actual.out, "[0, 1, 2, 3]");
}

#[test]
fn errors_in_nested_each_show() {
    let actual = nu!("[[1,2]] | each {|x| $x | each {|y| error make {msg: \"oh noes\"} } }");
    assert!(actual.err.contains("oh noes"))
}

#[test]
fn streaming_flatten_does_not_evaluate_unused_results() {
    let actual = nu!(
        r#"[1 2] | each --flatten { 1..3 | each {|x| if $x > 1 { error make {msg: 'overconsumed'} } else { $x } } } | first"#
    );
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "1");
}

#[test]
fn streaming_flatten_preserves_order() {
    let actual = nu!("[1 2] | each --flatten {|x| [$x ($x + 10)] } | to nuon");
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "[1, 11, 2, 12]");
}

#[test]
fn streaming_single_null_skips_closure() {
    for flags in ["", "--keep-empty", "--flatten"] {
        let actual = nu!(format!(
            "null | each {flags} {{ error make {{msg: 'called on null'}} }} | length"
        ));
        assert!(actual.err.is_empty(), "{}", actual.err);
        assert_eq!(actual.out, "0");
    }
}

#[test]
fn streaming_nested_errors_survive_ignoring_closures() {
    let actual = nu!(
        r#"[1] | each { [2] | each { error make {msg: 'nested failure'} } | each { 42 } } | each { 99 }"#
    );
    assert!(actual.err.contains("nested failure"), "{}", actual.err);
    assert_eq!(
        actual.err.matches("eval_block_with_input").count(),
        2,
        "{}",
        actual.err
    );
}

#[test]
fn streaming_default_each_keeps_nested_lists() {
    let actual = nu!("[1 2] | each {|x| [$x ($x + 10)] } | to nuon");
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "[[1, 11], [2, 12]]");
}

#[test]
fn nested_each_errors_include_each_closure_level_once() {
    for inner in ["", "--flatten"] {
        for outer in ["", "--flatten"] {
            let actual = nu!(format!(
                "[[1]] | each {outer} {{|xs| $xs | each {inner} {{ error make {{msg: 'original nested failure'}} }} }}"
            ));
            assert!(
                actual.err.contains("original nested failure"),
                "{}",
                actual.err
            );
            assert_eq!(
                actual.err.matches("eval_block_with_input").count(),
                2,
                "inner={inner}, outer={outer}: {}",
                actual.err
            );
        }
    }
}
