use std::path::Path;

use crate::{Id, LanguageServer, path_to_uri, span_to_range};
use lsp_textdocument::FullTextDocument;
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location};
use nu_protocol::Span;
use nu_protocol::engine::{CachedFile, StateWorkingSet};

impl LanguageServer {
    fn get_location_by_span<'a>(
        &self,
        files: impl Iterator<Item = &'a CachedFile>,
        span: &Span,
    ) -> Option<Location> {
        for cached_file in files.into_iter() {
            if cached_file.covered_span.contains(span.start) {
                let path = Path::new(&*cached_file.name);
                let target_uri = path_to_uri(path);
                if let Some(file) = self.docs.lock().ok()?.get_document(&target_uri) {
                    return Some(Location {
                        uri: target_uri,
                        range: span_to_range(span, file, cached_file.covered_span.start),
                    });
                } else if path.is_file() {
                    // in case where the document is not opened yet,
                    // typically included by the `use/source` command
                    let temp_doc = FullTextDocument::new(
                        "nu".to_string(),
                        0,
                        String::from_utf8_lossy(cached_file.content.as_ref()).to_string(),
                    );
                    return Some(Location {
                        uri: target_uri,
                        range: span_to_range(span, &temp_doc, cached_file.covered_span.start),
                    });
                }
            }
        }
        None
    }

    pub(crate) fn find_definition_span_by_id(
        working_set: &StateWorkingSet,
        id: &Id,
    ) -> Option<Span> {
        match id {
            Id::Declaration(decl_id) => {
                let block_id = working_set.get_decl(*decl_id).block_id()?;
                working_set.get_block(block_id).span
            }
            Id::Variable(var_id, _) => {
                let var = working_set.get_variable(*var_id);
                Some(var.declaration_span)
            }
            Id::Module(module_id, _) => {
                let module = working_set.get_module(*module_id);
                module.span
            }
            Id::CellPath(var_id, cell_path) => {
                let var = working_set.get_variable(*var_id);
                Some(
                    var.const_val
                        .as_ref()
                        .and_then(|val| val.follow_cell_path(cell_path).ok())
                        .map(|val| val.span())
                        .unwrap_or(var.declaration_span),
                )
            }
            _ => None,
        }
    }

    pub(crate) fn goto_definition(
        &mut self,
        params: &GotoDefinitionParams,
    ) -> Option<GotoDefinitionResponse> {
        let path_uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_owned();
        let mut engine_state = self.new_engine_state(Some(&path_uri));
        let (working_set, id, _, _) = self
            .parse_and_find(
                &mut engine_state,
                &path_uri,
                params.text_document_position_params.position,
            )
            .ok()?;

        Some(GotoDefinitionResponse::Scalar(self.get_location_by_span(
            working_set.files(),
            &Self::find_definition_span_by_id(&working_set, &id)?,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use crate::path_to_uri;
    use crate::tests::{initialize_language_server, open_unchecked, result_from_message};
    use assert_json_diff::assert_json_eq;
    use lsp_server::{Connection, Message};
    use lsp_types::{
        GotoDefinitionParams, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, Uri, WorkDoneProgressParams,
        request::{GotoDefinition, Request},
    };
    use nu_test_support::fs::{fixtures, root};

    fn send_goto_definition_request(
        client_connection: &Connection,
        uri: Uri,
        line: u32,
        character: u32,
    ) -> Message {
        client_connection
            .sender
            .send(Message::Request(lsp_server::Request {
                id: 2.into(),
                method: GotoDefinition::METHOD.to_string(),
                params: serde_json::to_value(GotoDefinitionParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri },
                        position: Position { line, character },
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .unwrap(),
            }))
            .unwrap();

        client_connection
            .receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
    }

    #[test]
    fn goto_definition_for_none_existing_file() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut none_existent_path = root();
        none_existent_path.push("none-existent.nu");
        let script = path_to_uri(&none_existent_path);
        let resp = send_goto_definition_request(&client_connection, script, 0, 0);

        assert_json_eq!(result_from_message(resp), serde_json::json!(null));
    }

    #[rstest::rstest]
    #[case("let local = 42\n$local", 1, 3, 4, 9)]
    #[case("let café = 42\n$café", 1, 3, 4, 8)]
    fn goto_definition_in_unsaved_document(
        #[case] text: &str,
        #[case] line: u32,
        #[case] character: u32,
        #[case] start: u32,
        #[case] end: u32,
    ) {
        let (client_connection, _recv) = initialize_language_server(None, None);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unsaved.nu");
        assert!(!path.exists());
        let uri = path_to_uri(&path);
        crate::tests::open_text(&client_connection, uri.clone(), text.into()).unwrap();
        let response =
            send_goto_definition_request(&client_connection, uri.clone(), line, character);
        assert_json_eq!(
            result_from_message(response),
            serde_json::json!({
                "uri": uri,
                "range": {
                    "start": { "line": 0, "character": start },
                    "end": { "line": 0, "character": end }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_cell_path() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("hover");
        script.push("use.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 2, 7);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 1, "character": 10 })
        );

        let resp = send_goto_definition_request(&client_connection, script, 2, 9);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 1, "character": 17 })
        );
    }

    #[test]
    fn goto_definition_of_module_in_another_file() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 0, 23);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 1, "character": 29 },
                    "end": { "line": 1, "character": 30 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_module_in_hide() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());
        let resp = send_goto_definition_request(&client_connection, script.clone(), 3, 6);

        assert_json_eq!(
            result_from_message(resp),
            serde_json::json!({
                "uri": script.to_string().replace("use_module", "module"),
                "range": {
                    "start": { "line": 1, "character": 29 },
                    "end": { "line": 1, "character": 30 }
                }
            })
        );
    }

    #[test]
    fn goto_definition_of_module_in_overlay() {
        let (client_connection, _recv) = initialize_language_server(None, None);

        let mut script = fixtures();
        script.push("lsp");
        script.push("goto");
        script.push("use_module.nu");
        let script = path_to_uri(&script);

        open_unchecked(&client_connection, script.clone());

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 20);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 0, "character": 0 })
        );

        let resp = send_goto_definition_request(&client_connection, script.clone(), 1, 25);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 0, "character": 0 })
        );

        let resp = send_goto_definition_request(&client_connection, script, 2, 30);
        assert_json_eq!(
            result_from_message(resp).pointer("/range/start").unwrap(),
            serde_json::json!({ "line": 0, "character": 0 })
        );
    }
    #[rstest::rstest]
    #[case::goto_definition_of_variable("var.nu", (2, 12), (0, 4), (0, 12))]
    #[case::goto_definition_of_command("command.nu", (4, 1), (0, 17), (2, 1))]
    #[case::goto_definition_of_command_unicode("command_unicode.nu", (4, 2), (0, 19), (2, 1))]
    #[case::goto_definition_of_command_parameter("command.nu", (1, 14), (0, 11), (0, 15))]
    #[case::goto_definition_of_variable_in_else_block("else.nu", (1, 21), (0, 4), (0, 7))]
    #[case::goto_definition_of_variable_in_match_guard("match.nu", (2, 9), (0, 4), (0, 7))]
    #[case::goto_definition_of_variable_in_each("collect.nu", (1, 16), (0, 4), (0, 7))]
    #[case::goto_definition_of_module("module.nu", (3, 15), (1, 29), (1, 30))]
    fn goto_definition_from_fixture(
        #[case] file: &str,
        #[case] position: (u32, u32),
        #[case] start: (u32, u32),
        #[case] end: (u32, u32),
    ) {
        let (connection, _recv) = initialize_language_server(None, None);
        let script = path_to_uri(&fixtures().join("lsp").join("goto").join(file));
        open_unchecked(&connection, script.clone());
        let response =
            send_goto_definition_request(&connection, script.clone(), position.0, position.1);
        assert_json_eq!(
            result_from_message(response),
            serde_json::json!({
                "uri": script,
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 }
                }
            })
        );
    }
}
