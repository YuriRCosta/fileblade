use fileblade::command::CommandSpec;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn detached_children_are_reaped() {
    let pid = CommandSpec::new("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn_detached()
        .expect("detached command should start");
    let process = PathBuf::from(format!("/proc/{pid}"));
    let deadline = Instant::now() + Duration::from_secs(2);
    while process.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!process.exists(), "detached child remained as a zombie");
}

#[test]
fn a_daemonising_child_does_not_hold_the_runner() {
    let started = Instant::now();
    let output = CommandSpec::new("/bin/sh")
        .args(["-c", "sleep 30 & printf done; exit 0"])
        .timeout(Duration::from_secs(5))
        .run()
        .expect("command should finish once the direct child exits");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"done");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "runner waited on a background descendant for {:?}",
        started.elapsed()
    );
}

#[test]
fn a_daemonising_child_with_closed_pipes_returns_at_once() {
    let started = Instant::now();
    let output = CommandSpec::new("/bin/sh")
        .args(["-c", "sleep 30 >/dev/null 2>&1 & printf done; exit 0"])
        .timeout(Duration::from_secs(5))
        .run()
        .expect("command should finish once the direct child exits");
    assert_eq!(output.stdout, b"done");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
}
