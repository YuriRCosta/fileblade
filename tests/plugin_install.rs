#[test]
fn every_example_extension_is_pinned_to_a_reviewed_commit() {
    for extension in fileblade::plugin_install::EXTENSIONS {
        assert!(fileblade::plugin_install::allowed(extension.url));
        assert_eq!(
            extension.commit.len(),
            40,
            "{} needs a full commit, not {}",
            extension.url,
            extension.commit
        );
        assert!(
            extension
                .commit
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
            "{} has a malformed commit",
            extension.url
        );
        assert!(
            extension
                .url
                .starts_with("https://github.com/data-goblin/fileblade-"),
            "{} is not a FileBlade example extension",
            extension.url
        );
    }
    assert!(!fileblade::plugin_install::allowed(
        "https://github.com/data-goblin/fileblade-memory"
    ));
    assert!(!fileblade::plugin_install::allowed(
        "https://example.invalid/evil.git"
    ));
    assert!(fileblade::plugin_install::pinned("https://example.invalid/evil.git").is_none());
}

fn install_mock(scripts: &std::path::Path) {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let mock = r#"#!/usr/bin/python3
import json, os, pathlib, shutil, sys
home = pathlib.Path(os.environ['HOME'])
verb = pathlib.Path(sys.argv[0]).name
if verb == 'git':
    while sys.argv[1:2] == ['-c']:
        del sys.argv[1:3]
plugins = home / '.config/omarchy/plugins'
plugins.mkdir(parents=True, exist_ok=True)
with (home / 'calls').open('a') as log:
    log.write(' '.join([verb] + sys.argv[1:]) + '\n')
if verb == 'git':
    if sys.argv[1:2] == ['clone']:
        assert '--no-checkout' in sys.argv and '--depth=1' in sys.argv and '--template=' in sys.argv
        assert '--revision' in sys.argv
        url, dest = sys.argv[-2], pathlib.Path(sys.argv[-1])
        name = url.split('/')[-1].removesuffix('.git')
        if name == 'fileblade-mcp' and not (home / 'network-restored').exists():
            print('network unavailable', file=sys.stderr)
            sys.exit(128)
        present = sorted(p.name for p in plugins.iterdir() if p.name.startswith('data-goblin.'))
        (home / 'present-at-clone').open('a').write(name + ' ' + ','.join(present) + '\n')
        dest.mkdir(parents=True)
        (dest / 'manifest.json').write_text(json.dumps({'id': 'data-goblin.' + name}))
        head = (home / 'branch-head').read_text().strip() if (home / 'branch-head').exists() else 'f' * 40
        (dest / '.head').write_text(head)
        (dest / '.origin').write_text(url)
        (dest / '.available').write_text((home / 'available-commits').read_text() if (home / 'available-commits').exists() else '')
        sys.exit(0)
    here = pathlib.Path.cwd()
    available = (here / '.available').read_text().split() if (here / '.available').exists() else []
    if sys.argv[1:3] == ['rev-parse', '--verify']:
        wanted = sys.argv[3].removesuffix('^{commit}')
        if wanted not in available:
            print('bad revision', file=sys.stderr)
            sys.exit(128)
        print(wanted)
    elif sys.argv[1:4] == ['checkout', '--detach', '--force']:
        (here / '.head').write_text(sys.argv[4])
    elif sys.argv[1:2] == ['ls-tree']:
        print('100644 blob ' + 'a' * 40 + ' 80\tmanifest.json\0', end='')
    elif sys.argv[1:] == ['rev-parse', '--show-toplevel']:
        print(here)
    elif sys.argv[1:] == ['remote', 'get-url', 'origin']:
        print((here / '.origin').read_text().strip())
    elif sys.argv[1:2] == ['status']:
        if (here / '.dirty').exists():
            print(' M manifest.json')
    elif sys.argv[1:] == ['rev-parse', 'HEAD']:
        print((here / '.head').read_text().strip())
    else:
        print('unexpected git call ' + ' '.join(sys.argv[1:]), file=sys.stderr)
        sys.exit(2)
elif verb == 'omarchy-git-url-check':
    assert sys.argv[1].startswith('https://github.com/data-goblin/')
elif verb == 'omarchy-plugin-validate':
    assert (pathlib.Path(sys.argv[1]) / 'manifest.json').exists()
elif verb == 'omarchy-plugin-enable':
    (plugins / sys.argv[1] / 'enabled').touch()
elif verb == 'omarchy-plugin-list':
    print(json.dumps([{'id': path.name, 'enabled': (path / 'enabled').exists()} for path in plugins.iterdir() if path.is_dir() and not path.name.startswith('.')]))
"#;
    for name in [
        "git",
        "omarchy-git-url-check",
        "omarchy-plugin-validate",
        "omarchy-plugin-enable",
        "omarchy-plugin-list",
    ] {
        let path = scripts.join(name);
        fs::write(&path, mock).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn publish_pins(home: &std::path::Path) {
    let commits: Vec<&str> = fileblade::plugin_install::EXTENSIONS
        .iter()
        .map(|extension| extension.commit)
        .collect();
    std::fs::write(home.join("available-commits"), commits.join("\n")).unwrap();
}

fn run_backend(home: &std::path::Path, scripts: &std::path::Path, verb: &str) -> serde_json::Value {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fileblade"))
        .args(["_backend", verb])
        .env("PATH", scripts)
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
}

#[test]
fn install_stages_every_extension_before_any_reaches_the_plugins_directory() {
    use std::fs;

    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    publish_pins(temporary.path());
    fs::write(temporary.path().join("network-restored"), "").unwrap();
    let result = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(result["state"], "installed", "{result}");
    assert_eq!(result["installed"], 4);
    let present = fs::read_to_string(temporary.path().join("present-at-clone")).unwrap();
    for line in present.lines() {
        assert!(
            line.ends_with(' '),
            "a clone saw installed extensions: {line}"
        );
    }
    assert_eq!(present.lines().count(), 4);
    let plugins = temporary.path().join(".config/omarchy/plugins");
    for name in ["memory", "skills", "mcp", "hooks"] {
        let dir = plugins.join(format!("data-goblin.fileblade-{name}"));
        assert!(dir.join("manifest.json").exists(), "{name} missing");
        assert!(dir.join("enabled").exists(), "{name} not enabled");
    }
    assert!(
        !temporary
            .path()
            .join(".config/omarchy/.fileblade-extension-stage")
            .exists()
    );
    let calls = fs::read_to_string(temporary.path().join("calls")).unwrap();
    assert!(!calls.contains("omarchy-plugin-add"));
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("omarchy-git-url-check "))
            .count(),
        4
    );
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("omarchy-plugin-validate "))
            .count(),
        8
    );
}

