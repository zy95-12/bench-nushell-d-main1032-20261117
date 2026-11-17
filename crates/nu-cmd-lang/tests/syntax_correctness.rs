use nu_cmd_lang::create_default_context;
use nu_parser::parse;
use nu_protocol::{
    PipelineData, Span, Value,
    debugger::WithoutDebug,
    engine::{Stack, StateWorkingSet},
};

fn eval(source: &str) -> Value {
    let mut engine = create_default_context();
    engine.add_env_var(
        "PWD".into(),
        Value::test_string(std::env::current_dir().unwrap().to_string_lossy()),
    );
    let mut ws = StateWorkingSet::new(&engine);
    let block = parse(&mut ws, None, source.as_bytes(), false);
    assert!(
        ws.parse_errors.is_empty(),
        "{source}: {:?}",
        ws.parse_errors
    );
    assert!(
        ws.compile_errors.is_empty(),
        "{source}: {:?}",
        ws.compile_errors
    );
    let delta = ws.render();
    engine.merge_delta(delta).unwrap();
    nu_engine::eval_block::<WithoutDebug>(&engine, &mut Stack::new(), &block, PipelineData::Empty)
        .unwrap()
        .into_value(Span::test_data())
        .unwrap()
}

#[test]
fn power_is_right_associative() {
    assert_eq!(eval("2 ** 3 ** 2"), Value::test_int(512));
    assert_eq!(eval("2 ** 3 ** 2 + 1"), Value::test_int(513));
    assert_eq!(eval("(2 ** 3) ** 2"), Value::test_int(64));
    assert_eq!(eval("20 - 5 - 2"), Value::test_int(13));
}

