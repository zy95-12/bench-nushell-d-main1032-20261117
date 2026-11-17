use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_path::expand_to_real_path;
use nu_protocol::engine::StateWorkingSet;
use std::path::{Component, Path, PathBuf};

struct Arguments {
    path: Spanned<String>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathRelativeTo;

impl Command for PathRelativeTo {
    fn name(&self) -> &str {
        "path relative-to"
    }

    fn signature(&self) -> Signature {
        Signature::build("path relative-to")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .required(
                "path",
                SyntaxShape::String,
                "Parent shared with the input path.",
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Express a path as relative to another path."
    }

    fn extra_description(&self) -> &str {
        r#"Can be used only when the input and the argument paths are either both
absolute or both relative. The argument path needs to be a parent of the input
path."#
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            path: call.req(engine_state, stack, 0)?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&relative_to, &args, value, head),
            engine_state.signals(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            path: call.req_const(working_set, 0)?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&relative_to, &args, value, head),
            working_set.permanent().signals(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths",
                example: r"'C:\Users\viking' | path relative-to 'C:\Users'",
                result: Some(Value::test_string(r"viking")),
            },
            Example {
                description: "Find a relative path from absolute paths in list",
                example: r"[ C:\Users\viking, C:\Users\spam ] | path relative-to C:\Users",
                result: Some(Value::test_list(vec![
                    Value::test_string("viking"),
                    Value::test_string("spam"),
                ])),
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs\bacon\sausage\spam' | path relative-to 'eggs\bacon\sausage'",
                result: Some(Value::test_string(r"spam")),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Find a relative path from two absolute paths",
                example: r"'/home/viking' | path relative-to '/home'",
                result: Some(Value::test_string(r"viking")),
            },
            Example {
                description: "Find a relative path from absolute paths in list",
                example: r"[ /home/viking, /home/spam ] | path relative-to '/home'",
                result: Some(Value::test_list(vec![
                    Value::test_string("viking"),
                    Value::test_string("spam"),
                ])),
            },
            Example {
                description: "Find a relative path from two relative paths",
                example: r"'eggs/bacon/sausage/spam' | path relative-to 'eggs/bacon/sausage'",
                result: Some(Value::test_string(r"spam")),
            },
        ]
    }
}

fn relative_to(path: &Path, span: Span, args: &Arguments) -> Value {
    let lhs = expand_to_real_path(path);
    let rhs = expand_to_real_path(&args.path.item);
    match lhs.strip_prefix(&rhs) {
        Ok(p) => Value::string(p.to_string_lossy(), span),
        Err(e) => {
            if is_case_insensitive_filesystem() {
                if let Some(path) = try_case_insensitive_strip_prefix(&lhs, &rhs) {
                    return Value::string(path.to_string_lossy(), span);
                }
            }
            Value::error(
                ShellError::CantConvert {
                    to_type: e.to_string(),
                    from_type: "string".into(),
                    span,
                    help: None,
                },
                span,
            )
        }
    }
}

fn is_case_insensitive_filesystem() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

fn try_case_insensitive_strip_prefix(lhs: &Path, rhs: &Path) -> Option<PathBuf> {
    let mut lhs_components = lhs.components();
    for rhs_component in rhs.components() {
        let lhs_component = lhs_components.next()?;
        let equal = match (lhs_component, rhs_component) {
            (Component::Normal(a), Component::Normal(b)) => {
                a.to_str()?.to_lowercase() == b.to_str()?.to_lowercase()
            }
            (a, b) => a == b,
        };
        if !equal {
            return None;
        }
    }
    Some(lhs_components.as_path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PathRelativeTo {})
    }
}

#[cfg(test)]
mod predictable_prefix_tests {
    use super::*;
    #[test]
    fn predictable_insensitive_components_preserve_suffix() {
        assert_eq!(
            try_case_insensitive_strip_prefix(
                Path::new("Ärea/Child/LeAf"),
                Path::new("ärea/child")
            ),
            Some(PathBuf::from("LeAf"))
        );
        assert_eq!(
            try_case_insensitive_strip_prefix(Path::new("parent/child"), Path::new("PARENT/CHILD")),
            Some(PathBuf::new())
        );
        assert_eq!(
            try_case_insensitive_strip_prefix(
                Path::new("parent/childish"),
                Path::new("PARENT/child")
            ),
            None
        );
        assert_eq!(
            try_case_insensitive_strip_prefix(Path::new("parent"), Path::new("parent/child")),
            None
        );
        assert_eq!(
            try_case_insensitive_strip_prefix(Path::new("/parent"), Path::new("parent")),
            None
        );
        assert_eq!(
            try_case_insensitive_strip_prefix(Path::new("parent"), Path::new("../parent")),
            None
        );
    }
}
