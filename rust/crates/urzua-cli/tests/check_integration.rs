//! CLI-golden integration tests: build a real, isolated git-repo fixture and
//! run the actual compiled binary against it. The fixture lives in its own
//! temp git repo -- entirely outside this project's own discovered record
//! set, so the containment SPEC-0002 requires is true by construction, not
//! assumed.

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn fixture_repo(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-cli-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    dir
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

#[test]
fn check_exits_0_on_a_clean_corpus() {
    let dir = fixture_repo("clean");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.toml"),
        "schema_version = 1\n\n[record_types.adr]\ndir = \"docs/adr\"\nrequired_fields = [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_exits_1_on_a_missing_required_field() {
    let dir = fixture_repo("findings");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.toml"),
        "schema_version = 1\n\n[record_types.adr]\ndir = \"docs/adr\"\nrequired_fields = [\"Status\", \"Deciders\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deciders"), "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_exits_2_when_no_config_exists() {
    let dir = fixture_repo("noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(output.status.code(), Some(2));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_untracked_scratch_file_is_never_examined() {
    // The exact regression this project's own design calls out: a raw
    // filesystem walk would pick this up; git-tracked discovery must not.
    let dir = fixture_repo("scratch");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.toml"),
        "schema_version = 1\n\n[record_types.adr]\ndir = \"docs/adr\"\nrequired_fields = []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    std::fs::write(dir.join("docs/adr/0002-untracked.md"), "not a real record").unwrap();

    let output = run_urzua(&dir, &["check", "docs/", "--format", "json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"files_examined\": 1"),
        "expected exactly 1 file examined (the untracked file must be excluded): {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_waiver_record_suppresses_blocking_but_the_finding_stays_listed() {
    let dir = fixture_repo("waiver");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.toml"),
        "schema_version = 1\n\n\
         [record_types.adr]\ndir = \"docs/adr\"\nrequired_fields = []\n\n\
         [record_types.waiver]\ndir = \"docs/waiver\"\nrequired_fields = []\n",
    )
    .unwrap();
    // A dangling pointer -- pointer.resolution's one error case, and not
    // redundant with any other rule, so waiving it is the only thing
    // standing between this fixture and a clean exit.
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n> Implements: RFC-9999\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waiver/0001-w.md"),
        "# 0001 — Waive the dangling RFC-9999 reference on ADR-0001\n\n\
         > Rule: pointer.resolution\n\
         > Scope: docs/adr/0001-x.md\n\
         > Reason: RFC-9999 is tracked externally, not yet in this corpus\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/", "--format", "json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a waived finding must not block: {stdout}"
    );
    assert!(
        stdout.contains("\"waived\""),
        "the waived finding must still be listed, not omitted: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_reports_missing_config_as_exit_2() {
    let dir = fixture_repo("doctor-noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(2));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn init_then_check_round_trips_on_a_fresh_corpus() {
    let dir = fixture_repo("bootstrap");
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let init_output = run_urzua(&dir, &["init"]);
    assert_eq!(
        init_output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );
    assert!(dir.join(".urzua/config.toml").exists());

    let check_output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(check_output.status.code(), Some(0));

    // Idempotence: a second init must refuse, not clobber.
    let second_init = run_urzua(&dir, &["init"]);
    assert_eq!(second_init.status.code(), Some(2));

    std::fs::remove_dir_all(&dir).ok();
}
