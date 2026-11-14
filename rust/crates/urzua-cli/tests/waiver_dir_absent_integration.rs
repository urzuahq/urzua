//! A suppression type whose directory does not exist yet is a type with no
//! waivers, not a misdeclared directory (the adopter switchover, item 3): git
//! does not track an empty directory, so a fresh clone never has one before the
//! first waiver is filed.

mod common;

use common::waive::git;

fn repo(name: &str, waiver_role: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-waiverdir-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  type.dir-matches-nothing: warn\nrecord_types:\n  adr:\n    \
             dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n  \
             waiver:\n    dir: docs/waiver\n    prefix: WAIVER\n{waiver_role}    header_shape: yaml-frontmatter\n    \
             required_fields: [Rule, Scope, Reason]\n    known_fields: [Expires]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Proposed\n---\n# 1 - X\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn dir_findings(dir: &std::path::Path) -> Vec<String> {
    let (_, stdout, stderr) = common::waive::run(dir, &["check"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout} {stderr}"));
    json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "type.dir-matches-nothing")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_suppression_type_with_no_directory_yet_is_not_a_misdeclared_directory() {
    let dir = repo(
        "suppression",
        "    role: suppression\n    effective_requires: [Reason, Expires]\n",
    );
    assert_eq!(dir_findings(&dir), Vec::<String>::new());
}

#[test]
fn any_other_type_with_no_directory_is_still_reported() {
    let dir = repo("plain", "");
    let found = dir_findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("'waiver'") && found[0].contains("docs/waiver"));
}
