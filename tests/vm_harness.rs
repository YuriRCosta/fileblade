use std::process::Command;

#[test]
fn an_unavailable_guest_fails_instead_of_reporting_an_empty_success() {
    let output = Command::new("bash")
        .args(["-c", "source tests/vm/expectations/lib.sh; require_guest"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("OVM", "/usr/bin/false")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("FAIL harness"), "{report}");
    assert!(report.contains("1 checks, 1 failed"), "{report}");
}

#[test]
fn a_missing_row_stops_the_scenario_before_input_can_reach_another_item() {
    let output = Command::new("bash")
        .args([
            "-c",
            "source tests/vm/expectations/lib.sh; row_index() { echo null; }; click_row missing.txt",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("OVM", "/usr/bin/false")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("FAIL harness")
    );
}

#[test]
fn row_ocr_does_not_depend_on_recognizing_a_dim_section_heading() {
    let output = Command::new("bash")
        .args([
            "-c",
            r#"source tests/vm/expectations/lib.sh
shot() { echo tests/vm_harness.rs; }
field() { echo 380; }
magick() { :; }
tesseract() { printf '5\t1\t1\t1\t1\t1\t201\t1971\t225\t30\t95\tundoable.txt\n'; }
OVM=shot
visible_row_y undoable.txt"#,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("OVM", "/usr/bin/false")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "662");
}

#[test]
fn row_ocr_does_not_substitute_a_similar_filename() {
    let output = Command::new("bash")
        .args([
            "-c",
            r#"source tests/vm/expectations/lib.sh
shot() { echo tests/vm_harness.rs; }
field() { echo 380; }
magick() { :; }
tesseract() { printf '5\t1\t1\t1\t1\t1\t201\t1971\t225\t30\t71\t2z-one.txt\n'; }
OVM=shot
visible_row_y zz-one.txt"#,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("OVM", "/usr/bin/false")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn picker_ocr_uses_the_current_blade_width_and_only_the_picker_card() {
    let output = Command::new("bash")
        .args([
            "-c",
            r#"source tests/vm/expectations/lib.sh
field() { echo 420; }
ocr_crop() { printf '%s\n' "$@"; }
picker_text
field() { echo null; }
if picker_text; then exit 1; fi"#,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("OVM", "/usr/bin/false")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ocr-picker\n400x120+10+58\n400%\n6\n5%,40%\n"
    );
}

#[test]
fn an_unchanged_selection_cannot_validate_a_pointer_click() {
    let output = Command::new("bash")
        .args([
            "-c",
            r#"source tests/vm/expectations/lib.sh
row_index() { echo 1; }
ctl() { :; }
guest() { :; }
field() { echo /fixture/target.txt; }
click_row target.txt"#,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("OVM", "/usr/bin/false")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("could not move the cursor off")
    );
}
