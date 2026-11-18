use indexmap::IndexMap;
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_engine::command_prelude::*;
use nu_protocol::{Config, ast::PathMember};
use std::collections::HashSet;

#[derive(Clone)]
pub struct ToMd;

impl Command for ToMd {
    fn name(&self) -> &str {
        "to md"
    }

    fn signature(&self) -> Signature {
        Signature::build("to md")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "escape-md",
                "Escape Markdown characters in cells.",
                Some('m'),
            )
            .switch("escape-html", "Escape HTML characters in cells.", Some('t'))
            .switch(
                "escape-all",
                "Escape Markdown and HTML characters in cells.",
                Some('a'),
            )
            .switch(
                "escape",
                "escape Markdown formatting characters and HTML in table cells",
                None,
            )
            .switch(
                "pretty",
                "Formats the Markdown table to vertically align items",
                Some('p'),
            )
            .switch(
                "per-element",
                "treat each row as markdown syntax element",
                Some('e'),
            )
            .named(
                "center",
                SyntaxShape::List(Box::new(SyntaxShape::CellPath)),
                "Formats the Markdown table to center given columns",
                Some('c'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into simple Markdown."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an MD string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to md",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1 | 2 |",
                )),
            },
            Example {
                description: "Optionally, output a formatted markdown string",
                example: "[[foo bar]; [1 2]] | to md --pretty",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1   | 2   |",
                )),
            },
            Example {
                description: "Treat each row as a markdown element",
                example: r#"[{"H1": "Welcome to Nushell" } [[foo bar]; [1 2]]] | to md --per-element --pretty"#,
                result: Some(Value::test_string(
                    "# Welcome to Nushell\n\n| foo | bar |\n| --- | --- |\n| 1   | 2   |",
                )),
            },
            Example {
                description: "Render a list",
                example: "[0 1 2] | to md --pretty",
                result: Some(Value::test_string("0\n1\n2")),
            },
            Example {
                description: "Separate list into markdown tables",
                example: "[ {foo: 1, bar: 2} {foo: 3, bar: 4} {foo: 5}] | to md --per-element",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n\n| foo |\n| --- |\n| 5 |",
                )),
            },
            Example {
                description: "Center a column of a markdown table",
                example: "[ {foo: 1, bar: 2} {foo: 3, bar: 4}] | to md --pretty --center [bar]",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- |:---:|\n| 1   |  2  |\n| 3   |  4  |",
                )),
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
        let pretty = call.has_flag(engine_state, stack, "pretty")?;
        let per_element = call.has_flag(engine_state, stack, "per-element")?;
        let center: Option<Vec<CellPath>> = call.get_flag(engine_state, stack, "center")?;
        let config = stack.get_config(engine_state);
        to_md(
            input,
            pretty,
            per_element,
            &center,
            &config,
            head,
            (
                call.has_flag(engine_state, stack, "escape-md")?
                    || call.has_flag(engine_state, stack, "escape-all")?
                    || call.has_flag(engine_state, stack, "escape")?,
                call.has_flag(engine_state, stack, "escape-html")?
                    || call.has_flag(engine_state, stack, "escape-all")?
                    || call.has_flag(engine_state, stack, "escape")?,
            ),
        )
    }
}

