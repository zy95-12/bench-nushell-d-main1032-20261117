# Predictable data command/path/format verification

Source: contract revision 2; supplied baseline is task_history:8300. Development starts at the accepted trial stacking commit named by that contract, on bot/predictable-data. The required fast-forward command succeeds there with “Already up to date.” Prior deliveries and their evidence are retained. No upstream, target/reference implementation, hidden tests, prior experiment material, or workflow changes were used. No dependencies were upgraded; rusqlite's existing column_decltype feature is enabled for declared JSON/JSONB types.

## Markdown block-separation repair

QA finding task_history:9462 concerns the returned candidate task_history:9479. The current repair changes only crates/nu-command/src/formats/to/md.rs and its CLI test file, apart from evidence. A content comparison against the returned candidate confirms every other implementation/test path is unchanged, so the prior targeted evidence remains applicable to those paths.

The renderer now ensures a blank line at boundaries involving tables, for both individual record tables and grouped table fragments. Headings and blockquotes retain their markup; ordinary scalar lists retain single line breaks. The examples now include the required blank lines.

New executable tests use to json -r to preserve exact newline assertions through the CLI test harness. Before rebuilding nu, cargo test -p nu-command --test main commands::to_md failed the three new cases on the returned candidate (2 existing pass, 3 new fail; md-blocks-before-repair.log). The same final five-test file was copied into the isolated accepted worktree; cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu-command --test main --features sqlite --no-run built successfully, then the isolated test binary ran commands::to_md --nocapture --test-threads=1 with the accepted target-dir/manifest environment described below. All five fail there (md-blocks-before-accepted.log), including adjacent records and grouped mixed schemas.

Current repair checks, all successful:

- cargo build -p nu (md-blocks-build.log)
- cargo test -p nu-command --test main commands::to_md: 5 pass (md-blocks-candidate.log)
- cargo test -p nu-command --lib formats::to::md: 14 pass (md-blocks-unit.log)
- cargo test -p nu-command --test main format_conversions::markdown: 7 pass (md-blocks-formats.log)
- cargo test -p nu-command --test main --features sqlite predictable: 30 pass (md-blocks-regressions.log)
- cargo check -p nu --locked (md-blocks-check.log)
- cargo fmt --all -- --check (md-blocks-fmt.log)

The original candidate checks below are historical for the returned candidate. The repair checks above supersede its Markdown results and reconfirm the full new CLI regression set. Known environment failures and platform limitations remain as recorded below; this repair does not touch their source or tests.

## Candidate verification

The root `nu` binary was rebuilt from this task's implementation before the CLI checks. All commands below ran on the candidate source; only subsequent test strengthening, formatting, and evidence recording followed. final-regressions.log reruns all 27 new CLI cases with SQLite, including JSONB blob storage and unchanged ordinary text/blob values. final-plugin-options.log reruns the options transport test; final-check.log reruns the locked nu check. fmt.log records cargo fmt --all -- --check. No implementation item remains unimplemented.

| Command | Result | Output |
| --- | --- | --- |
| `cargo test -p nu-path` | pass | [candidate-00.log](candidate-00.log) |
| `cargo test -p nuon` | pass | [candidate-01.log](candidate-01.log) |
| `cargo test -p nu-command --test main commands::str_length` | pass | [candidate-02.log](candidate-02.log) |
| `cargo test -p nu-command --test main commands::to_md` | pass | [candidate-03.log](candidate-03.log) |
| `cargo test -p nu-command --test main commands::into_binary` | pass | [candidate-04.log](candidate-04.log) |
| `cargo test -p nu-command --test main commands::get` | pass | [candidate-05.log](candidate-05.log) |
| `cargo test -p nu-command --test main commands::select` | pass | [candidate-06.log](candidate-06.log) |
| `cargo test -p nu-command --test main commands::reject` | pass | [candidate-07.log](candidate-07.log) |
| `cargo test -p nu-command --test main commands::relative_to` | pass | [candidate-08.log](candidate-08.log) |
| `cargo test -p nu-command --test main commands::open` | pass | [candidate-09.log](candidate-09.log) |
| `cargo test -p nu-command --test main commands::save` | pass | [candidate-10.log](candidate-10.log) |
| `cargo test -p nu-command --test main commands::source` | pass | [candidate-11.log](candidate-11.log) |
| `cargo test -p nu-command --test main --features sqlite predictable` | pass | [candidate-12.log](candidate-12.log) |
| `cargo test -p nu-command --test main --features sqlite commands::database::into_sqlite` | pass | [candidate-13.log](candidate-13.log) |
| `cargo test -p nu-command --test main format_conversions::markdown` | pass | [candidate-14.log](candidate-14.log) |
| `cargo test -p nu-command --lib formats::to::md` | pass | [candidate-15.log](candidate-15.log) |
| `cargo test -p nu-command --lib predictable_insensitive` | pass | [candidate-16.log](candidate-16.log) |
| `cargo test -p nu-color-config predictable_unspecified_theme` | pass | [candidate-17.log](candidate-17.log) |
| `cargo test -p nu-protocol` | pass | [candidate-18.log](candidate-18.log) |
| `cargo test -p nu-plugin -p nu-plugin-engine -p nu-plugin-protocol --lib` | pass | [candidate-19.log](candidate-19.log) |
| `cargo test -p nu-parser` | pass | [candidate-20.log](candidate-20.log) |
| `cargo check -p nu --locked` | pass | [candidate-21.log](candidate-21.log) |
| `cargo test -p nu --test main repl::test_config_path` | known baseline failure (unchanged) | [candidate-22.log](candidate-22.log) |
| `cargo test -p nu-cli --test main completions::variables_completions` | known baseline failure (unchanged) | [candidate-23.log](candidate-23.log) |

