# Streaming and command discovery verification

The implementation starts at the accepted trial commit named in contract:revision:2. The supplied published baseline (task_history:6681 / task_history:6683) has an identical tree but different commit ancestry: the required `git merge --ff-only` refused. A new local task branch was therefore created at the explicitly authorized accepted trial commit. Prior child content, including its evidence, is retained. No upstream/reference/hidden implementation, evaluator artifact, or prior experiment was accessed. No workflow file or dependency version changed. The only lockfile addition is the nu-command test dependency edge to already locked typetag.

The before worktree `/tmp/nu-streaming-before` starts at that accepted commit. Only this trial's new regression sources (and typetag dev-dependency/lockfile support) were copied there. Its implementation remains unchanged. This includes tests/main.rs registering the new root regression module, appended overlay regressions, command regressions, the in-process test file, and the appended protocol configuration test module.

## Environment and commands

The environment's default cargo wrapper forcibly replaces CARGO_TARGET_DIR. Early attempts sharing its build directory were discarded. All logs recorded here after restart use a native cargo shim, with Rust 1.88.0 and CARGO_BUILD_JOBS=2, preserving caller-selected targets:

- Before: CARGO_TARGET_DIR=/tmp/nu-streaming-before-target, cwd=/tmp/nu-streaming-before.
- Candidate: CARGO_TARGET_DIR=/tmp/nu-streaming-candidate-target, cwd=the task checkout.
- CARGO_HOME=/home/cdy/zhangyi/bot-benchmark/runs/ripgrep-001/toolchain/cargo; RUSTUP_HOME=/home/cdy/zhangyi/bot-benchmark/runs/ripgrep-001/toolchain/rustup; RUSTUP_TOOLCHAIN=1.88.0.
- Shim execs `$CARGO_HOME/bin/cargo` and does not override the caller's target directory. Both trees have their own built `debug/nu`, which nu-test-support selects through CARGO_TARGET_DIR.

`results.txt` contains exact candidate commands and exits; corresponding log names retain full output. `before-results.txt` and before-*.log record before-change commands/results:

```
cargo build -p nu --locked
cargo test -p nu --test main streaming
cargo test -p nu-command --test main streaming
cargo test -p nu-command --test streaming
cargo test -p nu-protocol streaming_table
cargo test -p nu --test main repl::test_config_path
cargo test -p nu-cli --test main dotnu_completions
cargo test -p nu-cli --test main variables_completions
```

The original detect-columns invocation exposed a test-fixture issue: nu! rewrites physical newlines into statement separators. The fixtures now use escaped newlines; detect-columns-final.log and before-commands-final.log are the applicable results. Earlier detect-columns.log is retained as a superseded attempt.

## Behavior coverage

| Contract item | Executable regression and evidence |
| --- | --- |
| Overlay listing/status/hidden/order | overlays::streaming_overlay_list_includes_hidden_with_status; before-root.log fails, root-streaming.log passes. Existing overlay assertions were adapted to the table API; overlays.log covers the full suite. Active overlays retain activation order, then hidden overlays are sorted by name. |
| Scoped overlay module registration | overlays::streaming_overlay_origin_module_is_registered_in_scope; hide the overlay that originally declared the module, then import through the activated overlay in a scoped block. Before fails, candidate passes. |
| Duplicate-key safety | commands::detect_columns::streaming_duplicate_headers_are_unique (default and guess algorithms, including a suffix collision), and streaming_overlapping_cells_do_not_duplicate_keys. Before fails, detect-columns-final.log passes. |
| Argument-free which | commands::which::streaming_which_without_arguments_lists_visible_commands; before fails, which.log passes. |
| Namespace fallbacks | streaming::streaming_namespace_help_retains_real_commands; orphan detect namespace gains help while math sum and ansi --list still work. Before fails, root-streaming.log passes. namespace-context.log checks full context conventions. |
| Lazy each flatten | commands::each::streaming_flatten_* and streaming_flatten_pulls_closure_results_on_demand. The in-process counted stream asserts zero pulls before iteration, then exactly one pull per requested output; no timing assumption or unbounded iterator. Before fails, each-final.log and inprocess-final.log pass. Default nested-list behavior has a compatibility guard. |
| Single null no-op | commands::each::streaming_single_null_skips_closure (including keep-empty and flatten flags); before fails, each-final.log passes. |
| Nested each errors | commands::each::streaming_nested_errors_survive_ignoring_closures; before fails, each-final.log passes. |
| Try handler cleanup | streaming::streaming_try_break_removes_handler and streaming_try_continue_removes_handler; before fails, root-streaming.log passes. streaming_loop_preserves_enclosing_handler guards existing enclosing-handler behavior and passes both. |
| Partial-result metadata | streaming_partial_results_clear_content_type_and_retain_origin exercises selectors, row filters, projections and mappings with both values and list streams; streaming_partial_binary_results_clear_content_type checks value and byte-stream branches. Before fails, inprocess-final.log passes. File-origin metadata is retained, including streamed reject. |
| Custom-value save | streaming_save_custom_values saves custom text, binary and JSON records. Before fails, inprocess-final.log passes. streaming_save_preserves_custom_extension_dispatch protects existing custom converter behavior (passes both trees). save.log covers existing save tests. |
| Configurable table streaming | streaming_table_page_size_controls_batches and streaming_table_batch_duration_controls_batches assert actual output batch counts. Protocol streaming_table_settings_* checks configuration update/serialization and validation. Before fails on missing settings; candidate passes. Config paths: table.page_size (positive int, default 1000), table.batch_duration (nonnegative duration, default 1sec). |
| Example-result spans | streaming::streaming_example_result_spans_are_source_spans accesses a nested command example result through scope commands and metadata. Before reports sentinel test spans; candidate uses the current scope command source span recursively. |

## Existing environment/feature failures

The unchanged config-path suite has 12 passing tests and three failures on both trees: test_default_config_path, test_xdg_config_bad and test_xdg_config_empty. The mismatch is the process home/config path versus the passwd home path. Tests were not weakened.

The requested dotnu_completions filter passes all three matching tests on the accepted tree; the candidate CLI suite also passes them. The home-dependent expectation did not fail in this run.

An additional default-feature CLI failure was observed and independently reproduced on the accepted tree: variables_completions expects 19 $nu fields, including plugin-path, while the default nu-cli feature set supplies 18. Full `cargo test -p nu-cli --features plugin` passes (53 unit tests, 91 integration tests, 4 ignored); the default-feature failure and its accepted-tree reproduction are retained in cli.log and before-cli-variables.log. No test changes mask it.

The mandated nu-command commands::overlay filter contains zero tests in this baseline layout; the actual overlay tests are in the root package and were also run in full (96 pass, 2 ignored).

No contract item is intentionally deferred. Final source changes after the first broad runs were limited to preserving save's existing extension-converter dispatch; the final in-process and save suites exercise that exact source, and the locked cargo check covers the final source tree. Submission remains subject to independent review/QA.