fn to_md(
    input: PipelineData,
    pretty: bool,
    per_element: bool,
    center: &Option<Vec<CellPath>>,
    config: &Config,
    head: Span,
    (escape_md, escape_html): (bool, bool),
) -> Result<PipelineData, ShellError> {
    // text/markdown became a valid mimetype with rfc7763
    let metadata = input
        .metadata()
        .unwrap_or_default()
        .with_content_type(Some("text/markdown".into()));

    let (grouped_input, single_list) = group_by(input, head, config);
    if per_element || single_list {
        let mut output = String::new();
        let mut previous_was_table = false;
        for val in grouped_input {
            let is_table = match &val {
                Value::Record { val, .. } => {
                    !(val.len() == 1
                        && val.columns().next().is_some_and(|name| {
                            matches!(
                                name.to_ascii_lowercase().as_str(),
                                "h1" | "h2" | "h3" | "blockquote"
                            )
                        }))
                }
                Value::List { vals, .. } => {
                    vals.iter().any(|val| matches!(val, Value::Record { .. }))
                }
                _ => false,
            };
            let fragment = match val {
                Value::List { .. } => table(
                    val.into_pipeline_data(),
                    pretty,
                    center,
                    escape_md,
                    escape_html,
                    config,
                ),
                other => fragment(other, pretty, center, escape_md, escape_html, config),
            };
            if fragment.is_empty() {
                continue;
            }
            if !output.is_empty() {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
                // Tables need their own blocks, including when the previous
                // fragment is a table's last row or a blockquote.
                if (previous_was_table || is_table) && !output.ends_with("\n\n") {
                    output.push('\n');
                }
            }
            output.push_str(&fragment);
            previous_was_table = is_table;
        }
        return Ok(
            Value::string(output.trim(), head).into_pipeline_data_with_metadata(Some(metadata))
        );
    }
    Ok(Value::string(
        table(
            grouped_input,
            pretty,
            center,
            escape_md,
            escape_html,
            config,
        ),
        head,
    )
    .into_pipeline_data_with_metadata(Some(metadata)))
}

fn fragment(
    input: Value,
    pretty: bool,
    center: &Option<Vec<CellPath>>,
    escape_md: bool,
    escape_html: bool,
    config: &Config,
) -> String {
    let mut out = String::new();

    if let Value::Record { val, .. } = &input {
        match val.get_index(0) {
            Some((header, data)) if val.len() == 1 => {
                let markup = match header.to_ascii_lowercase().as_ref() {
                    "h1" => "# ".to_string(),
                    "h2" => "## ".to_string(),
                    "h3" => "### ".to_string(),
                    "blockquote" => "> ".to_string(),
                    _ => {
                        return table(
                            input.into_pipeline_data(),
                            pretty,
                            center,
                            escape_md,
                            escape_html,
                            config,
                        );
                    }
                };

                out.push_str(&markup);
                out.push_str(&data.to_expanded_string("|", config));
            }
            _ => {
                out = table(
                    input.into_pipeline_data(),
                    pretty,
                    center,
                    escape_md,
                    escape_html,
                    config,
                )
            }
        }
    } else {
        out = input.to_expanded_string("|", config)
    }

    out.push('\n');
    out
}

fn collect_headers(
    headers: &[String],
    escape_md: bool,
    escape_html: bool,
) -> (Vec<String>, Vec<usize>) {
    let mut escaped_headers: Vec<String> = Vec::new();
    let mut column_widths: Vec<usize> = Vec::new();

    if !headers.is_empty() {
        for header in headers {
            let escaped_header_string = escape_cell(header, escape_md, escape_html);
            column_widths.push(escaped_header_string.len().max(3));
            escaped_headers.push(escaped_header_string);
        }
    } else {
        column_widths = vec![3; headers.len()]
    }

    (escaped_headers, column_widths)
}

