use fileblade::trash::TrashContext;
use rustix::process::getuid;
use serde_json::Value;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tempfile::TempDir;

#[test]
fn freedesktop_trash_is_merged_bounded_and_collision_safe() {
    let root = TempDir::new().expect("temporary root");
    let home_trash = root.path().join("data/Trash");
    create_store(&home_trash);

    let mount = root.path().join("mounted");
    let shared = mount.join(".Trash");
    fs::create_dir_all(&shared).expect("shared Trash");
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o1777)).expect("sticky Trash");
    let mounted_store = shared.join(getuid().as_raw().to_string());
    create_store(&mounted_store);
    let private_mounted_store = mount.join(format!(".Trash-{}", getuid().as_raw()));
    create_store(&private_mounted_store);

    let unsafe_mount = root.path().join("unsafe-mounted");
    let unsafe_shared = unsafe_mount.join(".Trash");
    fs::create_dir_all(&unsafe_shared).expect("unsafe shared Trash");
    fs::set_permissions(&unsafe_shared, fs::Permissions::from_mode(0o0777))
        .expect("non-sticky Trash");
    create_store(&unsafe_shared.join(getuid().as_raw().to_string()));

    let original = root.path().join("restored/report name.txt");
    create_item(
        &home_trash,
        "report.txt.2",
        &original,
        b"first version",
        "2026-09-02T08:09:10",
    );
    fs::write(home_trash.join("files/orphan"), b"orphaned").expect("orphan content");
    write_info(
        &home_trash,
        "gone",
        &encoded_path(&root.path().join("restored/gone.txt")),
        "2026-09-01T08:09:10",
    );

    let directory_name = "folder.internal";
    fs::create_dir(home_trash.join("files").join(directory_name)).expect("trashed directory");
    write_info(
        &home_trash,
        directory_name,
        &encoded_path(&root.path().join("restored/folder")),
        "2026-08-31T08:09:10",
    );
    let info_mtime = fs::metadata(home_trash.join("info/folder.internal.trashinfo"))
        .expect("directory info metadata")
        .mtime();
    fs::write(
        home_trash.join("directorysizes"),
        format!("123 {info_mtime} {directory_name}\n"),
    )
    .expect("directory sizes");

    let victim = root.path().join("must-survive");
    fs::write(&victim, b"safe").expect("victim");
    fs::write(home_trash.join("files/linked-info"), b"linked").expect("linked content");
    symlink(&victim, home_trash.join("info/linked-info.trashinfo")).expect("metadata symlink");

    fs::write(home_trash.join("files/oversized"), b"large metadata").expect("large item");
    let mut oversized =
        b"[Trash Info]\nPath=/tmp/oversized\nDeletionDate=2026-09-02T08:09:10\n".to_vec();
    oversized.resize(70 * 1024, b'x');
    fs::write(home_trash.join("info/oversized.trashinfo"), oversized).expect("oversized metadata");

    fs::write(mounted_store.join("files/mounted.internal"), b"mounted").expect("mounted content");
    write_info(
        &mounted_store,
        "mounted.internal",
        "restore/mounted.txt",
        "2026-09-02T07:00:00",
    );
    fs::write(
        private_mounted_store.join("files/private.internal"),
        b"private mounted",
    )
    .expect("private mounted content");
    write_info(
        &private_mounted_store,
        "private.internal",
        "restore/private.txt",
        "2026-09-02T06:00:00",
    );
    fs::write(mounted_store.join("files/traversal"), b"traversal").expect("traversal content");
    write_info(
        &mounted_store,
        "traversal",
        "../outside.txt",
        "2026-09-02T07:00:00",
    );

    let context = TrashContext::for_roots(
        home_trash.clone(),
        vec![mount.clone(), unsafe_mount.clone()],
    );
    let cancelled = AtomicBool::new(false);
    let listed = context.list(100, &cancelled);
    assert_eq!(listed["ok"], true);
    assert_eq!(listed["stores"], 3);
    assert_eq!(listed["watch_paths"].as_array().map(Vec::len), Some(6));
    assert!(watch_paths_are_validated(
        &listed,
        &home_trash,
        &mounted_store,
        &private_mounted_store
    ));
    assert!(
        listed["errors"]
            .as_array()
            .expect("errors")
            .iter()
            .any(|error| error.as_str().is_some_and(|text| text.contains("unsafe")))
    );

    let rows = listed["entries"].as_array().expect("Trash rows");
    let report = row_named(rows, "report name.txt");
    assert_eq!(report["stored_name"], "report.txt.2");
    assert_eq!(report["original_path"], original.to_string_lossy().as_ref());
    assert_eq!(report["metadata_valid"], true);
    assert_eq!(report["can_restore"], true);
    assert_ne!(report["name"], report["stored_name"]);
    let folder = row_named(rows, "folder");
    assert_eq!(folder["size"], 123);
    assert_eq!(folder["is_dir"], true);
    let mounted = row_named(rows, "mounted.txt");
    assert_eq!(
        mounted["original_path"],
        mount.join("restore/mounted.txt").to_string_lossy().as_ref()
    );
    assert_eq!(mounted["source_mount"], mount.to_string_lossy().as_ref());
    let private_mounted = row_named(rows, "private.txt");
    assert_eq!(
        private_mounted["original_path"],
        mount.join("restore/private.txt").to_string_lossy().as_ref()
    );
    for emergency in ["orphan", "linked-info", "oversized", "traversal"] {
        let row = row_named(rows, emergency);
        assert_eq!(row["emergency"], true);
        assert_eq!(row["can_restore"], false);
        assert_eq!(row["can_delete"], true);
    }

    let stale_id = report["id"].as_str().expect("report id").to_string();
    fs::write(home_trash.join("files/report.txt.2"), b"changed in place")
        .expect("replace trashed data");
    let stale = context.restore(&stale_id, "", false, &cancelled);
    assert_eq!(stale["ok"], false);
    assert!(
        stale["error"]
            .as_str()
            .is_some_and(|error| error.contains("missing or changed"))
    );

    let refreshed = context.list(100, &cancelled);
    let refreshed_rows = refreshed["entries"].as_array().expect("refreshed rows");
    let report_id = row_named(refreshed_rows, "report name.txt")["id"]
        .as_str()
        .expect("refreshed report id")
        .to_string();
    fs::create_dir_all(original.parent().expect("original parent")).expect("restore parent");
    fs::write(&original, b"occupied").expect("collision");
    let collision = context.restore(&report_id, "", false, &cancelled);
    assert_eq!(collision["ok"], false);
    assert_eq!(collision["collision"], true);
    assert!(home_trash.join("files/report.txt.2").exists());
    fs::remove_file(&original).expect("remove collision");
    let restored = context.restore(&report_id, "", false, &cancelled);
    assert_eq!(restored["ok"], true);
    assert_eq!(
        fs::read(&original).expect("restored bytes"),
        b"changed in place"
    );
    assert!(!home_trash.join("info/report.txt.2.trashinfo").exists());

    let after_restore = context.list(100, &cancelled);
    let after_rows = after_restore["entries"]
        .as_array()
        .expect("after restore rows");
    let mounted_id = row_named(after_rows, "mounted.txt")["id"]
        .as_str()
        .expect("mounted id")
        .to_string();
    let restore_to = root.path().join("restore-elsewhere");
    let restored_to = context.restore(
        &mounted_id,
        restore_to.to_string_lossy().as_ref(),
        true,
        &cancelled,
    );
    assert_eq!(restored_to["ok"], true);
    assert_eq!(
        fs::read(restore_to.join("mounted.txt")).expect("restore-to bytes"),
        b"mounted"
    );

    let before_delete = context.list(100, &cancelled);
    let before_delete_rows = before_delete["entries"].as_array().expect("delete rows");
    let orphan_id = row_named(before_delete_rows, "orphan")["id"]
        .as_str()
        .expect("orphan id")
        .to_string();
    let gone_id = row_named(before_delete_rows, "gone.txt")["id"]
        .as_str()
        .expect("gone id")
        .to_string();
    let deleted = context.delete_permanently(&[orphan_id, gone_id], &cancelled);
    assert_eq!(deleted["ok"], true);
    assert_eq!(deleted["completed"], 2);
    assert!(!home_trash.join("files/orphan").exists());
    assert!(!home_trash.join("info/gone.trashinfo").exists());

    create_item(
        &home_trash,
        "expired",
        &root.path().join("restored/expired"),
        b"expired",
        "2000-01-01T00:00:00",
    );
    create_item(
        &home_trash,
        "future",
        &root.path().join("restored/future"),
        b"future",
        "2999-01-01T00:00:00",
    );
    fs::write(home_trash.join("files/unknown-age"), b"unknown").expect("unknown content");
    write_info(
        &home_trash,
        "unknown-age",
        &encoded_path(&root.path().join("restored/unknown-age")),
        "not-a-date",
    );
    let pruned = context.prune(7, &cancelled, &mut |_| {});
    assert_eq!(pruned["ok"], true, "{pruned}");
    assert_eq!(pruned["eligible"], 1);
    assert_eq!(pruned["completed"], 1);
    assert!(pruned["unknown_age"].as_u64().unwrap_or(0) >= 1);
    assert!(!home_trash.join("files/expired").exists());
    assert!(home_trash.join("files/future").exists());
    assert!(home_trash.join("files/unknown-age").exists());

    create_item(
        &home_trash,
        "cancel-one",
        &root.path().join("restored/cancel-one"),
        b"one",
        "2026-09-02T10:00:00",
    );
    create_item(
        &home_trash,
        "cancel-two",
        &root.path().join("restored/cancel-two"),
        b"two",
        "2026-09-02T10:00:01",
    );
    let mut phases = Vec::new();
    let partial = context.empty(&cancelled, &mut |event| {
        if event["phase"] == "deleting" {
            cancelled.store(true, Ordering::Relaxed);
        }
        phases.push(event);
    });
    assert_eq!(partial["ok"], false);
    assert_eq!(partial["cancelled"], true);
    assert_eq!(partial["completed"], 1);
    assert!(phases.iter().any(|event| event["phase"] == "complete"));
    assert_eq!(fs::read(&victim).expect("symlink target survives"), b"safe");

    cancelled.store(false, Ordering::Relaxed);
    let emptied = context.empty(&cancelled, &mut |_| {});
    assert_eq!(emptied["ok"], true, "{emptied}");
    assert_eq!(context.list(100, &cancelled)["count"], 0);
    assert_eq!(
        fs::read(&victim).expect("symlink target still survives"),
        b"safe"
    );

    cancelled.store(true, Ordering::Relaxed);
    let cancelled_list = context.list(100, &cancelled);
    assert_eq!(cancelled_list["ok"], false);
    assert_eq!(cancelled_list["cancelled"], true);
}

