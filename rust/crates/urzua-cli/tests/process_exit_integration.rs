//! Process-level exit behaviour (MILE-176): a panic exits 2, a closed stdout
//! does not panic.
use std::io::Read;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

#[test]
fn a_panic_exits_2_with_a_not_run_result() {
    let out = bin()
        .arg("check")
        .env("URZUA_TEST_PANIC", "1")
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("\"not-run\""));
}

#[test]
fn a_closed_stdout_does_not_panic() {
    let mut child = bin()
        .args(["check", "--fail-on", "error"])
        .current_dir(repo_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close the read end before the child writes.
    drop(child.stdout.take());
    let mut err = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(!err.contains("panicked"), "{err}");
    assert_ne!(status.code(), Some(101));
    assert_ne!(status.code(), Some(2), "must not be the panic path: {err}");
}
