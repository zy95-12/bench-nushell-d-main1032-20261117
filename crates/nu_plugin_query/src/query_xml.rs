use crate::Query;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Record, Signature, Span, Spanned, SyntaxShape, Value};
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

pub struct QueryXml;

impl SimplePluginCommand for QueryXml {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query xml"
    }

    fn description(&self) -> &str {
        "execute xpath query on xml"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("query", SyntaxShape::String, "xpath query")
            .named(
                "namespaces",
                SyntaxShape::Record(vec![]),
                "map of prefixes to namespace URIs",
                Some('n'),
            )
            .switch("output-string-value", "include node string values", None)
            .switch("output-type", "include node types", None)
            .switch("output-names", "include node names", None)
            .category(Category::Filters)
    }

    fn run(
        &self,
        _plugin: &Query,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let query: Option<Spanned<String>> = call.opt(0)?;
        let namespaces: Option<Record> = call.get_flag::<Record>("namespaces")?;

        execute_xpath_query(call, input, query, namespaces)
    }
}

pub fn execute_xpath_query(
    call: &EvaluatedCall,
    input: &Value,
    query: Option<Spanned<String>>,
    namespaces: Option<Record>,
) -> Result<Value, LabeledError> {
    let (query_string, span) = match &query {
        Some(v) => (&v.item, v.span),
        None => {
            return Err(
                LabeledError::new("problem with input data").with_label("query missing", call.head)
            );
        }
    };

    let xpath = build_xpath(query_string, span)?;
    let input_string = input.coerce_str()?;
    let package = parser::parse(&input_string);

    if let Err(err) = package {
        return Err(
            LabeledError::new("Invalid XML document").with_label(err.to_string(), input.span())
        );
    }

    let package = package.expect("invalid xml document");

    let document = package.as_document();
    let mut context = Context::new();

    for (prefix, uri) in namespaces.unwrap_or_default().into_iter() {
        context.set_namespace(prefix.as_str(), uri.into_string()?.as_str());
    }

    context.set_namespace("xml", "http://www.w3.org/XML/1998/namespace");

    let output_type = call.has_flag("output-type")?;
    let output_names = call.has_flag("output-names")?;
    let output_string_value =
        call.has_flag("output-string-value")? || !(output_type || output_names);

    let result = xpath.evaluate(&context, document.root()).map_err(|err| {
        LabeledError::new("xpath query error").with_label(err.to_string(), call.head)
    })?;

    Ok(match result {
        sxd_xpath::Value::Boolean(value) => Value::bool(value, call.head),
        sxd_xpath::Value::Number(value) => Value::float(value, call.head),
        sxd_xpath::Value::String(value) => Value::string(value, call.head),
        sxd_xpath::Value::Nodeset(nodes) => Value::list(
            nodes
                .document_order()
                .into_iter()
                .map(|node| {
                    let mut record = Record::new();
                    if output_string_value {
                        record.push(
                            "string_value",
                            Value::string(node.string_value(), call.head),
                        );
                    }
                    if output_type {
                        use sxd_xpath::nodeset::Node;
                        let kind = match node {
                            Node::Element(_) => "element",
                            Node::Attribute(_) => "attribute",
                            Node::Text(_) => "text",
                            Node::Comment(_) => "comment",
                            Node::ProcessingInstruction(_) => "processing_instruction",
                            Node::Root(_) => "root",
                            Node::Namespace(_) => "namespace",
                        };
                        record.push("type", Value::string(kind, call.head));
                    }
                    if output_names {
                        let name = node.expanded_name();
                        let optional_string = |value: Option<&str>| {
                            value
                                .map(|value| Value::string(value, call.head))
                                .unwrap_or_else(|| Value::nothing(call.head))
                        };
                        record.push(
                            "local_name",
                            optional_string(name.map(|name| name.local_part())),
                        );
                        record.push(
                            "namespace",
                            optional_string(name.and_then(|name| name.namespace_uri())),
                        );
                        record.push(
                            "prefixed_name",
                            optional_string(node.prefixed_name().as_deref()),
                        );
                    }
                    Value::record(record, call.head)
                })
                .collect(),
            call.head,
        ),
    })
}

