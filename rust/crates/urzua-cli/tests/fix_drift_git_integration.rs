//! `MILE-159`: git trouble in the drift computation is a notice, never a
//! quiet "no drift", and `fix` pays for drift only when the rule is on.
//!
//! Verifies: SPEC-2, SPEC-3

mod common;

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

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn repo(name: &str, rule: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-fixgit-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  {rule}\n\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\", \"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("code.rs"), "fn a() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: code:code.rs\n---\n# 1 — X\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "one"]);
    std::fs::write(dir.join("code.rs"), "fn a() { }\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "two"]);
    dir
}

fn shallow_clone(src: &Path, name: &str) -> common::TempDir {
    let dst = common::TempDir::new(&format!("urzua-fixgit-{name}-clone"));
    let status = Command::new("git")
        .args(["clone", "-q", "--depth", "1"])
        .arg(format!("file://{}", src.display()))
        .arg(&dst)
        .status()
        .unwrap();
    assert!(status.success());
    dst
}

fn notice_messages(out: &std::process::Output) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    v["notices"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|n| n["subject"] == "embodiment-drift-git")
                .map(|n| n["message"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn check_and_fix_name_a_shallow_clone_instead_of_reporting_no_drift() {
    let src = repo("shallow", "embodiment.consistency: warn");
    let clone = shallow_clone(&src, "shallow");
    for cmd in ["check", "fix"] {
        let out = run_urzua(&clone, &[cmd]);
        let msgs = notice_messages(&out);
        assert!(
            msgs.iter().any(|m| m.contains("shallow clone")),
            "{cmd} must say the clone is shallow: {}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
    let full = run_urzua(&src, &["check"]);
    assert!(
        notice_messages(&full).is_empty(),
        "a full clone has no drift notice"
    );
}

#[test]
fn fix_does_not_run_when_embodiment_consistency_is_off() {
    let dir = repo("off", "embodiment.consistency: off");
    let out = run_urzua(&dir, &["fix"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("embodiment.consistency"), "{text}");
}

/// A runner for urzua with a `git` shim first on `PATH` that logs the arguments
/// of every `git blame` and then runs the real git. The runner returns the log
/// of one run, so a test counts the blame process spawns instead of timing them.
/// `stdin` is fed to urzua when given.
fn blame_runner(dir: &Path) -> impl Fn(&[&str], Option<&str>) -> String + '_ {
    let real_git = String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let shim_dir = common::TempDir::new("urzua-fixgit-shim");
    let log = shim_dir.join("calls.log");
    let shim = shim_dir.join("git");
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\nfor a in \"$@\"; do [ \"$a\" = blame ] && echo \"$*\" >> '{}'; done\nexec '{}' \"$@\"\n",
            log.display(),
            real_git.trim()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!(
        "{}:{}",
        shim_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    move |args: &[&str], stdin: Option<&str>| -> String {
        use std::io::Write;
        use std::process::Stdio;
        let _keep = &shim_dir;
        let _ = std::fs::remove_file(&log);
        let mut child = common::urzua()
            .args(args)
            .current_dir(dir)
            .env("PATH", &path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        if let Some(text) = stdin {
            input.write_all(text.as_bytes()).unwrap();
        }
        drop(input);
        let out = child.wait_with_output().unwrap();
        assert!(out.status.code().is_some_and(|c| c <= 1), "{out:?}");
        std::fs::read_to_string(&log).unwrap_or_default()
    }
}

/// A run narrowed to one path reports on that record alone, so it must not pay
/// for the git history of every other record: the blame calls name only the
/// record in scope, and the whole-corpus run still blames both.
#[test]
fn a_path_scoped_check_blames_only_the_records_in_scope() {
    let dir = repo("scoped-blame", "embodiment.consistency: warn");
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: code:code.rs\n---\n# 2 — Y\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "second record"]);

    let blames = blame_runner(&dir);

    let scoped = blames(&["check", "docs/adr/ADR-1-x.md"], None);
    assert!(scoped.contains("ADR-1-x.md"), "{scoped}");
    assert!(!scoped.contains("ADR-2-y.md"), "{scoped}");

    let whole = blames(&["check"], None);
    assert!(
        whole.contains("ADR-1-x.md") && whole.contains("ADR-2-y.md"),
        "{whole}"
    );
}

/// MILE-159: a single-file run is cheap on a large corpus because its cost does
/// not grow with the corpus. Time is not asserted (it depends on the machine);
/// the number of `git blame` process spawns is. One record in scope costs one
/// blame whether the corpus holds 2 records or 40, and the whole-corpus run
/// costs one per record.
#[test]
fn a_single_file_run_spawns_one_blame_however_large_the_corpus_is() {
    const RECORDS: usize = 40;
    let dir = repo("bounded-blame", "embodiment.consistency: warn");
    for n in 2..=RECORDS {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-r.md")),
            format!(
                "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: code:code.rs\n---\n# {n} — R\n"
            ),
        )
        .unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "many records"]);
    let blames = blame_runner(&dir);
    let count = |log: &str| log.lines().count();

    let one_path = blames(&["fix", "docs/adr/ADR-1-x.md"], None);
    assert_eq!(count(&one_path), 1, "path run: {one_path}");

    let one_check = blames(&["check", "docs/adr/ADR-1-x.md"], None);
    assert_eq!(count(&one_check), 1, "check path run: {one_check}");

    let stdin_record =
        "---\nStatus: Accepted\nEmbodiment: Implemented\nRealized-by: code:code.rs\n---\n# 1 — X\n";
    let one_stdin = blames(
        &["fix", "--stdin-path", "docs/adr/ADR-1-x.md"],
        Some(stdin_record),
    );
    assert!(count(&one_stdin) <= 1, "stdin run: {one_stdin}");

    let whole = blames(&["fix"], None);
    assert_eq!(count(&whole), RECORDS, "whole-corpus run: {whole}");
}
