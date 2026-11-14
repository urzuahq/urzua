//! Loading `.urzua/config.yaml`: a repeated key is refused, a byte order mark is
//! skipped, and the messages about the file are written in YAML, not TOML (the
//! first-run review, findings 6.3 and 6.6).
//!
//! Verifies: SPEC-3

mod common;

use std::path::Path;

const CONFIG: &str = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\nrules:\n  header.required-fields: error\n";

fn repo(label: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-config-load-{label}"));
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::create_dir_all(dir.path().join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.path().join(".urzua")).unwrap();
    std::fs::write(dir.path().join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(
        dir.path().join("docs/adr/ADR-1-first.md"),
        "---\nStatus: Accepted\n---\n# ADR-1: First\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code(), text)
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"))
}

#[test]
fn a_rule_written_twice_is_refused_by_check_and_doctor_names_it() {
    let config = CONFIG.to_string() + "  header.required-fields: off\n";
    let dir = repo("dup-rule", &config);
    let (code, text) = run(dir.path(), &["check"]);
    assert_eq!(code, Some(2), "{text}");
    assert!(
        text.contains("header.required-fields") && text.contains("twice"),
        "{text}"
    );
    let (code, text) = run(dir.path(), &["doctor"]);
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("header.required-fields"), "{text}");
}

#[test]
fn a_record_type_written_twice_is_refused() {
    let config = CONFIG.to_string().replace(
        "rules:",
        "  adr:\n    dir: docs/other\n    prefix: ADR\nrules:",
    );
    let dir = repo("dup-type", &config);
    let (code, text) = run(dir.path(), &["check"]);
    assert_eq!(code, Some(2), "{text}");
    assert!(text.contains("'adr'") && text.contains("twice"), "{text}");
}

/// Every record reader skips a BOM; the config was the one file that did not.
#[test]
fn a_config_with_a_byte_order_mark_loads_and_checks_as_without_one() {
    let plain = repo("bom-plain", CONFIG);
    let marked = repo("bom-marked", &format!("\u{feff}{CONFIG}"));
    let (plain_code, plain_text) = run(plain.path(), &["check"]);
    let (marked_code, marked_text) = run(marked.path(), &["check"]);
    assert_eq!(plain_code, Some(0), "{plain_text}");
    assert_eq!(marked_code, Some(0), "{marked_text}");
    assert_eq!(
        json(&plain_text)["rules_executed"],
        json(&marked_text)["rules_executed"]
    );
    let (code, text) = run(marked.path(), &["doctor"]);
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("config parses"), "{text}");
}

#[test]
fn a_message_about_the_config_is_written_in_yaml_not_toml() {
    let config = CONFIG.to_string() + "  type.no-declared-spec: warn\n";
    let dir = repo("yaml-words", &config);
    let (_, text) = run(dir.path(), &["check"]);
    let report = json(&text);
    let message = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == "type.no-declared-spec")
        .unwrap_or_else(|| panic!("no finding: {report}"))["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        message.contains("`spec: SPEC-N`") && !message.contains("spec = "),
        "{message}"
    );
    let (_, text) = run(dir.path(), &["new", "widget", "Title"]);
    assert!(
        text.contains("`record_types:`") && !text.contains("[record_types]"),
        "{text}"
    );
}

/// `new` named a missing `extra_dirs` entry on stderr only, which an agent
/// parsing stdout never saw; the notice is in the JSON now (main's `numbering`).
#[test]
fn new_reports_a_missing_extra_dir_in_the_json() {
    let config = CONFIG.replace(
        "    prefix: ADR\n",
        "    prefix: ADR\n    extra_dirs: [docs/old-adr]\n",
    );
    let dir = repo("extra-dir", &config);
    let out = common::urzua()
        .args(["new", "adr", "Second", "--by", "tester"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let report = json(&String::from_utf8_lossy(&out.stdout));
    let notice = report["notices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| {
            n["subject"] == "numbering" && n["message"].as_str().unwrap().contains("extra_dirs")
        })
        .unwrap_or_else(|| panic!("no notice: {report}"));
    assert!(
        notice["message"].as_str().unwrap().contains("docs/old-adr"),
        "{notice}"
    );
}