## Before-change discrimination

The isolated ../pre-change worktree starts at the contract's accepted trial stacking commit. Only the new regression tests (including test-only API probes) were applied there; no candidate implementation was applied. The isolated build target is /tmp/nu-predictable-pre-target. CLI tests execute that target's accepted-tree nu binary, never the candidate binary. The cargo wrapper overrides the CARGO_TARGET_DIR environment variable, so isolation uses the explicit --target-dir argument; direct test-binary execution supplies CARGO_TARGET_DIR for the test harness.

Executed commands:

- cargo build --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu (pass)
- cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu-command --test main --features sqlite --no-run (pass)
- /tmp/nu-predictable-pre-target/debug/deps/main-0f5c7bf05adfd82f predictable --nocapture --test-threads=1, with CARGO_TARGET_DIR=/tmp/nu-predictable-pre-target and CARGO_MANIFEST_DIR pointing at the accepted worktree's crates/nu-command (19 fail, 8 pass; before-cli.log). Failures exercise ignore-case access, const optional get, Markdown separators/escaping, endian conversion, NUON equals quoting, CellPath conversion/coercion, scalar counting, and SQLite complex columns. Passing cases are controls for unchanged behavior and rejection of invalid input.
- cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nuon predictable_equals (assertion fails; before-nuon.log).
- cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu-color-config predictable_unspecified_theme (assertion fails; before-theme.log).
- cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu-path predictable_device (compile failure: missing exported API; before-device-api.log).
- cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu-protocol predictable_custom (compile failure: missing option-aware methods; before-custom-api.log).
- cargo test --manifest-path ../pre-change/Cargo.toml --target-dir /tmp/nu-predictable-pre-target -p nu-command --lib predictable_insensitive (compile failure: missing helper; before-relative-api.log). Candidate portable helper assertions cover Unicode case folding, component boundaries, empty suffix, shorter paths, root mismatch, and parent components.

The API probes above distinguish API availability through compilation; they are not claimed as before-change runtime failures.

## Known environment failures and platform limits

Both accepted and candidate trees produce the same three failures for repl::test_config_path: test_default_config_path, test_xdg_config_bad, test_xdg_config_empty. Both pass the other 12 cases. Assertions compare /home/cdy/.config/nushell against the task HOME directory. before-config-path.log is the accepted root main test binary running repl::test_config_path --nocapture after cargo test -p nu --test main repl::test_config_path --no-run --message-format=json (with the same explicit accepted manifest and target-dir arguments).

Both trees fail cargo test -p nu-cli --test main completions::variables_completions with expected 19, actual 18 when the default feature set omits the plugin-path entry. The accepted command uses the same explicit manifest and target-dir arguments above; see before-completions.log and candidate-23.log. These tests were not modified or weakened.

Verification host: Linux. The device-path API's non-Windows false behavior executes here. Windows device open/save/source/parser regressions are cfg(windows) and were not executed here. Windows/macOS case fallback is platform-gated; its shared comparison helper executes on Linux, while the command's Linux case sensitivity also executes. Native Windows/macOS runtime evidence remains unavailable; no cross-platform runtime success is claimed.

Raw log output is retained, including diagnostic whitespace and environment-specific paths.
