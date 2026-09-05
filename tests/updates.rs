use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use fileblade::updates::{RepositorySpec, check, parse_specs};

fn scratch(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("fileblade-updates-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn git(cwd: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .status()
        .unwrap();
    assert!(status.success(), "git {arguments:?}");
}

fn git_stdout(cwd: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn manifest(version: &str) -> String {
    format!(
        "{{\"schemaVersion\":1,\"id\":\"t.plugin\",\"name\":\"T\",\"version\":\"{version}\",\"kinds\":[\"service\"],\"entryPoints\":{{\"service\":\"Service.qml\"}}}}\n"
    )
}

struct Fixture {
    remote: PathBuf,
    installed: PathBuf,
    author: PathBuf,
}

fn fixture(name: &str) -> Fixture {
    let root = scratch(name);
    let remote = root.join("remote.git");
    let author = root.join("author");
    let installed = root.join("installed");
    git(&root, &["init", "-q", "--bare", "-b", "main", "remote.git"]);
    fs::create_dir_all(&author).unwrap();
    git(&author, &["init", "-q", "-b", "main"]);
    fs::write(author.join("manifest.json"), manifest("1.0.0")).unwrap();
    fs::write(author.join("Service.qml"), "import QtQuick\nItem {}\n").unwrap();
    git(&author, &["add", "."]);
    git(&author, &["commit", "-q", "-m", "first"]);
    git(
        &author,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&author, &["push", "-q", "-u", "origin", "main"]);
    git(
        &root,
        &["clone", "-q", remote.to_str().unwrap(), "installed"],
    );
    Fixture {
        remote,
        installed,
        author,
    }
}

fn publish(fixture: &Fixture, version: &str, subject: &str) {
    fs::write(fixture.author.join("manifest.json"), manifest(version)).unwrap();
    git(&fixture.author, &["commit", "-q", "-am", subject]);
    git(&fixture.author, &["push", "-q", "origin", "main"]);
}

fn spec(id: &str, path: &Path) -> RepositorySpec {
    RepositorySpec {
        id: id.to_string(),
        path: path.to_path_buf(),
    }
}

#[test]
fn specs_parse_only_well_formed_absolute_entries() {
    let parsed = parse_specs(&[
        "data-goblin.fileblade=/tmp/a".to_string(),
        "bad id=/tmp/b".to_string(),
        "relative=tmp/c".to_string(),
        "noequals".to_string(),
    ]);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "data-goblin.fileblade");
}

#[test]
fn check_reports_behind_commits_versions_and_subjects() {
    let fixture = fixture("check");
    let cancelled = AtomicBool::new(false);
    let specs = [spec("t.plugin", &fixture.installed)];
    let clean = check(&specs, "t.plugin", &cancelled);
    assert_eq!(clean["available"], false);
    assert_eq!(clean["repositories"][0]["behind"], 0);
    assert_eq!(clean["repositories"][0]["core"], true);

    let installed_head = git_stdout(&fixture.installed, &["rev-parse", "HEAD"]);
    publish(&fixture, "1.1.0", "second");
    let behind = check(&specs, "", &cancelled);
    let row = &behind["repositories"][0];
    assert_eq!(behind["available"], true);
    assert_eq!(row["behind"], 1);
    assert_eq!(row["ahead"], 0);
    assert_eq!(row["dirty"], false);
    assert_eq!(row["updatable"], true);
    assert_eq!(row["current_version"], "1.0.0");
    assert_eq!(row["upstream_version"], "1.1.0");
    assert_eq!(row["subjects"][0], "second");
    assert_eq!(row["core"], false);
    assert_eq!(
        git_stdout(&fixture.installed, &["rev-parse", "HEAD"]),
        installed_head
    );
    assert!(
        fs::read_to_string(fixture.installed.join("manifest.json"))
            .unwrap()
            .contains("1.0.0"),
        "checking must not update the working tree"
    );

    fs::write(
        fixture.installed.join("Service.qml"),
        "import QtQuick\nItem { id: x }\n",
    )
    .unwrap();
    let dirty = check(&specs, "", &cancelled);
    assert_eq!(dirty["repositories"][0]["dirty"], true);
    assert_eq!(dirty["repositories"][0]["updatable"], false);
    assert_eq!(dirty["available"], false);
    let _ = fs::remove_dir_all(fixture.remote.parent().unwrap());
}

#[test]
fn check_refuses_non_checkouts_and_missing_upstreams() {
    let root = scratch("plain");
    let plain = root.join("plain");
    fs::create_dir_all(&plain).unwrap();
    let cancelled = AtomicBool::new(false);
    let report = check(&[spec("t.plain", &plain)], "", &cancelled);
    assert_eq!(report["repositories"][0]["ok"], false);
    assert_eq!(report["repositories"][0]["error"], "not a git checkout");

    let local = root.join("local");
    fs::create_dir_all(&local).unwrap();
    git(&local, &["init", "-q", "-b", "main"]);
    fs::write(local.join("a"), "a").unwrap();
    git(&local, &["add", "."]);
    git(&local, &["commit", "-q", "-m", "a"]);
    let report = check(&[spec("t.local", &local)], "", &cancelled);
    assert_eq!(report["repositories"][0]["error"], "no upstream branch");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn backend_exposes_update_check_but_not_update_apply() {
    assert!(fileblade::backend::parse(["fileblade", "update-check"]).is_ok());
    assert!(fileblade::backend::parse(["fileblade", "update-apply"]).is_err());
}
