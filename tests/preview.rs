use fileblade::preview;
use std::fs;
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;

#[test]
fn preview_returns_lines_marks_binaries_and_colours_code_when_bat_exists() {
    let temporary = tempdir().unwrap();
    let root = temporary.path();
    let source = root.join("main.rs");
    fs::write(&source, "fn main() {\n    let answer = 42;\n}\n").unwrap();
    fs::write(root.join("blob.bin"), b"\x00\x01\x02binary").unwrap();
    let cancelled = AtomicBool::new(false);
    let text = preview::preview(source.to_str().unwrap(), 10, &cancelled);
    assert_eq!(text["ok"], true, "{text}");
    assert_eq!(text["binary"], false);
    let lines = text["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    let first: String = lines[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["text"].as_str().unwrap())
        .collect();
    assert_eq!(first, "fn main() {");
    if text["backend"] == "bat" {
        assert!(
            lines
                .iter()
                .flat_map(|line| line.as_array().unwrap())
                .any(|run| run["color"].as_i64().unwrap() >= 0)
        );
    }
    let binary = preview::preview(root.join("blob.bin").to_str().unwrap(), 10, &cancelled);
    assert_eq!(binary["binary"], true);
    assert!(binary["lines"].as_array().unwrap().is_empty());
    let limited = preview::preview(source.to_str().unwrap(), 1, &cancelled);
    assert_eq!(limited["truncated"], true);
    assert_eq!(limited["lines"].as_array().unwrap().len(), 1);
}
