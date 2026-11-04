//! MILE-98 slice 2: a record type's declared `identity` and `fields` drive
//! record loading. Runs the real binary against a Nygard-style corpus
//! (bare zero-padded filenames, bare `Date:` lines) with a hand-written
//! config and no code specific to that corpus.

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

fn fixture_repo(name: &str, config: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "urzua-cli-docmodel-test-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("doc/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (path, content) in files {
        std::fs::write(dir.join(path), content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

const NYGARD: &str = "schema_version: 2\nrules: {header.required-fields: error, filename.title-consistency: error}\nrecord_types:\n  adr:\n    dir: \"doc/adr\"\n    prefix: ADR\n    required_fields: [\"Date\"]\n    identity: {from: filename, pattern: '^(?P<number>\\d+)-(?P<slug>.+)$'}\n    fields: {from: prefix-lines}\n";

fn export(dir: &Path) -> serde_json::Value {
    let out = run(dir, &["export", "--format", "json"]);
    assert!(
        out.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("JSON on stdout")
}

#[test]
fn a_nygard_corpus_gets_ids_and_fields_from_its_declaration() {
    let dir = fixture_repo(
        "nygard",
        NYGARD,
        &[
            (
                "doc/adr/0001-record-decisions.md",
                "# 1. Record decisions\n\nDate: 2016-02-12\n\n## Status\n\nAccepted\n",
            ),
            (
                "doc/adr/0002-use-rust.md",
                "# 2. Use Rust\n\nDate: 2016-02-13\n\n## Status\n\nAccepted\n",
            ),
        ],
    );
    let v = export(&dir);
    let records = v["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["id"], "ADR-0001");
    assert_eq!(records[1]["id"], "ADR-0002");
    assert_eq!(records[0]["header_readable"], true);
    assert_eq!(records[0]["header"][0]["name"], "Date");
    assert_eq!(records[0]["header"][0]["value"], "2016-02-12");

    let out = run(&dir, &["check", "doc/"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && !stdout.contains("no header-shaped region"),
        "{stdout}"
    );
}

#[test]
fn the_same_corpus_without_a_declaration_has_no_identity_and_no_header() {
    let config = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"doc/adr\"\n    prefix: ADR\n    required_fields: [\"Date\"]\n";
    let dir = fixture_repo(
        "undeclared",
        config,
        &[(
            "doc/adr/0001-record-decisions.md",
            "# 1. Record decisions\n\nDate: 2016-02-12\n\n## Status\n",
        )],
    );
    let v = export(&dir);
    assert_eq!(v["records"][0]["id"], serde_json::Value::Null);
    assert_eq!(v["records"][0]["header_readable"], false);
}

#[test]
fn a_title_form_mismatch_is_reported_for_a_declaring_type() {
    let dir = fixture_repo(
        "title",
        NYGARD,
        &[(
            "doc/adr/0001-record-decisions.md",
            "# ADR-1 Record decisions\n\nDate: 2016-02-12\n",
        )],
    );
    let out = run(&dir, &["check", "doc/"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("bare number"), "{stdout}");
}

#[test]
fn an_identity_pattern_that_does_not_compile_stops_the_run_naming_the_type() {
    let config = "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"doc/adr\"\n    identity: {from: filename, pattern: '('}\n";
    let dir = fixture_repo("badpattern", config, &[("doc/adr/1-x.md", "# 1\n")]);
    let out = run(&dir, &["check", "doc/"]);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success());
    assert!(all.contains("adr") && all.contains("identity"), "{all}");
}
