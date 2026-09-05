#[test]
fn only_the_example_extensions_are_installable() {
    for url in fileblade::plugin_install::ALLOWED_URLS {
        assert!(fileblade::plugin_install::allowed(url));
    }
    assert!(!fileblade::plugin_install::allowed(
        "https://github.com/data-goblin/fileblade-memory"
    ));
    assert!(!fileblade::plugin_install::allowed(
        "https://example.invalid/evil.git"
    ));
    let refused = fileblade::plugin_install::plugin_add("https://example.invalid/evil.git");
    assert_eq!(refused["ok"], false);
    assert!(
        refused["message"]
            .as_str()
            .unwrap_or("")
            .contains("Welcome tab")
    );
}

#[test]
fn welcome_install_confirms_and_enables_without_a_terminal() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let program = temporary.path().join("omarchy-plugin-add");
    fs::write(
        &program,
        "#!/bin/sh\n[ ! -t 0 ] || exit 9\n[ \"$1\" = https://github.com/data-goblin/fileblade-memory.git ] || exit 10\n[ \"$2\" = --yes ] || exit 11\n[ \"$3\" = --enable ] || exit 12\n[ \"$#\" = 3 ] || exit 13\nprintf 'Installed and enabled\\n'\n",
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fileblade"))
        .args([
            "_backend",
            "plugin-add",
            "--url",
            fileblade::plugin_install::ALLOWED_URLS[0],
        ])
        .env("PATH", temporary.path())
        .env("HOME", temporary.path())
        .env("XDG_STATE_HOME", temporary.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["output"], "Installed and enabled");
}

fn install_mock(scripts: &std::path::Path) {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let mock = r#"#!/usr/bin/python3
import json, os, pathlib, shutil, sys
home = pathlib.Path(os.environ['HOME'])
verb = pathlib.Path(sys.argv[0]).name
plugins = home / '.config/omarchy/plugins'
plugins.mkdir(parents=True, exist_ok=True)
with (home / 'calls').open('a') as log:
    log.write(' '.join([verb] + sys.argv[1:]) + '\n')
if verb == 'git':
    assert sys.argv[1:3] == ['clone', '--'], sys.argv
    url, dest = sys.argv[3], pathlib.Path(sys.argv[4])
    name = url.split('/')[-1].removesuffix('.git')
    if name == 'fileblade-mcp' and not (home / 'network-restored').exists():
        print('network unavailable', file=sys.stderr)
        sys.exit(128)
    present = sorted(p.name for p in plugins.iterdir() if p.name.startswith('data-goblin.'))
    (home / 'present-at-clone').open('a').write(name + ' ' + ','.join(present) + '\n')
    dest.mkdir(parents=True)
    (dest / 'manifest.json').write_text(json.dumps({'id': 'data-goblin.' + name}))
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
        4
    );
}

#[test]
fn batch_recovers_a_disabled_clone_and_retries_only_missing_repositories() {
    use std::fs;

    let temporary = tempfile::tempdir().unwrap();
    let scripts = temporary.path().join("bin");
    fs::create_dir(&scripts).unwrap();
    install_mock(&scripts);
    let plugins = temporary.path().join(".config/omarchy/plugins");
    let memory = plugins.join("data-goblin.fileblade-memory");
    fs::create_dir_all(&memory).unwrap();
    fs::write(
        memory.join("manifest.json"),
        r#"{"id":"data-goblin.fileblade-memory"}"#,
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
    assert!(!calls.contains("fileblade-memory.git"));
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
fn plugin_add_treats_an_existing_plugin_id_as_installed() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let program = temporary.path().join("omarchy-plugin-add");
    fs::write(
        &program,
        "#!/bin/sh\nprintf \"omarchy-plugin-add: plugin id 'data-goblin.fileblade-memory' is already used by /plugins/data-goblin.fileblade-memory/manifest.json\\n\" >&2\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fileblade"))
        .args([
            "_backend",
            "plugin-add",
            "--url",
            fileblade::plugin_install::ALLOWED_URLS[0],
        ])
        .env("PATH", temporary.path())
        .env("HOME", temporary.path())
        .env("XDG_STATE_HOME", temporary.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["alreadyInstalled"], true);
    assert_eq!(result["message"], "");
}
