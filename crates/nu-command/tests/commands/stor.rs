use nu_test_support::{nu, pipeline};

#[test]
fn stor_insert() {
    let actual = nu!(pipeline(
        r#"
            stor create --table-name test_table --columns { id: int, value: str };
            stor insert -t test_table --data-record {
                id: 1
                value: "'Initial value'"
            };
            stor open | query db 'select value from test_table' | get 0.value
        "#
    ));

    assert_eq!(actual.out, "'Initial value'");
}

#[test]
fn stor_update_with_quote() {
    let actual = nu!(pipeline(
        r#"
            stor create --table-name test_table --columns { id: int, value: str };
            stor insert -t test_table --data-record {
                id: 1
                value: "'Initial value'"
            };
            stor update -t test_table --where-clause 'id = 1' --update-record {
                id: 1
                value: "This didn't work, but should now."
            };
            stor open | query db 'select value from test_table' | get 0.value
        "#
    ));

    assert_eq!(actual.out, "This didn't work, but should now.");
}

#[test]
fn stor_json_and_jsonb_round_trip_nested_values() {
    for (json, jsonb) in [("json", "JSONB"), ("JsOn", "jSoNb")] {
        let actual = nu!(format!(
            r#"
            stor create -t json_round_trip -c {{id: int, data: {json}, nested: {jsonb}}};
            let expected = {{id: 1, data: {{name: example, items: [1 2], child: {{ok: true}}}}, nested: [1 {{a: [true null]}}]}};
            $expected | stor insert -t json_round_trip;
            let actual = (stor open | query db 'SELECT * FROM json_round_trip' | first);
            $actual == $expected
        "#
        ));
        assert!(actual.err.is_empty(), "{json}/{jsonb}: {}", actual.err);
        assert_eq!(actual.out, "true", "{json}/{jsonb}");
    }
}