fn build_xpath(xpath_str: &str, span: Span) -> Result<sxd_xpath::XPath, LabeledError> {
    let factory = Factory::new();

    match factory.build(xpath_str) {
        Ok(xpath) => xpath.ok_or_else(|| {
            LabeledError::new("invalid xpath query").with_label("the query must not be empty", span)
        }),
        Err(err) => Err(LabeledError::new("invalid xpath query").with_label(err.to_string(), span)),
    }
}

#[cfg(test)]
mod tests {
    use super::execute_xpath_query as query;
    use nu_plugin::EvaluatedCall;
    use nu_protocol::{Span, Spanned, Value, record};

    #[test]
    fn position_function_in_predicate() {
        let call = EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        };

        let text = Value::string(
            r#"<?xml version="1.0" encoding="UTF-8"?><a><b/><b/></a>"#,
            Span::test_data(),
        );

        let spanned_str: Spanned<String> = Spanned {
            item: "count(//a/*[position() = 2])".to_string(),
            span: Span::test_data(),
        };

        let actual = query(&call, &text, Some(spanned_str), None).expect("test should not fail");

        let expected = Value::test_float(1.0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn functions_implicitly_coerce_argument_types() {
        let call = EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        };

        let text = Value::string(
            r#"<?xml version="1.0" encoding="UTF-8"?><a>true</a>"#,
            Span::test_data(),
        );

        let spanned_str: Spanned<String> = Spanned {
            item: "count(//*[contains(., true)])".to_string(),
            span: Span::test_data(),
        };

        let actual = query(&call, &text, Some(spanned_str), None).expect("test should not fail");

        let expected = Value::test_float(1.0);

        assert_eq!(actual, expected);
    }

    #[test]
    fn namespaces_are_used() {
        let call = EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        };

        // document uses `dp` ("document prefix") as a prefix
        let text = Value::string(
            r#"<?xml version="1.0" encoding="UTF-8"?><a xmlns:dp="http://example.com/ns"><dp:b>yay</dp:b></a>"#,
            Span::test_data(),
        );

        // but query uses `qp` ("query prefix") as a prefix
        let namespaces = record! {
            "qp" => Value::string("http://example.com/ns", Span::test_data()),
        };

        let spanned_str: Spanned<String> = Spanned {
            item: "//qp:b/text()".to_string(),
            span: Span::test_data(),
        };

        let actual =
            query(&call, &text, Some(spanned_str), Some(namespaces)).expect("test should not fail");

        let expected = Value::list(
            vec![Value::test_record(record! {
                "string_value" => Value::string("yay", Span::test_data()),
            })],
            Span::test_data(),
        );