#[test]
fn batch_recovers_a_disabled_clone_and_retries_only_missing_repositories() {
    use std::fs;

    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    publish_pins(temporary.path());
    let plugins = temporary.path().join(".config/omarchy/plugins");
    let memory = plugins.join("data-goblin.fileblade-memory");
    fs::create_dir_all(&memory).unwrap();
    fs::write(
        memory.join("manifest.json"),
        r#"{"id":"data-goblin.fileblade-memory"}"#,
    )
    .unwrap();
    fs::write(
        memory.join(".head"),
        fileblade::plugin_install::EXTENSIONS[0].commit,
    )
    .unwrap();
    fs::write(
        memory.join(".origin"),
        fileblade::plugin_install::EXTENSIONS[0].url,
    )
    .unwrap();
    assert_eq!(
        run_backend(temporary.path(), &scripts, "plugin-install-status")["state"],
        "idle"
    );
    let failed = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(failed["state"], "failed", "{failed}");
    assert_eq!(failed["installed"], 2);
    assert!(
        failed["message"]
            .as_str()
            .unwrap()
            .contains("network unavailable")
    );
    assert!(
        !plugins.join("data-goblin.fileblade-skills").exists(),
        "a staged clone leaked into the plugins dir"
    );
    assert!(
        !temporary
            .path()
            .join(".config/omarchy/.fileblade-extension-stage")
            .exists()
    );
    assert_eq!(
        run_backend(temporary.path(), &scripts, "plugin-install-status")["state"],
        "failed"
    );
    fs::write(temporary.path().join("network-restored"), "").unwrap();
    let result = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(result["state"], "installed", "{result}");
    assert_eq!(result["installed"], 4);
    assert!(
        memory.join("enabled").exists(),
        "the pre-existing disabled clone was not enabled"
    );
    let validated = fs::read_to_string(temporary.path().join("calls")).unwrap();
    assert!(
        validated
            .lines()
            .any(|line| line.starts_with("omarchy-plugin-validate ")
                && line.contains("plugins/data-goblin.fileblade-memory")),
        "the pre-existing clone must be validated before it is enabled: {validated}"
    );
    assert_eq!(
        run_backend(temporary.path(), &scripts, "plugin-install-status")["state"],
        "installed"
    );
    assert_eq!(
        run_backend(temporary.path(), &scripts, "plugin-install")["state"],
        "installed"
    );
    let calls = fs::read_to_string(temporary.path().join("calls")).unwrap();
    let clones: Vec<_> = calls
        .lines()
        .filter(|line| line.starts_with("git clone "))
        .collect();
    assert_eq!(clones.len(), 5, "{calls}");
    assert_eq!(
        clones
            .iter()
            .filter(|line| line.contains("fileblade-skills.git"))
            .count(),
        2,
        "skills is staged again because the first attempt failed before the move"
    );
    assert_eq!(
        clones
            .iter()
            .filter(|line| line.contains("fileblade-hooks.git"))
            .count(),
        1,
        "hooks is only reached on the retry"
    );
    assert_eq!(
        clones
            .iter()
            .filter(|line| line.contains("fileblade-mcp.git"))
            .count(),
        2
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("git clone ") && line.contains("fileblade-memory.git"))
    );
    assert!(!calls.contains("fileblade-git"));
}