#[test]
fn bare_interpolation_with_subexpressions_at_both_ends() {
    assert_eq!(
        eval(r##"echo (r#'first')last'#)"##),
        Value::test_string("first')last")
    );
    assert_eq!(
        eval(r#"echo ('first')-('last')"#),
        Value::test_string("first-last")
    );
    assert_eq!(
        eval(r#"echo ('first')('last')"#),
        Value::test_string("firstlast")
    );
}

#[test]
fn negated_string_operators_evaluate() {
    for (source, expected) in [
        (r#"'abc' not-starts-with 'a'"#, false),
        (r#"'abc' not-starts-with 'b'"#, true),
        (r#"'abc' not-ends-with 'c'"#, false),
        (r#"'abc' not-ends-with 'b'"#, true),
        (r#"'abc' not-starts-with ''"#, false),
        (r#"const x = 'abc' not-ends-with 'x'; $x"#, true),
    ] {
        assert_eq!(eval(source), Value::test_bool(expected));
    }
}

#[test]
fn export_def_does_not_leak_parameters() {
    let engine = create_default_context();
    let mut ws = StateWorkingSet::new(&engine);
    parse(
        &mut ws,
        None,
        b"export def foo [parameter: int] { $parameter }; $parameter",
        false,
    );
    assert!(
        ws.parse_errors
            .iter()
            .any(|err| matches!(err, nu_protocol::ParseError::VariableNotFound(..))),
        "{:?}",
        ws.parse_errors
    );
    assert_eq!(
        eval("export def foo [parameter: int] { $parameter }; foo 7"),
        Value::test_int(7)
    );
}

#[test]
fn redefining_def_reports_a_diagnostic() {
    let engine = create_default_context();
    let mut ws = StateWorkingSet::new(&engine);
    parse(&mut ws, None, b"def def [] {}; def foo [] {}", false);
    assert!(
        ws.parse_errors
            .iter()
            .any(|err| err.to_string().contains("Cannot redefine")),
        "{:?}",
        ws.parse_errors
    );
}

#[test]
fn loop_control_is_lexical() {
    for source in [
        "break",
        "continue",
        "if false { break }",
        "loop { let f = {|| break } }",
        "loop { def f [] { continue } }",
        "do { break }",
        "module m { export def f [] { break } }",
        "loop { let f = {|| continue } }",
    ] {
        let engine = create_default_context();
        let mut ws = StateWorkingSet::new(&engine);
        parse(&mut ws, None, source.as_bytes(), false);
        assert!(
            ws.parse_errors.is_empty(),
            "{source}: {:?}",
            ws.parse_errors
        );
        assert!(
            ws.compile_errors
                .iter()
                .any(|err| matches!(err, nu_protocol::CompileError::NotInALoop { .. })),
            "{source}: {:?}",
            ws.compile_errors
        );
    }
    for source in [
        "loop { if true { break } }; 7",
        "mut n = 0; for x in [1 2 3] { if $x == 2 { continue }; $n += $x }; $n + 3",
        "do { loop { break } }; 7",
        "while (break) {}; 7",
        "loop { try { 1 / 0 } catch { break } }; 7",
        "loop { match 1 { 1 => { break } }; }; 7",
        "def f [] { loop { break }; 7 }; f",
    ] {
        assert_eq!(eval(source), Value::test_int(7));
    }
}

#[test]
fn branched_block_calls_with_redirected_output() {
    let path = std::env::temp_dir().join(format!("nu-branched-redirection-{}", std::process::id()));
    for branch in [true, false] {
        let source = format!(
            "def f [] {{ 'result' }}; if {branch} {{ f }} else {{ f }} o> '{}'",
            path.display()
        );
        eval(&source);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "result");
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn external_subexpressions_keep_each_argument_intact() {
    let mut engine = create_default_context();
    let mut ws = StateWorkingSet::new(&engine);
    ws.add_decl(nu_protocol::Signature::build("run-external").predeclare());
    let block = parse(
        &mut ws,
        None,
        br#"^external pre('two words')post ('left')-('right') tail a('"quoted"')z"#,
        false,
    );
    assert!(ws.parse_errors.is_empty(), "{:?}", ws.parse_errors);
    assert!(ws.compile_errors.is_empty(), "{:?}", ws.compile_errors);
    let nu_protocol::ast::Expr::ExternalCall(_, args) = &block.pipelines[0].elements[0].expr.expr
    else {
        panic!("{block:?}")
    };
    let args = args.to_vec();
    // Subexpressions are normally compiled inline with their enclosing call.
    // Compile their standalone blocks as well for direct argument evaluation.
    let compiled: Vec<_> = ws
        .delta
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.ir_block.is_none())
        .map(|(index, block)| {
            (
                nu_protocol::BlockId::new(engine.num_blocks() + index),
                nu_engine::compile(&ws, block).unwrap(),
            )
        })
        .collect();
    for (id, ir) in compiled {
        ws.get_block_mut(id).ir_block = Some(ir);
    }
    engine.merge_delta(ws.render()).unwrap();
    let mut stack = Stack::new();
    let values: Vec<_> = args
        .iter()
        .map(|arg| {
            nu_engine::eval_expression::<WithoutDebug>(&engine, &mut stack, arg.expr())
                .unwrap()
                .coerce_string()
                .unwrap()
        })
        .collect();
    assert_eq!(
        values,
        ["pretwo wordspost", "left-right", "tail", "a\"quoted\"z"]
    );
}

#[test]
fn imported_function_cannot_borrow_the_callers_loop() {
    let path = std::env::temp_dir().join(format!("nu-loop-boundary-{}.nu", std::process::id()));
    std::fs::write(&path, "export def invalid [] { break }").unwrap();
    let mut engine = create_default_context();
    engine.add_env_var(
        "PWD".into(),
        Value::test_string(std::env::current_dir().unwrap().to_string_lossy()),
    );
    let mut ws = StateWorkingSet::new(&engine);
    let source = format!("use '{}' *; loop {{ invalid }}", path.display());
    parse(&mut ws, None, source.as_bytes(), false);
    std::fs::remove_file(path).unwrap();
    assert!(ws.parse_errors.is_empty(), "{:?}", ws.parse_errors);
    assert!(
        ws.compile_errors
            .iter()
            .any(|error| matches!(error, nu_protocol::CompileError::NotInALoop { .. })),
        "{:?}",
        ws.compile_errors
    );
}
