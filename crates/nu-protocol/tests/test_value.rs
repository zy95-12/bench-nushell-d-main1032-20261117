use nu_protocol::{
    Config, Span, Value,
    engine::{EngineState, Stack},
};
use rstest::rstest;

#[test]
fn test_comparison_nothing() {
    let values = vec![
        Value::test_int(1),
        Value::test_string("string"),
        Value::test_float(1.0),
    ];

    let nothing = Value::nothing(Span::test_data());

    for value in values {
        assert!(matches!(
            value.eq(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool { val: false, .. })
        ));

        assert!(matches!(
            value.ne(Span::test_data(), &nothing, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));

        assert!(matches!(
            nothing.eq(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool { val: false, .. })
        ));

        assert!(matches!(
            nothing.ne(Span::test_data(), &value, Span::test_data()),
            Ok(Value::Bool { val: true, .. })
        ));
    }
}

#[rstest]
#[case(365 * 24 * 3600 * 1_000_000_000, "52wk 1day")]
#[case( (((((((7 + 2) * 24 + 3) * 60 + 4) * 60) + 5) * 1000 + 6) * 1000 + 7) * 1000 + 8,
"1wk 2day 3hr 4min 5sec 6ms 7µs 8ns")]
fn test_duration_to_string(#[case] in_ns: i64, #[case] expected: &str) {
    let dur = Value::test_duration(in_ns);
    assert_eq!(
        expected,
        dur.to_expanded_string("", &Config::default()),
        "expected != observed"
    );
}

#[test]
fn test_case_insensitive_env_var() {
    let mut engine_state = EngineState::new();
    let stack = Stack::new();

    for (name, value) in std::env::vars() {
        engine_state.add_env_var(name, Value::test_string(value));
    }

    let path_lower = engine_state.get_env_var_insensitive("path");
    let path_upper = engine_state.get_env_var_insensitive("PATH");
    let path_mixed = engine_state.get_env_var_insensitive("PaTh");

    assert_eq!(path_lower, path_upper);
    assert_eq!(path_lower, path_mixed);

    let stack_path_lower = stack.get_env_var_insensitive(&engine_state, "path");
    let stack_path_upper = stack.get_env_var_insensitive(&engine_state, "PATH");
    let stack_path_mixed = stack.get_env_var_insensitive(&engine_state, "PaTh");

    assert_eq!(stack_path_lower, stack_path_upper);
    assert_eq!(stack_path_lower, stack_path_mixed);
}

#[test]
fn float_equality_is_symmetric_at_scaled_epsilon_boundary() {
    for lhs in [2.0_f64, -2.0, 4.0, -4.0] {
        let rhs = f64::from_bits(lhs.to_bits() + 1);
        let a = Value::test_float(lhs);
        let b = Value::test_float(rhs);
        assert_eq!(a == b, b == a, "{lhs:?}, {rhs:?}");
        assert_eq!(
            a.partial_cmp(&b),
            b.partial_cmp(&a).map(std::cmp::Ordering::reverse)
        );
        assert_eq!(
            a.eq(Span::test_data(), &b, Span::test_data()).unwrap(),
            b.eq(Span::test_data(), &a, Span::test_data()).unwrap()
        );
    }
    assert_ne!(Value::test_float(2.0), Value::test_float(2.001));
}

#[test]
fn negated_prefix_and_suffix_are_string_only() {
    let span = Span::test_data();
    for (lhs, rhs) in [
        ("abc", "a"),
        ("abc", "c"),
        ("abc", ""),
        ("", "abc"),
        ("αβ", "β"),
    ] {
        let lhs = Value::test_string(lhs);
        let rhs = Value::test_string(rhs);
        assert_eq!(
            lhs.not_starts_with(span, &rhs, span)
                .unwrap()
                .as_bool()
                .unwrap(),
            !lhs.starts_with(span, &rhs, span)
                .unwrap()
                .as_bool()
                .unwrap()
        );
        assert_eq!(
            lhs.not_ends_with(span, &rhs, span)
                .unwrap()
                .as_bool()
                .unwrap(),
            !lhs.ends_with(span, &rhs, span).unwrap().as_bool().unwrap()
        );
    }
    for (lhs, rhs) in [
        (Value::test_int(1), Value::test_string("1")),
        (Value::test_string("1"), Value::test_int(1)),
        (Value::test_list(vec![]), Value::test_list(vec![])),
    ] {
        assert!(lhs.not_starts_with(span, &rhs, span).is_err());
        assert!(lhs.not_ends_with(span, &rhs, span).is_err());
    }
}
