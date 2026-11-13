//! Path confinement: a path a user or the config names is read and written
//! only inside the repository, symlinks included. Real git fixtures, real
//! binary.

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

fn run(dir: &Path, args: &[&str]) -> (std::process::Output, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    (out, parsed)
}

const RECORD: &str = "---\nStatus: Accepted\n---\n# 1 \u{2014} One\nline two\n";

/// A repo with one ADR, an `index.freshness` entry at `index_path`, a tracked
/// symlink `out` to a directory outside it, and a tracked symlink `leak.txt`
/// to a file outside it. Returns (repo, outside dir).
fn fixture(name: &str, index_path: &str) -> (common::TempDir, common::TempDir) {
    let dir = common::TempDir::new(&format!("urzua-confine-{name}"));
    let outside = common::TempDir::new(&format!("urzua-confine-out-{name}"));
    std::fs::write(outside.join("secret.txt"), "top secret\n").unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  index.freshness:\n    level: error\n    indexes: [{{path: \"{index_path}\", type: adr, format: \"- [{{id}}]({{file}}): {{title}}\", begin: \"<!-- begin -->\", end: \"<!-- end -->\"}}]\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-x.md"), RECORD).unwrap();
    std::os::unix::fs::symlink(&*outside, dir.join("out")).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.txt"), dir.join("leak.txt")).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    (dir, outside)
}

#[test]
fn index_apply_does_not_create_a_file_through_a_symlinked_directory() {
    let (dir, outside) = fixture("index", "out/idx.md");
    let (out, parsed) = run(&dir, &["index", "--apply"]);
    assert!(
        !outside.join("idx.md").exists(),
        "wrote outside the repository: {parsed}"
    );
    assert_ne!(out.status.code(), Some(0), "{parsed}");
    assert!(
        parsed.to_string().contains("outside the repository"),
        "{parsed}"
    );
}

#[test]
fn explain_refuses_paths_outside_the_repository() {
    let (dir, _outside) = fixture("explain", "docs/adr/README.md");
    let abs = format!("{}#L1", dir.join("..").join("urzua-nonexistent").display());
    for arg in [
        "../../etc/hostname#L1",
        "/etc/hostname#L1",
        "leak.txt#L1",
        "out/secret.txt#L1",
        abs.as_str(),
    ] {
        let (out, parsed) = run(&dir, &["explain", arg]);
        assert_eq!(out.status.code(), Some(2), "{arg}: {parsed}");
        let text = parsed.to_string();
        assert!(text.contains("outside the repository"), "{arg}: {text}");
        assert!(!text.contains("top secret"), "{arg}: {text}");
    }
    let (out, parsed) = run(&dir, &["explain", "docs/adr/ADR-1-x.md#L4"]);
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    assert_eq!(parsed["scope"]["text"], "# 1 \u{2014} One", "{parsed}");
}

#[test]
fn explain_refuses_an_absolute_path_to_a_real_outside_file() {
    let (dir, outside) = fixture("explain-abs", "docs/adr/README.md");
    let arg = format!("{}#L1", outside.join("secret.txt").display());
    let (out, parsed) = run(&dir, &["explain", &arg]);
    assert_eq!(out.status.code(), Some(2), "{parsed}");
    assert!(!parsed.to_string().contains("top secret"), "{parsed}");
}

#[test]
fn new_does_not_write_a_record_through_a_symlinked_record_directory() {
    let (dir, outside) = fixture("new", "docs/adr/README.md");
    // Point the record type's directory at the tracked symlink out of the repo.
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace("dir: \"docs/adr\"", "dir: \"out\""),
    )
    .unwrap();
    let (out, parsed) = run(&dir, &["new", "adr", "Escapes", "--by", "t"]);
    assert_ne!(out.status.code(), Some(0), "{parsed}");
    assert!(
        parsed.to_string().contains("outside the repository"),
        "{parsed}"
    );
    let written: Vec<_> = std::fs::read_dir(&*outside)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        written.len(),
        1,
        "wrote outside the repository: {written:?}"
    );
}