#[test]
fn abandoned_install_reports_interruption_without_restarting() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let state = temporary.path().join("omarchy/fileblade");
    fs::create_dir_all(&state).unwrap();
    let progress = state.join("extension-install.json");
    fs::write(&progress, r#"{"ok":true,"state":"running","installed":2}"#).unwrap();
    fs::set_permissions(&progress, fs::Permissions::from_mode(0o600)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fileblade"))
        .args(["_backend", "plugin-install-status"])
        .env("PATH", temporary.path())
        .env("HOME", temporary.path())
        .env("XDG_STATE_HOME", temporary.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["state"], "failed");
    assert_eq!(result["installed"], 2);
    assert!(result["message"].as_str().unwrap().contains("interrupted"));
}

#[test]
fn install_follows_a_symlinked_plugins_directory() {
    use std::fs;

    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    publish_pins(temporary.path());
    fs::write(temporary.path().join("network-restored"), "").unwrap();
    let real = temporary.path().join("dotfiles/plugins");
    fs::create_dir_all(&real).unwrap();
    fs::create_dir_all(temporary.path().join(".config/omarchy")).unwrap();
    std::os::unix::fs::symlink(&real, temporary.path().join(".config/omarchy/plugins")).unwrap();
    let result = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(result["state"], "installed", "{result}");
    assert_eq!(result["installed"], 4);
    assert!(
        real.join("data-goblin.fileblade-hooks/manifest.json")
            .exists()
    );
    assert!(
        !temporary
            .path()
            .join("dotfiles/.fileblade-extension-stage")
            .exists()
    );
}

#[test]
fn a_repository_that_moved_past_the_pinned_commit_is_refused() {
    use std::fs;

    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    fs::write(temporary.path().join("network-restored"), "").unwrap();
    let memory = &fileblade::plugin_install::EXTENSIONS[0];
    let others: Vec<&str> = fileblade::plugin_install::EXTENSIONS
        .iter()
        .skip(1)
        .map(|extension| extension.commit)
        .collect();
    fs::write(
        temporary.path().join("available-commits"),
        others.join("\n"),
    )
    .unwrap();
    fs::write(temporary.path().join("branch-head"), "a".repeat(40)).unwrap();

    let result = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(result["state"], "failed", "{result}");
    let message = result["message"].as_str().unwrap();
    assert!(
        message.contains("does not contain the reviewed commit") && message.contains(memory.commit),
        "{message}"
    );
    let plugins = temporary.path().join(".config/omarchy/plugins");
    assert!(
        !plugins.join("data-goblin.fileblade-memory").exists(),
        "nothing may land in the plugins directory when the pin is missing"
    );
    assert!(
        !temporary
            .path()
            .join(".config/omarchy/.fileblade-extension-stage")
            .exists()
    );
}

#[test]
fn the_installed_tree_is_the_pinned_commit_not_the_branch_head() {
    use std::fs;

    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    publish_pins(temporary.path());
    fs::write(temporary.path().join("network-restored"), "").unwrap();
    fs::write(temporary.path().join("branch-head"), "b".repeat(40)).unwrap();

    let result = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(result["state"], "installed", "{result}");
    let plugins = temporary.path().join(".config/omarchy/plugins");
    for extension in fileblade::plugin_install::EXTENSIONS {
        let id = extension
            .url
            .rsplit('/')
            .next()
            .unwrap()
            .trim_end_matches(".git");
        let head = fs::read_to_string(plugins.join(format!("data-goblin.{id}/.head")).as_path())
            .unwrap_or_default();
        assert_eq!(
            head.trim(),
            extension.commit,
            "{id} was left at the branch head instead of its pin"
        );
    }
}

#[test]
fn welcome_preserves_an_existing_checkout_with_local_changes_or_a_different_pin() {
    for dirty in [false, true] {
        let temporary = tempfile::tempdir().unwrap();
        let scripts = temporary.path().join("bin");
        std::fs::create_dir(&scripts).unwrap();
        install_mock(&scripts);
        publish_pins(temporary.path());
        std::fs::write(temporary.path().join("network-restored"), "").unwrap();
        let memory = temporary
            .path()
            .join(".config/omarchy/plugins/data-goblin.fileblade-memory");
        std::fs::create_dir_all(&memory).unwrap();
        let extension = &fileblade::plugin_install::EXTENSIONS[0];
        let head = if dirty {
            extension.commit
        } else {
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        };
        std::fs::write(memory.join(".head"), head).unwrap();
        std::fs::write(memory.join(".origin"), extension.url).unwrap();
        std::fs::write(memory.join("manifest.json"), "local content").unwrap();
        if dirty {
            std::fs::write(memory.join(".dirty"), "").unwrap();
        }
        let result = run_backend(temporary.path(), &scripts, "plugin-install");
        assert_eq!(result["state"], "failed", "{result}");
        assert!(
            result["message"]
                .as_str()
                .unwrap()
                .contains("was preserved")
        );
        assert_eq!(std::fs::read_to_string(memory.join(".head")).unwrap(), head);
        assert_eq!(
            std::fs::read_to_string(memory.join("manifest.json")).unwrap(),
            "local content"
        );
        assert!(!memory.join("enabled").exists());
        assert!(!temporary.path().join("present-at-clone").exists());
    }
}

#[test]
fn welcome_never_cleans_an_unrelated_directory_with_the_old_stage_name() {
    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    std::fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    publish_pins(temporary.path());
    let foreign = temporary
        .path()
        .join(".config/omarchy/.fileblade-extension-stage");
    std::fs::create_dir_all(&foreign).unwrap();
    std::fs::write(foreign.join("keep"), "user data").unwrap();
    let result = run_backend(temporary.path(), &scripts, "plugin-install");
    assert_eq!(result["state"], "failed");
    assert_eq!(
        std::fs::read_to_string(foreign.join("keep")).unwrap(),
        "user data"
    );
    assert!(
        !std::fs::read_dir(foreign.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .as_encoded_bytes()
                .starts_with(b".fileblade-partial-"))
    );
}
