use nu_protocol::{
    DataSource, IntoPipelineData, PipelineData, PipelineMetadata, Span, Value,
    engine::{EngineState, Stack, StateWorkingSet},
};

fn evaluate(code: &str, input: PipelineData) -> PipelineData {
    evaluate_result(code, input).unwrap()
}

fn evaluate_result(
    code: &str,
    input: PipelineData,
) -> Result<PipelineData, nu_protocol::ShellError> {
    let mut engine = nu_command::add_shell_command_context(nu_cmd_lang::create_default_context());
    engine.add_env_var(
        "PWD".into(),
        Value::test_string(std::env::current_dir().unwrap().to_string_lossy()),
    );
    let mut working = StateWorkingSet::new(&engine);
    let block = nu_parser::parse(&mut working, None, code.as_bytes(), false);
    assert!(
        working.parse_errors.is_empty(),
        "{:?}",
        working.parse_errors
    );
    let delta = working.render();
    engine.merge_delta(delta).unwrap();
    nu_engine::eval_block::<nu_protocol::debugger::WithoutDebug>(
        &engine,
        &mut Stack::new(),
        &block,
        input,
    )
}

#[test]
fn streaming_partial_results_clear_content_type_and_retain_origin() {
    for command in [
        "first 1",
        "last 1",
        "skip 1",
        "take 1",
        "skip while { false }",
        "skip until { true }",
        "take while { true }",
        "take until { false }",
        "where { true }",
        "get a",
        "select a",
        "select 0",
        "reject b",
        "each { $in }",
        "drop 1",
        "drop nth 0",
        "drop column 1",
        "compact",
        "find 1",
        "slice 0..1",
        "every 2",
        "uniq",
        "uniq-by a",
        "columns",
        "values",
    ] {
        for streamed in [false, true] {
            let metadata = Some(PipelineMetadata {
                data_source: DataSource::FilePath("input.json".into()),
                content_type: Some("application/json".into()),
            });
            let data = Value::test_list(vec![
                Value::test_record(
                    nu_protocol::record! {"a" => Value::test_int(1), "b" => Value::test_int(2)},
                ),
                Value::test_record(
                    nu_protocol::record! {"a" => Value::test_int(3), "b" => Value::test_int(4)},
                ),
            ]);
            let mut input = data.into_pipeline_data();
            if streamed {
                input = input.try_into_stream(&EngineState::new()).unwrap();
            }
            let output = evaluate(command, input.set_metadata(metadata));
            let meta = output.metadata().expect(command);
            assert_eq!(meta.content_type, None, "{command}, stream={streamed}");
            assert_eq!(
                meta.data_source,
                DataSource::FilePath("input.json".into()),
                "{command}"
            );
            output.into_value(Span::test_data()).unwrap();
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SaveValue(Value);

#[typetag::serde]
impl nu_protocol::CustomValue for SaveValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }
    fn type_name(&self) -> String {
        "save regression".into()
    }
    fn to_base_value(&self, _: Span) -> Result<Value, nu_protocol::ShellError> {
        Ok(self.0.clone())
    }
    fn save(
        &self,
        path: nu_protocol::Spanned<&std::path::Path>,
        _: Span,
        save_span: Span,
    ) -> Result<(), nu_protocol::ShellError> {
        std::fs::write(path.item, "custom save marker").map_err(|error| {
            nu_protocol::ShellError::Io(nu_protocol::shell_error::io::IoError::new(
                error,
                save_span,
                Some(path.item.into()),
            ))
        })
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[test]
fn streaming_save_custom_values() {
    let dir = tempfile::tempdir().unwrap();
    for suffix in ["txt", "json"] {
        let path = dir.path().join(format!("value.{suffix}"));
        evaluate(
            &format!("save '{}'", path.display()),
            Value::custom(Box::new(SaveValue(Value::test_int(42))), Span::test_data())
                .into_pipeline_data(),
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), "custom save marker");
    }
}

#[test]
fn streaming_table_page_size_controls_batches() {
    let result = evaluate(
        "$env.config.table.stream_page_size = 2; $env.config.table.batch_duration = 1hr; $in | table",
        Value::test_list((0..5).map(Value::test_int).collect()).into_pipeline_data(),
    );
    let PipelineData::ByteStream(stream, _) = result else {
        panic!("expected table byte stream")
    };
    let chunks: Vec<_> = stream.chunks().unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(chunks.len(), 3);
}

#[test]
fn streaming_table_batch_duration_controls_batches() {
    let result = evaluate(
        "$env.config.table.batch_duration = 0ns; $in | table",
        Value::test_list((0..3).map(Value::test_int).collect()).into_pipeline_data(),
    );
    let PipelineData::ByteStream(stream, _) = result else {
        panic!("expected table byte stream")
    };
    let chunks: Vec<_> = stream.chunks().unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(chunks.len(), 3);
}

#[test]
fn streaming_partial_binary_results_clear_content_type() {
    for command in ["first 1", "skip 1", "take 1", "bytes at 0..1"] {
        for streamed in [false, true] {
            let input = if streamed {
                PipelineData::byte_stream(
                    nu_protocol::ByteStream::read_binary(
                        vec![1, 2, 3],
                        Span::test_data(),
                        nu_protocol::Signals::empty(),
                    ),
                    None,
                )
            } else {
                Value::test_binary(vec![1, 2, 3]).into_pipeline_data()
            };
            let output = evaluate(
                command,
                input.set_metadata(Some(PipelineMetadata {
                    content_type: Some("application/octet-stream".into()),
                    ..Default::default()
                })),
            );
            assert_eq!(
                output.metadata().and_then(|m| m.content_type),
                None,
                "{command}, stream={streamed}"
            );
            output.into_value(Span::test_data()).unwrap();
        }
    }
}

#[derive(Clone)]
struct CountedStream(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl nu_protocol::engine::Command for CountedStream {
    fn name(&self) -> &str {
        "counted-stream"
    }
    fn signature(&self) -> nu_protocol::Signature {
        nu_protocol::Signature::build(self.name()).input_output_types(vec![(
            nu_protocol::Type::Nothing,
            nu_protocol::Type::list(nu_protocol::Type::Int),
        )])
    }
    fn description(&self) -> &str {
        "Count values pulled from a closure result."
    }
    fn run(
        &self,
        _: &EngineState,
        _: &mut Stack,
        call: &nu_protocol::engine::Call,
        _: PipelineData,
    ) -> Result<PipelineData, nu_protocol::ShellError> {
        use nu_protocol::IntoInterruptiblePipelineData;
        let pulls = self.0.clone();
        Ok((0..5)
            .map(move |value| {
                pulls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Value::test_int(value)
            })
            .into_pipeline_data(call.head, nu_protocol::Signals::empty()))
    }
}

#[test]
fn streaming_flatten_pulls_closure_results_on_demand() {
    let pulls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut engine = nu_cmd_lang::create_default_context();
    let mut working = StateWorkingSet::new(&engine);
    working.add_decl(Box::new(nu_command::Each));
    working.add_decl(Box::new(CountedStream(pulls.clone())));
    let block = nu_parser::parse(
        &mut working,
        None,
        b"[1 2] | each --flatten { counted-stream }",
        false,
    );
    assert!(
        working.parse_errors.is_empty(),
        "{:?}",
        working.parse_errors
    );
    engine.merge_delta(working.render()).unwrap();
    let output = nu_engine::eval_block::<nu_protocol::debugger::WithoutDebug>(
        &engine,
        &mut Stack::new(),
        &block,
        PipelineData::empty(),
    )
    .unwrap();
    assert_eq!(pulls.load(std::sync::atomic::Ordering::SeqCst), 0);
    let mut output = output.into_iter();
    assert_eq!(output.next().unwrap(), Value::test_int(0));
    assert_eq!(pulls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(output.next().unwrap(), Value::test_int(1));
    assert_eq!(pulls.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[test]
fn custom_save_takes_precedence_over_extension_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("value.special");
    evaluate(
        &format!(
            "let original = $in; def 'to special' [] {{ $in | describe }}; $original | save '{}'",
            path.display()
        ),
        Value::custom(Box::new(SaveValue(Value::test_int(42))), Span::test_data())
            .into_pipeline_data(),
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "custom save marker");
}

#[test]
fn partial_substrings_clear_content_type_including_cell_paths_and_const() {
    for (command, value) in [
        ("str substring 0..1", Value::test_string("hello")),
        (
            "str substring 0..1 text",
            Value::test_record(nu_protocol::record! {"text" => Value::test_string("hello")}),
        ),
        (
            "bytes at 0..1 data",
            Value::test_record(nu_protocol::record! {"data" => Value::test_binary(vec![1, 2, 3])}),
        ),
    ] {
        let output = evaluate(
            command,
            value
                .into_pipeline_data()
                .set_metadata(Some(PipelineMetadata {
                    content_type: Some("application/json".into()),
                    data_source: DataSource::FilePath("origin".into()),
                })),
        );
        let metadata = output.metadata().unwrap();
        assert_eq!(metadata.content_type, None, "{command}");
        assert_eq!(metadata.data_source, DataSource::FilePath("origin".into()));
    }
    let output = evaluate(
        "const part = ('hello' | str substring 0..1); $part",
        PipelineData::empty(),
    );
    assert_eq!(
        output.into_value(Span::test_data()).unwrap(),
        Value::test_string("he")
    );
}

#[test]
fn pipeline_constructors_accept_metadata_without_option_wrapper() {
    let span = Span::test_data();
    let metadata = PipelineMetadata {
        content_type: Some("text/plain".into()),
        ..Default::default()
    };
    let value = PipelineData::value(Value::test_int(1), metadata.clone());
    let list = PipelineData::list_stream(
        nu_protocol::ListStream::new(
            std::iter::once(Value::test_int(1)),
            span,
            nu_protocol::Signals::empty(),
        ),
        metadata.clone(),
    );
    let bytes = PipelineData::byte_stream(
        nu_protocol::ByteStream::read_binary(vec![1], span, nu_protocol::Signals::empty()),
        metadata.clone(),
    );
    for pipeline in [value, list, bytes] {
        assert_eq!(pipeline.metadata(), Some(metadata.clone()));
    }
}

#[test]
fn custom_save_respects_overwrite_and_append_flags() {
    let dir = tempfile::tempdir().unwrap();
    for flag in ["", "--force", "--append"] {
        let path = dir.path().join("existing.txt");
        std::fs::write(&path, "original contents").unwrap();
        let result = evaluate_result(
            &format!("save {flag} '{}'", path.display()),
            Value::custom(Box::new(SaveValue(Value::test_int(42))), Span::test_data())
                .into_pipeline_data(),
        );
        if flag == "--force" {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                "custom save marker"
            );
        } else {
            assert!(result.is_err(), "{flag} unexpectedly succeeded");
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                "original contents",
                "{flag}"
            );
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains(if flag.is_empty() {
                    "already exists"
                } else {
                    "append"
                }),
                "{error}"
            );
        }
    }
}
