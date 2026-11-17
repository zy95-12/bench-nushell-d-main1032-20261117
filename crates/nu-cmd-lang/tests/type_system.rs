use nu_cmd_lang::create_default_context;
use nu_parser::parse;
use nu_protocol::{Type, ast::Expr, engine::StateWorkingSet};

#[test]
fn range_loop_infers_element_type_before_parsing_body() {
    for (range, expected) in [
        ("1..3", Type::Int),
        ("1.0..3.0", Type::Float),
        ("1..0.5..3", Type::Float),
    ] {
        let engine = create_default_context();
        let mut ws = StateWorkingSet::new(&engine);
        let source = format!("for i in {range} {{ $i + 1 }}");
        let block = parse(&mut ws, None, source.as_bytes(), false);
        assert!(ws.parse_errors.is_empty(), "{:?}", ws.parse_errors);
        let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
            panic!("for call")
        };
        let var = call.positional_nth(0).unwrap().as_var().unwrap();
        assert_eq!(ws.get_variable(var).ty, expected);
        let body = ws.get_block(call.positional_nth(2).unwrap().as_block().unwrap());
        let Expr::BinaryOp(lhs, _, _) = &body.pipelines[0].elements[0].expr.expr else {
            panic!("body expression")
        };
        assert_eq!(lhs.ty, expected);
    }
}

#[test]
fn inferred_variables_are_distinct_from_annotated_variables() {
    let engine = create_default_context();
    let mut ws = StateWorkingSet::new(&engine);
    let block = parse(&mut ws, None, b"let a: int = 1; let b = 1", false);
    assert!(ws.parse_errors.is_empty(), "{:?}", ws.parse_errors);
    for (index, annotated) in [(0, true), (1, false)] {
        let Expr::Call(call) = &block.pipelines[index].elements[0].expr.expr else {
            panic!("let call")
        };
        let var = call.positional_nth(0).unwrap().as_var().unwrap();
        assert_eq!(ws.get_variable(var).type_annotated, annotated);
    }
}
