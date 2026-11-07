//! MILE-168: `waiver.matches-nothing` over a planted corpus, whole-corpus and
//! partial runs.

mod common;

use std::path::Path;
use std::process::Command;

const CONFIG: &str = "schema_version: 2
rules:
  header.required-fields: error
  waiver.matches-nothing: warn
record_types:
  adr:
    dir: docs/adr
    required_fields: [Status, Author]
    header_shape: yaml-frontmatter
  waiver:
    dir: docs/waiver
    required_fields: [Rule, Scope]
    known_fields: [Expires]
    header_shape: yaml-frontmatter
";

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

fn waiver(rule: &str, scope: &str, expires: Option<&str>) -> String {
    let exp = expires
        .map(|e| format!("Expires: {e}\n"))
        .unwrap_or_default();
    format!("---\nRule: {rule}\nScope: {scope}\n{exp}---\n# Waiver\n")
}

/// ADR-1 lacks Author (a live `header.required-fields` finding); ADR-2 is
/// clean. Committed and tagged `base`.
fn corpus(name: &str, waivers: &[(&str, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-waiver-stale-{name}"));
    for d in ["docs/adr", "docs/waiver", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Accepted\n---\n# 1 - A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Accepted\nAuthor: Ana\n---\n# 2 - B\n",
    )
    .unwrap();
    for (file, body) in waivers {
        std::fs::write(dir.join("docs/waiver").join(file), body).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn check(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn stale(report: &serde_json::Value) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "waiver.matches-nothing")
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect()
}

fn planted() -> Vec<(&'static str, String)> {
    vec![
        (
            "WAIVER-1-live.md",
            waiver("header.required-fields", "docs/adr/ADR-1-a.md", None),
        ),
        (
            "WAIVER-2-stale.md",
            waiver("header.required-fields", "docs/adr/ADR-2-b.md", None),
        ),
        (
            "WAIVER-3-expired.md",
            waiver(
                "header.required-fields",
                "docs/adr/ADR-1-a.md",
                Some("2020-01-01"),
            ),
        ),
    ]
}

#[test]
fn a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one() {
    let dir = corpus("whole", &planted());
    let report = check(&dir, &[]);
    assert_eq!(
        stale(&report),
        vec![
            "docs/waiver/WAIVER-2-stale.md".to_string(),
            "docs/waiver/WAIVER-3-expired.md".to_string()
        ],
        "{report}"
    );
    let msg = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["file"] == "docs/waiver/WAIVER-2-stale.md")
        .unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        msg.contains("header.required-fields") && msg.contains("docs/adr/ADR-2-b.md"),
        "{msg}"
    );
}

#[test]
fn a_path_scoped_run_does_not_call_a_waiver_stale() {
    let dir = corpus("scoped", &planted());
    let report = check(&dir, &["docs/adr"]);
    assert!(stale(&report).is_empty(), "{report}");
}

#[test]
fn a_base_run_does_not_call_an_unchanged_waiver_stale() {
    let dir = corpus("base", &planted());
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Accepted\nAuthor: Ana\n---\n# 2 - B\n\nEdited.\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "edit"]);
    let report = check(&dir, &["--base", "base"]);
    assert!(stale(&report).is_empty(), "{report}");
}

#[test]
fn a_base_run_reports_a_changed_expired_waiver() {
    let dir = corpus("base-changed", &planted());
    std::fs::write(
        dir.join("docs/waiver/WAIVER-3-expired.md"),
        waiver(
            "header.required-fields",
            "docs/adr/ADR-1-a.md",
            Some("2020-01-02"),
        ),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "edit"]);
    let report = check(&dir, &["--base", "base"]);
    assert_eq!(
        stale(&report),
        vec!["docs/waiver/WAIVER-3-expired.md".to_string()],
        "{report}"
    );
}

#[test]
fn a_waiver_for_a_rule_that_is_not_enabled_is_not_judged() {
    let dir = corpus(
        "off",
        &[(
            "WAIVER-1-x.md",
            waiver("field.quality", "docs/adr/ADR-2-b.md", None),
        )],
    );
    let report = check(&dir, &[]);
    assert!(stale(&report).is_empty(), "{report}");
}
