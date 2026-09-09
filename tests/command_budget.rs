use fileblade::command::CommandSpec;
use std::time::Duration;

#[test]
fn a_file_budget_stops_a_child_before_it_can_fill_staging() {
    let root = tempfile::tempdir().unwrap();
    let result = CommandSpec::new("/usr/bin/python3")
        .args([
            "-c",
            "with open('data', 'wb') as output: output.write(b'x' * 16384); output.flush()",
        ])
        .cwd(root.path())
        .resource_limits(4096, 256 * 1024 * 1024)
        .timeout(Duration::from_secs(5))
        .run()
        .unwrap();
    assert!(!result.status.success());
    assert!(std::fs::metadata(root.path().join("data")).unwrap().len() <= 4096);
}

#[test]
fn aggregate_staging_and_output_budgets_refuse_incomplete_results() {
    let root = tempfile::tempdir().unwrap();
    let result = CommandSpec::new("/usr/bin/python3")
        .args([
            "-c",
            "open('a', 'wb').write(b'x' * 4096); open('b', 'wb').write(b'y' * 4096)",
        ])
        .cwd(root.path())
        .directory_budget(root.path(), 4096, 16)
        .timeout(Duration::from_secs(5))
        .run();
    assert!(result.unwrap_err().to_string().contains("byte limit"));
    let result = CommandSpec::new("/usr/bin/python3")
        .args(["-c", "print('x' * 1000)"])
        .limits(64, 64)
        .stop_on_output_limit()
        .run();
    assert!(result.unwrap_err().to_string().contains("output limit"));
}
