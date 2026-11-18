use nu_test_support::nu;

#[test]
fn which_ls() {
    let actual = nu!("which ls | get type.0");

    assert_eq!(actual.out, "built-in");
}

#[ignore = "TODO: Can't have alias recursion"]
#[test]
fn which_alias_ls() {
    let actual = nu!("alias ls = ls -a; which ls | get path.0 | str trim");

    assert_eq!(actual.out, "Nushell alias: ls -a");
}

#[test]
fn which_custom_alias() {
    let actual = nu!(r#"alias foo = print "foo!"; which foo | to nuon"#);

    assert_eq!(actual.out, r#"[[command, path, type]; [foo, "", alias]]"#);
}

#[test]
fn which_def_ls() {
    let actual = nu!("def ls [] {echo def}; which ls | get type.0");

    assert_eq!(actual.out, "custom");
}

#[ignore = "TODO: Can't have alias with the same name as command"]
#[test]
fn correct_precedence_alias_def_custom() {
    let actual =
        nu!("def ls [] {echo def}; alias ls = echo alias; which ls | get path.0 | str trim");

    assert_eq!(actual.out, "Nushell alias: echo alias");
}

#[ignore = "TODO: Can't have alias with the same name as command"]
#[test]
fn multiple_reports_for_alias_def_custom() {
    let actual = nu!("def ls [] {echo def}; alias ls = echo alias; which -a ls | length");

    let length: i32 = actual.out.parse().unwrap();
    assert!(length >= 2);
}

// `get_aliases_with_name` and `get_custom_commands_with_name` don't return the correct count of
// values
// I suspect this is due to the ScopeFrame getting discarded at '}' and the command is then
// executed in the parent scope
// See: parse_definition, line 2187 for reference.
#[ignore]
#[test]
fn correctly_report_of_shadowed_alias() {
    let actual = nu!("alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            which -a xaz
        }
        helper | get path | str contains alias2");

    assert_eq!(actual.out, "true");
}

#[test]
fn one_report_of_multiple_defs() {
    let actual = nu!("def xaz [] { echo def1 }
        def helper [] {
            def xaz [] { echo def2 }
            which -a xaz
        }
        helper | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn def_only_seen_once() {
    let actual = nu!("def xaz [] {echo def1}; which -a xaz | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn do_not_show_hidden_aliases() {
    let actual = nu!("alias foo = echo foo
        hide foo
        which foo | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn do_not_show_hidden_commands() {
    let actual = nu!("def foo [] { echo foo }
        hide foo
        which foo | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn which_accepts_spread_list() {
    let actual = nu!(
        cwd: ".",  // or any valid path
        r#"
        let apps = [ls]; 
        $apps | which ...$in | get command.0
        "#
    );

    assert_eq!(actual.out, "ls");
}

#[test]
fn streaming_which_without_arguments_lists_visible_commands() {
    let actual =
        nu!("def visible_command [] {}; which | where command == visible_command | get 0.type");
    assert!(actual.err.is_empty(), "{}", actual.err);
    assert_eq!(actual.out, "custom");
}

#[test]
fn no_arguments_lists_builtins_and_path_executables() {
    let result = nu!(
        r#"let unique = (which); let all = (which -a); [($unique | any {|entry| $entry.command == 'which' and $entry.type == 'built-in'}) ($unique | any {|entry| $entry.type == 'external'}) (($unique | length) <= ($all | length)) (($unique | get command | uniq | length) == ($unique | length))] | all {$in}"#
    );
    assert!(result.err.is_empty(), "{}", result.err);
    assert_eq!(result.out, "true");
}

#[test]
fn namespace_commands_display_help() {
    for name in ["detect", "attr", "error", "query"] {
        let result = nu!(name);
        assert!(result.err.is_empty(), "{name}: {}", result.err);
        assert!(result.out.contains("subcommand"), "{name}: {}", result.out);
    }
}

#[cfg(unix)]
#[test]
fn listing_respects_path_order_duplicates_and_executable_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    for path in [first.join("sample-exe"), second.join("sample-exe")] {
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(first.join("not-executable"), "data").unwrap();
    std::os::unix::fs::symlink(first.join("sample-exe"), first.join("linked-exe")).unwrap();
    let source = format!(
        r#"$env.PATH = ['{}' '{}']; let one = (which | where command == sample-exe); let all = (which -a | where command == sample-exe); [($one | length) ($all | length) ($one.0.path == '{}') (which | any {{|entry| $entry.command == not-executable}}) (which | any {{|entry| $entry.command == linked-exe}})] | to nuon"#,
        first.display(),
        second.display(),
        first.join("sample-exe").display()
    );
    let result = nu!(source);
    assert!(result.err.is_empty(), "{}", result.err);
    assert_eq!(result.out, "[1, 2, true, false, true]");
}
