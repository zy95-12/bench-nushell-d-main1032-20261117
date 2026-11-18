use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct OverlayList;

impl Command for OverlayList {
    fn name(&self) -> &str {
        "overlay list"
    }

    fn description(&self) -> &str {
        "List all overlays and their active status."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("overlay list")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::table())])
    }

    fn extra_description(&self) -> &str {
        "Hidden overlays are listed first, sorted by name, followed by active overlays in activation order. The most recently activated overlay is last."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut hidden: Vec<String> = engine_state
            .scope
            .overlays
            .iter()
            .map(|(name, _)| String::from_utf8_lossy(name).into_owned())
            .filter(|name| !stack.active_overlays.contains(name))
            .collect();
        hidden.sort();
        hidden.dedup();
        let names = hidden
            .into_iter()
            .chain(stack.active_overlays.iter().cloned());
        let rows = names
            .map(|name| {
                let active = stack.active_overlays.contains(&name);
                Value::record(
                    record! {
                        "name" => Value::string(name, call.head),
                        "active" => Value::bool(active, call.head),
                    },
                    call.head,
                )
            })
            .collect();
        Ok(Value::list(rows, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Find an active overlay",
            example: r#"module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay list | where name == spam | get 0.name"#,
            result: Some(Value::test_string("spam")),
        }]
    }
}