        // and yet it should work regardless
        assert_eq!(actual, expected);
    }
    fn evaluate(xml: &str, xpath: &str, flags: &[&str]) -> Value {
        let span = Span::test_data();
        let call = EvaluatedCall {
            head: span,
            positional: vec![],
            named: flags
                .iter()
                .map(|flag| {
                    (
                        Spanned {
                            item: (*flag).to_owned(),
                            span,
                        },
                        None,
                    )
                })
                .collect(),
        };
        query(
            &call,
            &Value::test_string(xml),
            Some(Spanned {
                item: xpath.to_owned(),
                span,
            }),
            None,
        )
        .expect("valid XPath")
    }

    #[test]
    fn scalars_are_native_even_with_output_flags() {
        for flags in [vec![], vec!["output-type", "output-names"]] {
            assert_eq!(evaluate("<a/>", "true()", &flags), Value::test_bool(true));
            assert_eq!(evaluate("<a/>", "false()", &flags), Value::test_bool(false));
            assert_eq!(evaluate("<a/>", "1.5", &flags), Value::test_float(1.5));
            assert_eq!(
                evaluate("<a/>", "string('hello')", &flags),
                Value::test_string("hello")
            );
        }
    }

    #[test]
    fn nodeset_columns_are_fixed_for_long_unicode_queries() {
        let expected = Value::test_list(vec![
            Value::test_record(record! {"string_value" => Value::test_string("one")}),
            Value::test_record(record! {"string_value" => Value::test_string("two")}),
        ]);
        for xpath in [
            "//b",
            "//b[string(.) != '長長長長長長長長長長長長長長長長長長長長']",
        ] {
            assert_eq!(
                evaluate("<a><b>one</b><b>two</b></a>", xpath, &[]),
                expected
            );
        }
        assert_eq!(evaluate("<a/>", "//missing", &[]), Value::test_list(vec![]));
    }

    #[test]
    fn all_output_flag_combinations_select_columns_in_order() {
        for mask in 0..8 {
            let mut flags = vec![];
            if mask & 1 != 0 {
                flags.push("output-string-value");
            }
            if mask & 2 != 0 {
                flags.push("output-type");
            }
            if mask & 4 != 0 {
                flags.push("output-names");
            }
            let mut expected = nu_protocol::Record::new();
            if mask == 0 || mask & 1 != 0 {
                expected.push("string_value", Value::test_string("text"));
            }
            if mask & 2 != 0 {
                expected.push("type", Value::test_string("element"));
            }
            if mask & 4 != 0 {
                expected.push("local_name", Value::test_string("b"));
                expected.push("namespace", Value::test_string("urn:test"));
                expected.push("prefixed_name", Value::test_string("p:b"));
            }
            let actual = evaluate("<a xmlns:p='urn:test'><p:b>text</p:b></a>", "//*/*", &flags);
            assert_eq!(
                actual,
                Value::test_list(vec![Value::test_record(expected.clone())])
            );
            let row = &actual.as_list().unwrap()[0];
            assert_eq!(
                row.as_record().unwrap().columns().collect::<Vec<_>>(),
                expected.columns().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn every_node_kind_and_missing_names() {
        let xml = "<a xmlns:p='urn:test' p:attr='value'>text<!--comment--><?target data?></a>";
        for (xpath, kind, local, namespace, prefixed) in [
            ("/", "root", None, None, None),
            ("/a", "element", Some("a"), None, Some("a")),
            (
                "/a/@*",
                "attribute",
                Some("attr"),
                Some("urn:test"),
                Some("p:attr"),
            ),
            ("/a/text()", "text", None, None, None),
            ("/a/comment()", "comment", None, None, None),
            (
                "/a/processing-instruction()",
                "processing_instruction",
                Some("target"),
                None,
                Some("target"),
            ),
            ("/a/namespace::p", "namespace", Some("p"), None, Some("p")),
        ] {
            let optional = |s: Option<&str>| {
                s.map(Value::test_string)
                    .unwrap_or_else(Value::test_nothing)
            };
            assert_eq!(
                evaluate(xml, xpath, &["output-type", "output-names"]),
                Value::test_list(vec![Value::test_record(record! {
                    "type" => Value::test_string(kind),
                    "local_name" => optional(local),
                    "namespace" => optional(namespace),
                    "prefixed_name" => optional(prefixed),
                })]),
                "{xpath}"
            );
        }
    }

    #[test]
    fn xml_prefix_is_implicitly_registered() {
        assert_eq!(
            evaluate("<a xml:lang='en'/>", "string(/a/@xml:lang)", &[]),
            Value::test_string("en")
        );
    }

    #[test]
    fn signature_exposes_output_switches() {
        use nu_plugin::SimplePluginCommand;
        let signature = super::QueryXml.signature();
        for flag in ["output-string-value", "output-type", "output-names"] {
            assert!(
                signature
                    .named
                    .iter()
                    .any(|named| named.long == flag && named.arg.is_none())
            );
        }
    }
}
