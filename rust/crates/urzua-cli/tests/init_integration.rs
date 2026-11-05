//! `urzua init` (SPEC-5): the success criteria and the withheld-rule mechanism.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn adopt_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-init-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join("docs/adr/ADR-0001-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn init(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("init")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

/// SPEC-5 criterion 1.
#[test]
fn a_dry_run_matches_the_real_run_byte_for_byte() {
    let dir = adopt_repo("dry-vs-real");
    let (code, stdout) = init(&dir, &["--dry-run"]);
    assert_eq!(code, Some(0), "{stdout}");
    let preview = serde_json::from_str::<serde_json::Value>(&stdout).unwrap()["config_yaml"]
        .as_str()
        .expect("a dry run carries config_yaml")
        .to_string();
    assert!(!dir.join(".urzua/config.yaml").exists());

    let (code, stdout) = init(&dir, &[]);
    assert_eq!(code, Some(0), "{stdout}");
    let written = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    assert_eq!(preview, written);
    std::fs::remove_dir_all(&dir).ok();
}

/// SPEC-5 criterion 2: only `.urzua/` appears, `.gitignore` is untouched.
#[test]
fn init_changes_only_the_urzua_directory_and_leaves_gitignore_alone() {
    let dir = adopt_repo("files-touched");
    let (code, stdout) = init(&dir, &[]);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        git(&dir, &["status", "--porcelain", "-uall"]).trim(),
        "?? .urzua/config.yaml"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".gitignore")).unwrap(),
        "target/\n"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// SPEC-5 "Proposed rules": every rule whose table entry has a
/// `reads_declaration` is withheld from the config and named in the notice,
/// with no list kept in `init`.
#[test]
fn init_withholds_every_rule_that_reads_a_declaration() {
    let dir = adopt_repo("withholds");
    let (code, stdout) = init(&dir, &[]);
    assert_eq!(code, Some(0), "{stdout}");
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    let mut seen = 0;
    for def in urzua_core::rules::RULE_TABLE {
        let Some(declaration) = def.reads_declaration else {
            continue;
        };
        seen += 1;
        assert!(
            !config.contains(def.id),
            "{} was proposed:\n{config}",
            def.id
        );
        assert!(stdout.contains(def.id), "notice omits {}: {stdout}", def.id);
        assert!(
            stdout.contains(declaration.key()),
            "notice omits {}: {stdout}",
            declaration.key()
        );
    }
    assert!(stdout.contains("revision-log.version-bump-required"));
    assert!(
        seen >= 5,
        "expected at least five declaration-reading rules"
    );
    std::fs::remove_dir_all(&dir).ok();
}