fn create_store(path: &Path) {
    fs::create_dir_all(path.join("info")).expect("Trash info directory");
    fs::create_dir_all(path.join("files")).expect("Trash files directory");
    for directory in [path, &path.join("info"), &path.join("files")] {
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .expect("private Trash directory");
    }
}

fn create_item(
    store: &Path,
    stored_name: &str,
    original: &Path,
    contents: &[u8],
    deletion_date: &str,
) {
    fs::write(store.join("files").join(stored_name), contents).expect("stored content");
    write_info(store, stored_name, &encoded_path(original), deletion_date);
}

fn write_info(store: &Path, stored_name: &str, path: &str, deletion_date: &str) {
    fs::write(
        store.join("info").join(format!("{stored_name}.trashinfo")),
        format!("[Trash Info]\nPath={path}\nDeletionDate={deletion_date}\n"),
    )
    .expect("Trash metadata");
}

fn encoded_path(path: &Path) -> String {
    let mut output = String::new();
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            output.push(*byte as char);
        } else {
            output.push('%');
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    output
}

fn row_named<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing Trash row {name}: {rows:?}"))
}

fn watch_paths_are_validated(
    listed: &Value,
    home: &Path,
    mounted: &Path,
    private_mounted: &Path,
) -> bool {
    let expected = [
        home.join("info"),
        home.join("files"),
        mounted.join("info"),
        mounted.join("files"),
        private_mounted.join("info"),
        private_mounted.join("files"),
    ]
    .into_iter()
    .map(|path| path.to_string_lossy().into_owned())
    .collect::<Vec<_>>();
    let actual = listed["watch_paths"]
        .as_array()
        .expect("watch paths")
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    actual == expected
}
