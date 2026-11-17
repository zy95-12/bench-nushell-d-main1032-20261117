use nu_parser::parse;
use nu_protocol::{
    Signature, SyntaxShape, Type,
    engine::{EngineState, StateWorkingSet},
};

fn parsed_type(source: &str) -> Type {
    let engine = EngineState::new();
    let mut ws = StateWorkingSet::new(&engine);
    let block = parse(&mut ws, None, source.as_bytes(), false);
    assert!(ws.parse_errors.is_empty(), "{:?}", ws.parse_errors);
    block.pipelines[0].elements[0].expr.ty.clone()
}

#[test]
fn mixed_literal_types_widen() {
    assert_eq!(parsed_type("[1 2.5]"), Type::List(Box::new(Type::Number)));
    assert_eq!(
        parsed_type("[[a]; [null] [1]]"),
        Type::Table(vec![("a".into(), Type::Any)].into())
    );
    assert_eq!(
        parsed_type("[[a]; [1] [null]]"),
        Type::Table(vec![("a".into(), Type::Any)].into())
    );
    assert_eq!(
        parsed_type("[[a]; [[1]] [[2.5]]]"),
        Type::Table(vec![("a".into(), Type::List(Box::new(Type::Number)))].into())
    );
}

fn pipeline_errors(source: &str) -> Vec<nu_protocol::ParseError> {
    let engine = EngineState::new();
    let mut ws = StateWorkingSet::new(&engine);
    ws.add_decl(
        Signature::build("multi")
            .input_output_types(vec![
                (Type::Nothing, Type::Int),
                (Type::Nothing, Type::String),
            ])
            .predeclare(),
    );
    ws.add_decl(
        Signature::build("text")
            .input_output_types(vec![(Type::String, Type::String)])
            .predeclare(),
    );
    ws.add_decl(
        Signature::build("flag")
            .input_output_types(vec![(Type::Bool, Type::Bool)])
            .predeclare(),
    );
    ws.add_decl(
        Signature::build("condition")
            .required("test", SyntaxShape::RowCondition, "test")
            .predeclare(),
    );
    parse(&mut ws, None, source.as_bytes(), false);
    ws.parse_errors
}

#[test]
fn pipeline_preserves_alternative_outputs() {
    assert!(pipeline_errors("null | multi | text").is_empty());
    let errors = pipeline_errors("multi | flag");
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message = errors[0].to_string();
    assert!(
        message.contains("int") && message.contains("string"),
        "{message}"
    );
    assert!(!pipeline_errors("multi | text | flag").is_empty());
}

#[test]
fn row_condition_checks_pipeline_types() {
    assert!(!pipeline_errors("condition (multi | flag)").is_empty());
}

#[test]
fn string_glob_compatibility_is_symmetric() {
    let engine = EngineState::new();
    let mut ws = StateWorkingSet::new(&engine);
    ws.add_decl(
        Signature::build("text")
            .required("s", SyntaxShape::String, "text")
            .predeclare(),
    );
    ws.add_variable(
        b"g".to_vec(),
        nu_protocol::Span::unknown(),
        Type::Glob,
        false,
    );
    parse(&mut ws, None, b"text $g", false);
    assert!(ws.parse_errors.is_empty(), "{:?}", ws.parse_errors);
}

#[test]
fn table_columns_never_narrow_after_mixed_rows() {
    assert_eq!(
        parsed_type("[[a]; [1] ['text'] [2]]"),
        Type::Table(vec![("a".into(), Type::Any)].into())
    );
    assert_eq!(
        parsed_type("[[a]; ['text'] [1] ['text']]"),
        Type::Table(vec![("a".into(), Type::Any)].into())
    );
}

#[test]
fn list_records_widen_fields_recursively() {
    assert_eq!(
        parsed_type("[{a: [1]} {a: [2.5]}]"),
        Type::List(Box::new(Type::Record(
            vec![("a".into(), Type::List(Box::new(Type::Number)))].into()
        )))
    );
}

#[test]
fn row_conditions_reject_known_non_boolean_results() {
    assert!(!pipeline_errors("condition 42").is_empty());
    assert!(!pipeline_errors("condition { 'text' }").is_empty());
    assert!(pipeline_errors("condition true").is_empty());
}
