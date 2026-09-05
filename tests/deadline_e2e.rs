mod support;

use std::fs;
use support::Resident;
use tempfile::tempdir;

fn slow_tree(directory: &std::path::Path, name: &str, files: usize) -> String {
    let path = directory.join(name);
    fs::create_dir(&path).unwrap();
    for index in 0..files {
        fs::write(path.join(format!("entry-{index:04}.txt")), "x").unwrap();
    }
    path.to_string_lossy().into_owned()
}

#[test]
fn a_mutation_that_outlives_its_deadline_completes_and_is_reported_ok_and_late() {
    let temporary = tempdir().unwrap();
    let root = temporary.path();
    let source = slow_tree(root, "payload", 3000);
    let destination = root.join("out");
    fs::create_dir(&destination).unwrap();

    let mut resident = Resident::start(4);
    resident.request(
        "copy-late",
        "copy",
        &[
            "--source".to_string(),
            source,
            "--destination".to_string(),
            destination.to_string_lossy().into_owned(),
        ],
        1,
    );
    let response = resident.response("copy-late");
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(response["late"], true, "{response}");
    let payload = &response["payload"];
    assert_eq!(payload["ok"], true, "{payload}");
    assert_eq!(payload["late"], true, "{payload}");
    assert_eq!(payload["mappings"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        fs::read_dir(destination.join("payload")).unwrap().count(),
        3000,
        "the copy must run to completion instead of being cancelled at the deadline"
    );
    resident.finish();
}

#[test]
fn a_failed_transfer_keeps_the_completed_mappings_even_past_the_deadline() {
    let temporary = tempdir().unwrap();
    let root = temporary.path();
    let first = slow_tree(root, "first", 3000);
    let missing = root.join("missing.bin").to_string_lossy().into_owned();
    let destination = root.join("out");
    fs::create_dir(&destination).unwrap();

    let mut resident = Resident::start(4);
    resident.request(
        "copy-partial",
        "copy",
        &[
            "--source".to_string(),
            first,
            "--source".to_string(),
            missing,
            "--destination".to_string(),
            destination.to_string_lossy().into_owned(),
        ],
        1,
    );
    let response = resident.response("copy-partial");
    assert_eq!(response["ok"], true, "{response}");
    let payload = &response["payload"];
    assert_eq!(payload["ok"], false, "{payload}");
    assert_eq!(payload["late"], true, "{payload}");
    assert_eq!(
        payload["mappings"].as_array().map(Vec::len),
        Some(1),
        "the first item finished and its mapping must reach the client"
    );
    assert_eq!(
        fs::read_dir(destination.join("first")).unwrap().count(),
        3000
    );
    resident.finish();
}

#[test]
fn a_read_past_its_deadline_answers_late_or_cancelled_and_the_next_read_works() {
    let temporary = tempdir().unwrap();
    let root = temporary.path();
    let wide = root.join("wide");
    fs::create_dir(&wide).unwrap();
    for index in 0..3000 {
        fs::write(wide.join(format!("entry-{index:04}.txt")), "x").unwrap();
    }

    let mut resident = Resident::start(1);
    let arguments = vec![
        "--path".to_string(),
        wide.to_string_lossy().into_owned(),
        "--show-hidden".to_string(),
    ];
    resident.request("read-late", "children-window", &arguments, 1);
    let late = resident.response("read-late");
    assert!(
        late["late"] == true || late["deadline_exceeded"] == true,
        "a read past its deadline is either late or cancelled: {late}"
    );
    resident.request("read-after", "children-window", &arguments, 5000);
    let after = resident.response("read-after");
    assert_eq!(after["ok"], true, "{after}");
    resident.finish();
}
