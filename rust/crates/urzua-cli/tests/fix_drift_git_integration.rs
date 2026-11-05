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
    Command::new(env!("CARGO_BIN_EXE_urzua"))
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
    let blames = |args: &[&str]| -> String {
        let _ = std::fs::remove_file(&log);
        let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
            .args(args)
            .current_dir(&dir)
            .env("PATH", &path)
            .output()
            .unwrap();
        assert!(out.status.code().is_some_and(|c| c <= 1), "{out:?}");
        std::fs::read_to_string(&log).unwrap_or_default()
    };

    let scoped = blames(&["check", "docs/adr/ADR-1-x.md"]);
    assert!(scoped.contains("ADR-1-x.md"), "{scoped}");
    assert!(!scoped.contains("ADR-2-y.md"), "{scoped}");

    let whole = blames(&["check"]);
    assert!(
        whole.contains("ADR-1-x.md") && whole.contains("ADR-2-y.md"),
        "{whole}"
    );
}
