use fileblade::audit;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use tempfile::tempdir;

#[test]
fn a_log_past_the_read_cap_with_a_multibyte_path_at_the_cut_still_reads() {
    let temporary = tempdir().unwrap();
    let state = temporary.path().join("state");
    unsafe { std::env::set_var("XDG_STATE_HOME", &state) };
    let path = audit::path();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    let line = format!(
        "{{\"ts\":\"2026-09-03T10:00:00.000Z\",\"via\":\"cli\",\"actor\":\"cli\",\"command\":\"trash\",\"arguments\":[\"--path\",\"/home/me/Dokumente/Übersicht-{}.txt\"],\"ok\":true,\"error\":\"\",\"elapsed_ms\":1,\"result\":null,\"pid\":1,\"display\":\"\",\"home\":\"\"}}\n",
        "ü".repeat(40)
    );
    let mut written = 0usize;
    while written < audit::AUDIT_READ_CAP + 64 * 1024 {
        file.write_all(line.as_bytes()).unwrap();
        written += line.len();
    }
    drop(file);
    let size = fs::metadata(&path).unwrap().len();
    assert!(size > audit::AUDIT_READ_CAP as u64);

    let document = audit::read(5, "", "");
    assert_eq!(document["ok"], true, "{document}");
    assert_eq!(document["truncated"], true);
    let entries = document["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 5, "{document}");
    assert_eq!(entries[0]["command"], "trash");
}