fn escape_cell(text: &str, escape: bool, html: bool) -> String {
    let text = if html {
        v_htmlescape::escape(text).to_string()
    } else {
        text.to_owned()
    };
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\r' => {}
            '\n' => out.push_str("<br>"),
            '|' => out.push_str("\\|"),
            '\\' => out.push_str("\\\\"),
            '*' | '_' | '[' | ']' | '`' | '<' | '>' | '(' | ')' | '#' if escape => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn table(
    input: PipelineData,
    pretty: bool,
    center: &Option<Vec<CellPath>>,
    escape_md: bool,
    escape_html: bool,
    config: &Config,
) -> String {
    let vec_of_values = input
        .into_iter()
        .flat_map(|val| match val {
            Value::List { vals, .. } => vals,
            other => vec![other],
        })
        .collect::<Vec<Value>>();
    let mut headers = merge_descriptors(&vec_of_values);

    for value in &vec_of_values {
        if let Value::Record { val, .. } = value
            && let Some(empty_index) = val.columns().position(String::is_empty)
        {
            // Anchor to this record's neighbors in the merged header order.
            // Earlier records may repeat columns or contain additional columns.
            let insertion_index = val
                .columns()
                .take(empty_index)
                .last()
                .and_then(|column| headers.iter().position(|header| header == column))
                .map(|index| index + 1)
                .or_else(|| {
                    val.columns()
                        .nth(empty_index + 1)
                        .and_then(|column| headers.iter().position(|header| header == column))
                })
                .unwrap_or(0);
            headers.insert(insertion_index.min(headers.len()), String::new());
            break;
        }
    }

    let (escaped_headers, mut column_widths) = collect_headers(&headers, escape_md, escape_html);

    let mut escaped_rows: Vec<Vec<String>> = Vec::new();

    for row in vec_of_values {
        let mut escaped_row: Vec<String> = Vec::new();
        let span = row.span();

        match row.to_owned() {
            Value::Record { val: row, .. } => {
                for i in 0..headers.len() {
                    let value_string = row
                        .get(&headers[i])
                        .cloned()
                        .unwrap_or_else(|| Value::nothing(span))
                        .to_expanded_string(", ", config);
                    let value_string = escape_cell(&value_string, escape_md, escape_html);
                    let new_column_width = value_string.len();

                    escaped_row.push(value_string);

                    if column_widths[i] < new_column_width {
                        column_widths[i] = new_column_width;
                    }
                }
            }
            p => {
                let value_string =
                    escape_cell(&p.to_abbreviated_string(config), escape_md, escape_html);
                escaped_row.push(value_string);
            }
        }

        escaped_rows.push(escaped_row);
    }

    if (column_widths.is_empty() || column_widths.iter().all(|x| *x == 0))
        && escaped_rows.is_empty()
    {
        String::from("")
    } else {
        get_output_string(
            &headers,
            &escaped_headers,
            &escaped_rows,
            &column_widths,
            pretty,
            center,
        )
        .trim()
        .to_string()
    }
}

pub fn group_by(values: PipelineData, head: Span, config: &Config) -> (PipelineData, bool) {
    // Keep column boundaries and distinguish record schemas from scalar text.
    #[derive(Hash, PartialEq, Eq)]
    enum GroupKey {
        Columns(Vec<String>),
        Value(String),
    }
    let mut lists = IndexMap::new();
    let mut single_list = false;
    for val in values {
        if let Value::Record {
            val: ref record, ..
        } = val
        {
            lists
                .entry(GroupKey::Columns(record.columns().cloned().collect()))
                .and_modify(|v: &mut Vec<Value>| v.push(val.clone()))
                .or_insert_with(|| vec![val.clone()]);
        } else {
            lists
                .entry(GroupKey::Value(val.to_expanded_string(",", config)))
                .and_modify(|v: &mut Vec<Value>| v.push(val.clone()))
                .or_insert_with(|| vec![val.clone()]);
        }
    }
    let mut output = vec![];
    for (_, mut value) in lists {
        if value.len() == 1 {
            output.push(value.pop().unwrap_or_else(|| Value::nothing(head)))
        } else {
            output.push(Value::list(value.to_vec(), head))
        }
    }
    if output.len() == 1 {
        single_list = true;
    }
    (Value::list(output, head).into_pipeline_data(), single_list)
}

fn get_output_string(
    headers: &[String],
    escaped_headers: &[String],
    rows: &[Vec<String>],
    column_widths: &[usize],
    pretty: bool,
    center: &Option<Vec<CellPath>>,
) -> String {
    let mut output_string = String::new();

    let mut to_center: HashSet<String> = HashSet::new();
    if let Some(center_vec) = center.as_ref() {
        for cell_path in center_vec {
            if let Some(PathMember::String { val, .. }) = cell_path
                .members
                .iter()
                .find(|member| matches!(member, PathMember::String { .. }))
            {
                to_center.insert(val.clone());
            }
        }
    }

    if !headers.is_empty() {
        output_string.push('|');

        for i in 0..headers.len() {
            if pretty {
                output_string.push(' ');
                if center.is_some() && to_center.contains(&headers[i]) {
                    output_string.push_str(&get_centered_string(
                        escaped_headers[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                } else {
                    output_string.push_str(&get_padded_string(
                        escaped_headers[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                }
                output_string.push(' ');
            } else {
                output_string.push(' ');
                output_string.push_str(&escaped_headers[i]);
                output_string.push(' ');
            }

            output_string.push('|');
        }

        output_string.push_str("\n|");

        for i in 0..headers.len() {
            let centered_column = center.is_some() && to_center.contains(&headers[i]);
            let border_char = if centered_column { ':' } else { ' ' };
            if pretty {
                output_string.push(border_char);
                output_string.push_str(&get_padded_string(
                    String::from("-"),
                    column_widths[i],
                    '-',
                ));
                output_string.push(border_char);
            } else if centered_column {
                output_string.push(':');
                output_string.push_str("---");
                output_string.push(':');
            } else {
                output_string.push_str(" --- ");
            }

            output_string.push('|');
        }

        output_string.push('\n');
    }

    for row in rows {
        if !headers.is_empty() {
            output_string.push('|');
        }

        for i in 0..row.len() {
            if pretty && column_widths.get(i).is_some() {
                output_string.push(' ');
                if center.is_some() && to_center.contains(&headers[i]) {
                    output_string.push_str(&get_centered_string(
                        row[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                } else {
                    output_string.push_str(&get_padded_string(
                        row[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                }
                output_string.push(' ');
            } else {
                if !headers.is_empty() {
                    output_string.push(' ');
                }
                output_string.push_str(&row[i]);
                if !headers.is_empty() {
                    output_string.push(' ');
                }
            }

            if !headers.is_empty() {
                output_string.push('|');
            }
        }

        output_string.push('\n');
    }

    output_string
}

fn get_centered_string(text: String, desired_length: usize, padding_character: char) -> String {
    let total_padding = if text.len() > desired_length {
        0
    } else {
        desired_length - text.len()
    };

    let repeat_left = total_padding / 2;
    let repeat_right = total_padding - repeat_left;

    format!(
        "{}{}{}",
        padding_character.to_string().repeat(repeat_left),
        text,
        padding_character.to_string().repeat(repeat_right)
    )
}

fn get_padded_string(text: String, desired_length: usize, padding_character: char) -> String {
    let repeat_length = if text.len() > desired_length {
        0
    } else {
        desired_length - text.len()
    };

    format!(
        "{}{}",
        text,
        padding_character.to_string().repeat(repeat_length)
    )
}

#[cfg(test)]
mod tests {
    use crate::{Get, Metadata};

    use super::*;
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;
    use nu_protocol::{Config, IntoPipelineData, Value, casing::Casing, record};

    fn one(string: &str) -> String {
        string
            .lines()
            .skip(1)
            .map(|line| line.trim())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim_end()
            .to_string()
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToMd {})
    }

    #[test]
    fn render_h1() {
        let value = Value::test_record(record! {
            "H1" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "# Ecuador\n"
        );
    }

    #[test]
    fn render_h2() {
        let value = Value::test_record(record! {
            "H2" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "## Ecuador\n"
        );
    }

    #[test]
    fn render_h3() {
        let value = Value::test_record(record! {
            "H3" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "### Ecuador\n"
        );
    }

    #[test]
    fn render_blockquote() {
        let value = Value::test_record(record! {
            "BLOCKQUOTE" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "> Ecuador\n"
        );
    }

    #[test]
    fn render_table() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "country" => Value::test_string("Ecuador"),
            }),
            Value::test_record(record! {
                "country" => Value::test_string("New Zealand"),
            }),
            Value::test_record(record! {
                "country" => Value::test_string("USA"),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | country |
            | --- |
            | Ecuador |
            | New Zealand |
            | USA |
        "#)
        );

        assert_eq!(
            table(
                value.into_pipeline_data(),
                true,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | country     |
            | ----------- |
            | Ecuador     |
            | New Zealand |
            | USA         |
        "#)
        );
    }

    #[test]
    fn test_empty_column_header() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "" => Value::test_string("1"),
                "foo" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "" => Value::test_string("3"),
                "foo" => Value::test_string("4"),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            |  | foo |
            | --- | --- |
            | 1 | 2 |
            | 3 | 4 |
        "#)
        );
    }

    #[test]
    fn empty_header_first_appears_in_later_record() {
        let input = Value::test_list(vec![
            Value::test_record(record! {"a" => Value::test_int(1), "b" => Value::test_int(2)}),
            Value::test_record(record! {"a" => Value::test_int(3), "b" => Value::test_int(4)}),
            Value::test_record(
                record! {"a" => Value::test_int(5), "" => Value::test_int(6), "b" => Value::test_int(7)},
            ),
        ]);
        assert_eq!(
            table(
                input.into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            "| a |  | b |\n| --- | --- | --- |\n| 1 |  | 2 |\n| 3 |  | 4 |\n| 5 | 6 | 7 |"
        );
    }

    #[test]
    fn empty_header_uses_known_column_order_and_stays_in_place() {
        let input = Value::test_list(vec![
            Value::test_record(
                record! {"x" => Value::test_int(1), "a" => Value::test_int(2), "b" => Value::test_int(3)},
            ),
            Value::test_record(
                record! {"a" => Value::test_int(4), "" => Value::test_int(5), "b" => Value::test_int(6)},
            ),
            Value::test_record(
                record! {"" => Value::test_int(7), "b" => Value::test_int(8), "x" => Value::test_int(9)},
            ),
        ]);
        assert_eq!(
            table(
                input.into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            "| x | a |  | b |\n| --- | --- | --- | --- |\n| 1 | 2 |  | 3 |\n|  | 4 | 5 | 6 |\n| 9 |  | 7 | 8 |"
        );
    }

    #[test]
    fn test_empty_row_value() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "foo" => Value::test_string("1"),
                "bar" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("3"),
                "bar" => Value::test_string("4"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("5"),
                "bar" => Value::test_string(""),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- | --- |
            | 1 | 2 |
            | 3 | 4 |
            | 5 |  |
        "#)
        );
    }

    #[test]
    fn test_center_column() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "foo" => Value::test_string("1"),
                "bar" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("3"),
                "bar" => Value::test_string("4"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("5"),
                "bar" => Value::test_string("6"),
            }),
        ]);

        let center_columns = Value::test_list(vec![Value::test_cell_path(CellPath {
            members: vec![PathMember::test_string(
                "bar".into(),
                false,
                Casing::Sensitive,
            )],
        })]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        // With pretty
        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- |:---:|
            | 1   |  2  |
            | 3   |  4  |
            | 5   |  6  |
        "#)
        );

        // Without pretty
        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- |:---:|
            | 1 | 2 |
            | 3 | 4 |
            | 5 | 6 |
        "#)
        );
    }

    #[test]
    fn test_empty_center_column() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "foo" => Value::test_string("1"),
                "bar" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("3"),
                "bar" => Value::test_string("4"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("5"),
                "bar" => Value::test_string("6"),
            }),
        ]);

        let center: Option<Vec<CellPath>> = Some(vec![]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- | --- |
            | 1   | 2   |
            | 3   | 4   |
            | 5   | 6   |
        "#)
        );
    }

    #[test]
    fn test_center_multiple_columns() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "command" => Value::test_string("ls"),
                "input" => Value::test_string("."),
                "output" => Value::test_string("file.txt"),
            }),
            Value::test_record(record! {
                "command" => Value::test_string("echo"),
                "input" => Value::test_string("'hi'"),
                "output" => Value::test_string("hi"),
            }),
            Value::test_record(record! {
                "command" => Value::test_string("cp"),
                "input" => Value::test_string("a.txt"),
                "output" => Value::test_string("b.txt"),
            }),
        ]);

        let center_columns = Value::test_list(vec![
            Value::test_cell_path(CellPath {
                members: vec![PathMember::test_string(
                    "command".into(),
                    false,
                    Casing::Sensitive,
                )],
            }),
            Value::test_cell_path(CellPath {
                members: vec![PathMember::test_string(
                    "output".into(),
                    false,
                    Casing::Sensitive,
                )],
            }),
        ]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | command | input |  output  |
            |:-------:| ----- |:--------:|
            |   ls    | .     | file.txt |
            |  echo   | 'hi'  |    hi    |
            |   cp    | a.txt |  b.txt   |
        "#)
        );
    }

    #[test]
    fn test_center_non_existing_column() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "name" => Value::test_string("Alice"),
                "age" => Value::test_string("30"),
            }),
            Value::test_record(record! {
                "name" => Value::test_string("Bob"),
                "age" => Value::test_string("5"),
            }),
            Value::test_record(record! {
                "name" => Value::test_string("Charlie"),
                "age" => Value::test_string("20"),
            }),
        ]);

        let center_columns = Value::test_list(vec![Value::test_cell_path(CellPath {
            members: vec![PathMember::test_string(
                "none".into(),
                false,
                Casing::Sensitive,
            )],
        })]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | name    | age |
            | ------- | --- |
            | Alice   | 30  |
            | Bob     | 5   |
            | Charlie | 20  |
        "#)
        );
    }

    #[test]
    fn test_center_complex_cell_path() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "k" => Value::test_string("version"),
                "v" => Value::test_string("0.104.1"),
            }),
            Value::test_record(record! {
                "k" => Value::test_string("build_time"),
                "v" => Value::test_string("2025-05-28 11:00:45 +01:00"),
            }),
        ]);

        let center_columns = Value::test_list(vec![Value::test_cell_path(CellPath {
            members: vec![
                PathMember::test_int(1, false),
                PathMember::test_string("v".into(), false, Casing::Sensitive),
            ],
        })]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | k          |             v              |
            | ---------- |:--------------------------:|
            | version    |          0.104.1           |
            | build_time | 2025-05-28 11:00:45 +01:00 |
        "#)
        );
    }

    #[test]
    fn center_escaped_column_names() {
        for (name, escape_md, escape_html, rendered) in [
            ("a|b", false, false, r"a\|b"),
            ("a*b", true, false, r"a\*b"),
            ("a&b", false, true, "a&amp;b"),
            ("a<b", true, true, "a&lt;b"),
        ] {
            let center = Some(vec![CellPath {
                members: vec![PathMember::test_string(
                    name.into(),
                    false,
                    Casing::Sensitive,
                )],
            }]);
            for pretty in [false, true] {
                let output = table(
                    Value::test_record(record! {name => Value::test_int(1)}).into_pipeline_data(),
                    pretty,
                    &center,
                    escape_md,
                    escape_html,
                    &Config::default(),
                );
                assert!(
                    output.lines().next().unwrap().contains(rendered),
                    "{output}"
                );
                let dashes = if pretty { rendered.len().max(3) } else { 3 };
                assert_eq!(
                    output.lines().nth(1).unwrap(),
                    format!("|:{}:|", "-".repeat(dashes))
                );
            }
        }
    }

    #[test]
    fn per_element_keeps_distinct_column_groups() {
        let input = Value::test_list(vec![
            Value::test_record(record! {"a" => Value::test_int(1), "bc" => Value::test_int(2)}),
            Value::test_record(record! {"ab" => Value::test_int(3), "c" => Value::test_int(4)}),
        ]);
        let output = to_md(
            input.into_pipeline_data(),
            false,
            true,
            &None,
            &Config::default(),
            Span::test_data(),
            (false, false),
        )
        .unwrap()
        .into_value(Span::test_data())
        .unwrap();
        assert_eq!(
            output.as_str().unwrap(),
            "| a | bc |\n| --- | --- |\n| 1 | 2 |\n\n| ab | c |\n| --- | --- |\n| 3 | 4 |"
        );
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let state_delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToMd {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };
        let delta = state_delta;

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to md  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("text/markdown"),
            result.expect("There should be a result")
        );
    }
}
