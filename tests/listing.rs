use fileblade::listing::{self, WindowRequest};
use serde_json::json;
use std::fs;
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;

fn request(path: &str) -> WindowRequest {
    WindowRequest {
        path: path.to_string(),
        show_hidden: false,
        start: 0,
        count: 400,
        sort: "name".to_string(),
        descending: false,
        filter: json!({}),
        include_created: false,
        fresh: false,
        git_enabled: true,
        fresh_git: false,
    }
}

fn names(response: &serde_json::Value) -> Vec<String> {
    response["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|row| row["name"].as_str().expect("name").to_string())
        .collect()
}

#[test]
fn windows_page_a_large_directory_in_backend_order() {
    let fixture = tempdir().expect("fixture");
    let root = fixture.path();
    for index in 0..1200 {
        fs::write(
            root.join(format!("file-{index}.txt")),
            vec![b'x'; index % 7 + 1],
        )
        .expect("file");
    }
    for name in ["zeta", "alpha", "Mid"] {
        fs::create_dir(root.join(name)).expect("directory");
    }
    fs::write(root.join(".hidden"), b"h").expect("hidden");
    let path = root.to_string_lossy().to_string();
    let cancelled = AtomicBool::new(false);

    let first = listing::window(&request(&path), &cancelled);
    assert_eq!(first["ok"], true);
    assert_eq!(first["total"], 1203);
    assert_eq!(first["truncated"], true);
    assert_eq!(first["windowed"], true);
    let first_names = names(&first);
    assert_eq!(first_names.len(), 400);
    assert_eq!(&first_names[..3], ["alpha", "Mid", "zeta"]);
    assert_eq!(first_names[3], "file-0.txt");
    assert_eq!(first_names[4], "file-1.txt");
    assert_eq!(first_names[13], "file-10.txt");

    let mut tail = request(&path);
    tail.start = 1200;
    let last = listing::window(&tail, &cancelled);
    assert_eq!(names(&last).len(), 3);
    assert_eq!(last["truncated"], false);
    assert_eq!(names(&last)[2], "file-1199.txt");
    let mut descending = request(&path);
    descending.descending = true;
    descending.count = 5;
    let reversed = names(&listing::window(&descending, &cancelled));
    assert_eq!(&reversed[..3], ["zeta", "Mid", "alpha"]);
    assert_eq!(&reversed[3..], ["file-1199.txt", "file-1198.txt"]);

    let mut hidden = request(&path);
    hidden.show_hidden = true;
    assert_eq!(listing::window(&hidden, &cancelled)["total"], 1204);

    let mut by_size = request(&path);
    by_size.sort = "size".to_string();
    by_size.descending = true;
    by_size.count = 5;
    let biggest = listing::window(&by_size, &cancelled);
    assert_eq!(biggest["partial_metadata"], false);
    let rows = biggest["entries"].as_array().expect("rows");
    assert!(rows[..3].iter().all(|row| row["is_dir"] == true));
    assert_eq!(rows[3]["size"], 7);
    assert_eq!(rows[4]["size"], 7);
    fs::write(root.join("file-0.txt"), vec![b'x'; 100]).unwrap();
    listing::invalidate_within(&root.join("file-0.txt"));
    let refreshed_size = listing::window(&by_size, &cancelled);
    assert_eq!(refreshed_size["entries"][3]["name"], "file-0.txt");
    assert_eq!(refreshed_size["entries"][3]["size"], 100);
    fs::write(root.join("file-0.txt"), b"x").unwrap();
    by_size.fresh = true;
    assert_eq!(
        listing::window(&by_size, &cancelled)["entries"][3]["size"],
        7
    );

    let mut filtered = request(&path);
    filtered.filter = json!({"size": {"min": 7}});
    let large = listing::window(&filtered, &cancelled);
    assert_eq!(large["total"], 3 + 1200 / 7 + usize::from(1200 % 7 > 6));

    let mut with_created = request(&path);
    with_created.include_created = true;
    with_created.count = 5;
    let created = listing::window(&with_created, &cancelled);
    assert_eq!(
        created["entries"][0]["created"],
        fileblade::filesystem::stat_path(created["entries"][0]["path"].as_str().unwrap())["entry"]
            ["created"]
    );

    assert_eq!(
        listing::entry_count(&path, false, &cancelled).expect("count"),
        1203
    );
    fs::write(root.join("file-new.txt"), b"n").expect("new file");
    let refreshed = listing::window(&request(&path), &cancelled);
    assert_eq!(refreshed["total"], 1204);
    listing::forget(&path);
    assert_eq!(listing::window(&request(&path), &cancelled)["total"], 1204);

    let missing = listing::window(&request(&root.join("gone").to_string_lossy()), &cancelled);
    assert_eq!(missing["ok"], false);
    assert_eq!(missing["missing"], true);
}
