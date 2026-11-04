//! `urzua export --format json` (SPEC-1): run the real binary against a small
//! git-repo fixture holding two record types in two header shapes.

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
    let dir = std::env::temp_dir().join(format!(
        "urzua-cli-export-test-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/notes")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n  note:\n    dir: \"docs/notes\"\n    prefix: \"NOTE\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    // yaml-frontmatter, with a Stable-Id.
    std::fs::write(
        dir.join("docs/adr/ADR-7-pick-a-thing.md"),
        "---\nStable-Id: 01ABCDEF\nStatus: Accepted\n---\n# 7 — Pick a thing\n\nBody text.\n",
    )
    .unwrap();
    // A second type, no Stable-Id.
    std::fs::write(
        dir.join("docs/notes/NOTE-3-a-note.md"),
        "---\nStatus: Open\n---\n# 3 — A note\n\nNote body.\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

#[test]
fn export_json_emits_every_record_with_parsed_header_and_raw_text() {
    let dir = fixture_repo("json");
    let output = run_urzua(&dir, &["export", "--format", "json"]);
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON on stdout");

    assert_eq!(v["status"], "ok");
    assert_eq!(v["record_count"], 2);
    assert_eq!(
        v["types"],
        serde_json::json!([
            {"type": "adr", "dir": "docs/adr", "records": 1},
            {"type": "note", "dir": "docs/notes", "records": 1},
        ])
    );

    let records = v["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);

    let adr = &records[0];
    assert_eq!(adr["type"], "adr");
    assert_eq!(adr["path"], "docs/adr/ADR-7-pick-a-thing.md");
    assert_eq!(adr["id"], "ADR-7");
    assert_eq!(adr["stable_id"], "01ABCDEF");
    assert_eq!(adr["header_readable"], true);
    let names: Vec<&str> = adr["header"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Stable-Id", "Status"]);
    assert_eq!(adr["header"][1]["value"], "Accepted");
    assert!(adr["raw_text"].as_str().unwrap().contains("Body text."));

    let note = &records[1];
    assert_eq!(note["type"], "note");
    assert_eq!(note["id"], "NOTE-3");
    assert!(note["stable_id"].is_null(), "no Stable-Id means null");
    assert_eq!(note["header"][0]["name"], "Status");
    assert_eq!(note["header"][0]["value"], "Open");
    assert!(note["raw_text"].as_str().unwrap().contains("Note body."));
}

#[test]
fn export_agdr_stays_unimplemented_and_json_must_be_named() {
    let dir = fixture_repo("agdr");
    for args in [&["export", "--format", "agdr"][..], &["export"][..]] {
        let output = run_urzua(&dir, args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(v["status"], "not-run");
        assert!(v["error"].as_str().unwrap().contains("not implemented"));
    }
}

#[test]
fn export_json_marks_an_unreadable_header_instead_of_an_empty_one() {
    let dir = fixture_repo("unreadable");
    std::fs::write(
        dir.join("docs/adr/ADR-8-broken.md"),
        "---\nStatus: [unclosed\n---\n# 8 — Broken\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "broken"]);
    let output = run_urzua(&dir, &["export", "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let broken = v["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "ADR-8")
        .expect("broken record still exported");
    assert_eq!(broken["header_readable"], false);
}

#[test]
fn export_json_treats_a_headerless_type_as_readable() {
    let dir = fixture_repo("headerless");
    std::fs::create_dir_all(dir.join("docs/log")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrecord_types:\n  log:\n    dir: \"docs/log\"\n    required_fields: []\n    header_shape: \"none\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/log/LOG-1-entry.md"),
        "# 1 — Entry\n\nText.\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "headerless"]);
    let output = run_urzua(&dir, &["export", "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let entry = v["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "LOG-1")
        .expect("headerless record exported");
    assert_eq!(entry["header_readable"], true, "{v}");
}

#[cfg(unix)]
#[test]
fn export_json_refuses_a_record_symlinked_outside_the_repository() {
    let dir = fixture_repo("symlink-escape");
    let outside = std::env::temp_dir().join(format!("urzua-export-secret-{}", std::process::id()));
    std::fs::write(
        &outside,
        "---\nStatus: Accepted\n---\n# 9 — secret contents\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("docs/adr/ADR-9-link.md")).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "link"]);
    let output = run_urzua(&dir, &["export", "--format", "json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_ne!(output.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("secret contents"), "{stdout}");
    assert!(
        stdout.contains("resolves outside the repository"),
        "{stdout}"
    );
    std::fs::remove_file(&outside).ok();
}
