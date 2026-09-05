use super::*;
use std::fs;

fn definition(patch: Value) -> Value {
    let mut base = json!({
        "id": "dump",
        "title": "Dump",
        "contexts": ["dir"],
        "argv": ["scripts/dump"],
    });
    for (key, value) in patch.as_object().unwrap() {
        base[key] = value.clone();
    }
    base
}

fn accepted(patch: Value) -> Action {
    normalize(&definition(patch)).unwrap()
}

fn refused(patch: Value) -> bool {
    normalize(&definition(patch)).is_err()
}

#[test]
fn argv_bounds_reject_the_entry_and_not_the_list() {
    let many: Vec<Value> = (0..33).map(|index| json!(format!("a{index}"))).collect();
    let wide: Vec<Value> = (0..9).map(|_| json!("b".repeat(1024))).collect();
    assert!(refused(json!({"argv": many})));
    assert!(refused(json!({"argv": wide})));
    assert!(refused(json!({"argv": ["scripts/dump", "a".repeat(1025)]})));
    assert!(refused(json!({"argv": ["scripts/dump", "a\u{7}b"]})));
    assert!(refused(json!({"argv": []})));
    assert!(refused(json!({"argv": "scripts/dump"})));
    let (actions, errors, truncated) = normalize_source(&[
        definition(json!({"argv": []})),
        definition(json!({"id": "good"})),
    ]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].id, "good");
    assert_eq!(errors.len(), 1);
    assert!(!truncated);
}

#[test]
fn contexts_ids_and_scalar_fields_are_bounded() {
    assert_eq!(
        accepted(json!({"contexts": ["dir", "nope", "dir"]})).contexts,
        vec![Context::Dir]
    );
    assert!(refused(json!({"contexts": ["nope"]})));
    assert!(refused(json!({"contexts": []})));
    assert!(refused(json!({"id": "-bad"})));
    assert!(refused(json!({"id": "a".repeat(65)})));
    assert!(refused(json!({"title": ""})));
    assert_eq!(accepted(json!({"timeout": 0})).timeout, MIN_TIMEOUT_SECONDS);
    assert_eq!(
        accepted(json!({"timeout": 100_000})).timeout,
        MAX_TIMEOUT_SECONDS
    );
    assert_eq!(accepted(json!({})).timeout, DEFAULT_TIMEOUT_SECONDS);
    let words = accepted(json!({"cwd": "x", "paths": "x", "output": "x"}));
    assert_eq!(words.cwd, CwdMode::Plugin);
    assert_eq!(words.paths, PathsMode::Env);
    assert_eq!(words.output, OutputMode::Notice);
    let chosen = accepted(json!({"cwd": "target", "paths": "append", "output": "silent"}));
    assert_eq!(chosen.cwd, CwdMode::Target);
    assert_eq!(chosen.paths, PathsMode::Append);
    assert_eq!(chosen.output, OutputMode::Silent);
    assert_eq!(accepted(json!({})).glyph, DEFAULT_GLYPH);
    assert_eq!(accepted(json!({"title": "Du\u{7}mp "})).title, "Dump");
}

#[test]
fn a_seventeenth_action_is_truncated_and_duplicates_are_reported() {
    let entries: Vec<Value> = (0..17)
        .map(|index| definition(json!({"id": format!("a{index}")})))
        .collect();
    let (actions, errors, truncated) = normalize_source(&entries);
    assert_eq!(actions.len(), MAX_ACTIONS_PER_SOURCE);
    assert!(truncated);
    assert!(errors.is_empty());
    let (single, duplicates, _) = normalize_source(&[definition(json!({})), definition(json!({}))]);
    assert_eq!(single.len(), 1);
    assert_eq!(duplicates.len(), 1);
}

#[test]
fn plugin_argv0_stays_inside_the_plugin_and_needs_the_exec_bit() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::create_dir(root.join("scripts")).unwrap();
    let script = root.join("scripts/dump");
    fs::write(&script, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(resolve_program(&accepted(json!({})), root, Source::Plugin).is_err());
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        resolve_program(&accepted(json!({})), root, Source::Plugin).unwrap(),
        script
    );
    std::os::unix::fs::symlink("/bin/sh", root.join("scripts/link")).unwrap();
    for argv0 in [
        "/usr/bin/env",
        "../x",
        "scripts/../../x",
        "./scripts/dump",
        "scripts/link",
        "scripts/missing",
    ] {
        let hostile = accepted(json!({"argv": [argv0]}));
        assert!(
            resolve_program(&hostile, root, Source::Plugin).is_err(),
            "{argv0} must be refused"
        );
    }
}

#[test]
fn user_argv0_accepts_a_bare_program_and_an_absolute_executable() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let bare = accepted(json!({"argv": ["printf", "%s", "x"]}));
    assert!(resolve_program(&bare, root, Source::User).is_ok());
    assert_eq!(
        resolve_program(&accepted(json!({"argv": ["/bin/sh"]})), root, Source::User).unwrap(),
        PathBuf::from("/bin/sh")
    );
    let plain_file = root.join("notes.txt");
    fs::write(&plain_file, "body").unwrap();
    let text = accepted(json!({"argv": [plain_file.to_string_lossy()]}));
    assert!(resolve_program(&text, root, Source::User).is_err());
    let missing = accepted(json!({"argv": ["fileblade-not-a-program"]}));
    assert!(resolve_program(&missing, root, Source::User).is_err());
}

#[test]
fn socket_entries_reads_only_the_fileblade_action_key() {
    let manifest = json!({
        "id": "kurt.tools",
        "extensions": {SOCKET_KEY: [definition(json!({}))], "other": [definition(json!({}))]}
    });
    assert_eq!(socket_entries(&manifest).len(), 1);
    assert!(socket_entries(&json!({"id": "kurt.tools"})).is_empty());
}

#[test]
fn a_symlinked_directory_component_does_not_escape_the_plugin() {
    let plugin = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let tool = outside.path().join("tool");
    fs::write(&tool, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink(outside.path(), plugin.path().join("scripts")).unwrap();
    let escape = accepted(json!({"argv": ["scripts/tool"]}));
    let refusal = resolve_program(&escape, plugin.path(), Source::Plugin)
        .expect_err("a symlinked directory component must not resolve");
    assert!(refusal.contains("inside the plugin"), "{refusal}");
}

#[test]
fn a_hostile_manifest_stops_at_the_error_cap_and_never_echoes_an_unbounded_id() {
    let entries: Vec<Value> = (0..500).map(|_| json!({})).collect();
    let (actions, errors, truncated) = normalize_source(&entries);
    assert!(actions.is_empty());
    assert_eq!(errors.len(), MAX_ERRORS_PER_SOURCE);
    assert!(truncated);
    let long = normalize(&definition(json!({"id": "!".repeat(50_000)}))).unwrap_err();
    assert!(long.len() < 256, "{}", long.len());
}

#[test]
fn rendered_text_drops_bidi_overrides_and_separators() {
    assert_eq!(
        accepted(json!({"title": "Run\u{202e}gpj.exe"})).title,
        "Rungpj.exe"
    );
    assert_eq!(
        accepted(json!({"description": "one\u{2028}two\u{2066}three"})).description,
        "onetwothree"
    );
}
