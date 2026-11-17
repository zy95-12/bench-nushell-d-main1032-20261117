use super::utils::chain_error_with_input;
use nu_engine::{ClosureEval, ClosureEvalOnce, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Each;

impl Command for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn description(&self) -> &str {
        "Run a closure on each row of the input list, creating a new list with the results."
    }

    fn extra_description(&self) -> &str {
        r#"Since tables are lists of records, passing a table into 'each' will
iterate over each record, not necessarily each cell within it.

Avoid passing single records to this command. Since a record is a
one-row structure, 'each' will only run once, behaving similar to 'do'.
To iterate over a record's values, use 'items' or try converting it to a table
with 'transpose' first."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["for", "loop", "iterate", "map"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("each")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::List(Box::new(Type::Any))),
                (Type::Any, Type::Any),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
            )
            .switch(
                "flatten",
                "stream closure results into a single list",
                Some('f'),
            )
            .switch("keep-empty", "keep empty result cells", Some('k'))
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3] | each {|e| 2 * $e }",
                description: "Multiplies elements in the list",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(4),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: "{major:2, minor:1, patch:4} | values | each {|| into string }",
                description: "Produce a list of values in the record, converted to string",
                result: Some(Value::test_list(vec![
                    Value::test_string("2"),
                    Value::test_string("1"),
                    Value::test_string("4"),
                ])),
            },
            Example {
                example: r#"[1 2 3 2] | each {|e| if $e == 2 { "two" } }"#,
                description: "'null' items will be dropped from the result list. It has the same effect as 'filter_map' in other languages.",
                result: Some(Value::test_list(vec![
                    Value::test_string("two"),
                    Value::test_string("two"),
                ])),
            },
            Example {
                example: r#"[1 2 3] | enumerate | each {|e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description: "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(Value::test_list(vec![Value::test_string("found 2 at 1!")])),
            },
            Example {
                example: r#"[1 2 3] | each --keep-empty {|e| if $e == 2 { "found 2!"} }"#,
                description: "Iterate over each element, keeping null results",
                result: Some(Value::test_list(vec![
                    Value::nothing(Span::test_data()),
                    Value::test_string("found 2!"),
                    Value::nothing(Span::test_data()),
                ])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let keep_empty = call.has_flag(engine_state, stack, "keep-empty")?;

        let metadata = input.metadata().map(|m| m.with_content_type(None));
        let flatten = call.has_flag(engine_state, stack, "flatten")?;
        match input {
            PipelineData::Empty | PipelineData::Value(Value::Nothing { .. }, _) => {
                Ok(PipelineData::empty())
            }
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream(..) => {
                let mut closure = ClosureEval::new(engine_state, stack, closure);
                Ok(input
                    .into_iter()
                    .flat_map(move |value| each_result(&mut closure, value, head, flatten))
                    .into_pipeline_data(head, engine_state.signals().clone()))
            }
            PipelineData::ByteStream(stream, ..) => {
                if let Some(chunks) = stream.chunks() {
                    let mut closure = ClosureEval::new(engine_state, stack, closure);
                    Ok(chunks
                        .flat_map(move |value| match value {
                            Ok(value) => each_result(&mut closure, value, head, flatten),
                            Err(err) => {
                                PipelineData::value(Value::error(err, head), None).into_iter()
                            }
                        })
                        .into_pipeline_data(head, engine_state.signals().clone()))
                } else {
                    Ok(PipelineData::empty())
                }
            }
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(value @ Value::Error { .. }, _) => Ok(value.into_pipeline_data()),
            PipelineData::Value(value, ..) => {
                ClosureEvalOnce::new(engine_state, stack, closure).run_with_value(value)
            }
        }
        .and_then(|x| {
            x.filter(
                move |x| if !keep_empty { !x.is_nothing() } else { true },
                engine_state.signals(),
            )
        })
        .map(|data| data.set_metadata(metadata))
    }
}

// Preserve incoming errors even when a closure ignores its argument. Otherwise a nested
// iterator can silently turn an upstream failure into a successful value.
fn each_result(
    closure: &mut ClosureEval,
    value: Value,
    head: Span,
    flatten: bool,
) -> nu_protocol::PipelineIterator {
    let span = value.span();
    if value.is_error() {
        return PipelineData::value(value, None).into_iter();
    }
    let result = match closure.run_with_value(value) {
        Ok(data) if flatten => return data.into_iter(),
        Ok(PipelineData::ListStream(stream, _)) => {
            let mut values = Vec::new();
            for value in stream {
                if value.is_error() {
                    return PipelineData::value(value, None).into_iter();
                }
                values.push(value);
            }
            Ok(Value::list(values, span))
        }
        Ok(data) => data.into_value(head),
        Err(err) => Err(err),
    };
    std::iter::once(
        result.unwrap_or_else(|err| Value::error(chain_error_with_input(err, false, span), span)),
    )
    .into_pipeline_data(head, nu_protocol::Signals::empty())
    .into_iter()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Each {})
    }
}
