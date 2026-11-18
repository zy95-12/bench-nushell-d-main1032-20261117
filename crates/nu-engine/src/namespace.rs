use crate::{command_prelude::*, get_full_help};

#[derive(Clone)]
pub struct Namespace(pub String);

impl Command for Namespace {
    fn name(&self) -> &str {
        &self.0
    }
    fn description(&self) -> &str {
        "Show help for the subcommands in this namespace."
    }
    fn extra_description(&self) -> &str {
        "Use a subcommand to perform an operation in this namespace."
    }
    fn signature(&self) -> Signature {
        Signature::build(self.name()).input_output_types(vec![(Type::Nothing, Type::String)])
    }
    fn run(
        &self,
        engine: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine, stack), call.head).into_pipeline_data())
    }
}

/// Register help commands for missing prefixes, preserving all real declarations.
pub fn add_namespace_commands(engine: &mut EngineState) -> Result<(), ShellError> {
    let commands = engine.find_commands_by_predicate(|_| true, false);
    let mut prefixes = std::collections::BTreeSet::new();
    for (_, name, _, _) in commands {
        for (index, byte) in name.iter().enumerate() {
            if *byte == b' ' && index > 0 {
                prefixes.insert(name[..index].to_vec());
            }
        }
    }
    let mut working_set = StateWorkingSet::new(engine);
    for prefix in prefixes {
        if working_set.find_decl(&prefix).is_none() {
            working_set.add_decl(Box::new(Namespace(
                String::from_utf8_lossy(&prefix).into_owned(),
            )));
        }
    }
    let delta = working_set.render();
    engine.merge_delta(delta)
}
