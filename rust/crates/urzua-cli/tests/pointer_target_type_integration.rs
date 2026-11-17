//! MILE-259: a pointer field, or a relation pair's forward field, declares the
//! record types its targets may have, and `pointer.target-type` reports a
//! target of another type.

mod common;

use std::process::Command;

const TYPES: &str = "schema_version: 2
record_types:
  a:
    dir: docs/a
    required_fields: [Status]
    known_fields: [Parent, Up, Down]
    pointer_fields: @POINTERS
    narrative_fields: []
  b:
    dir: docs/b
    required_fields: [Status]
    known_fields: [Up, Down]
    pointer_fields: []
    narrative_fields: []
";

fn config(pointers: &str, tail: &str) -> String {
    format!("{}{tail}", TYPES.replace("@POINTERS", pointers))
}

fn repo(name: &str, config: &str, files: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-target-type-{name}"));
    for d in ["docs/a", "docs/b", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (path, text) in files {
        std::fs::write(dir.join(path), text).unwrap();
    }
    for args in [&["init", "-q"][..], &["add", "-A"][..]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    dir
}

fn urzua(dir: &common::TempDir, args: &[&str]) -> (serde_json::Value, std::process::Output) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let json =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"));
    (json, out)
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

const A1_PARENT: &str = "---\nStatus: x\nParent: @P\n---\n# 1 - One\n";
const A2: &str = "---\nStatus: x\n---\n# 2 - Two\n";
const B3: &str = "---\nStatus: x\n---\n# 3 - Three\n";

fn parent(value: &str) -> String {
    A1_PARENT.replace("@P", value)
}

#[test]
fn a_pointer_to_a_record_of_an_undeclared_type_is_a_finding() {
    let cfg = config(
        "[{name: Parent, targets: [b]}]",
        "rules:\n  pointer.resolution: error\n",
    );
    let a1 = parent("A-2");
    let dir = repo(
        "pointer",
        &cfg,
        &[
            ("docs/a/A-1-one.md", &a1),
            ("docs/a/A-2-two.md", A2),
            ("docs/b/B-3-three.md", B3),
        ],
    );
    let (report, out) = urzua(&dir, &["check", "docs/"]);
    let got = findings(&report, "pointer.target-type");
    assert_eq!(got.len(), 1, "{report}");
    let message = got[0]["message"].as_str().unwrap();
    assert_eq!(
        message,
        "Parent: A-2 is of type 'a', but Parent on type 'a' may only name records of type b"
    );
    assert_eq!(got[0]["severity"], "error", "{report}");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn a_suppression_record_suppresses_a_wrong_type_finding() {
    let tail = "  waiver:
    dir: docs/waiver
    role: suppression
    effective_requires: [Reason]
    required_fields: [Rule, Scope, Reason]
    known_fields: [Expires]
    pointer_fields: []
    narrative_fields: []
rules:
  pointer.target-type: error
";
    let cfg = config("[{name: Parent, targets: [b]}]", tail);
    let a1 = parent("A-2");
    let waiver = "---\nRule: pointer.target-type\nScope: docs/a/A-1-one.md\nReason: the parent is deliberately a sibling\nExpires: 2999-01-01\n---\n# 1 - Sibling parent\n";
    let dir = repo(
        "waived",
        &cfg,
        &[("docs/a/A-1-one.md", &a1), ("docs/a/A-2-two.md", A2)],
    );
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(dir.join("docs/waiver/WAIVER-1-sibling.md"), waiver).unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    let (report, out) = urzua(&dir, &["check", "docs/"]);
    let got = findings(&report, "pointer.target-type");
    assert_eq!(got.len(), 1, "{report}");
    assert_eq!(
        got[0]["waived"], "docs/waiver/WAIVER-1-sibling.md",
        "{report}"
    );
    assert_eq!(out.status.code(), Some(0), "{report}");
}

#[test]
fn an_allowed_target_and_an_unconstrained_config_are_silent() {
    let a1 = parent("B-3");
    let files: &[(&str, &str)] = &[("docs/a/A-1-one.md", &a1), ("docs/b/B-3-three.md", B3)];
    let dir = repo(
        "allowed",
        &config("[{name: Parent, targets: [b]}]", ""),
        files,
    );
    let (report, _) = urzua(&dir, &["check", "docs/"]);
    assert!(
        findings(&report, "pointer.target-type").is_empty(),
        "{report}"
    );

    let a1 = parent("A-2");
    let files: &[(&str, &str)] = &[("docs/a/A-1-one.md", &a1), ("docs/a/A-2-two.md", A2)];
    let dir = repo("unconstrained", &config("[Parent]", ""), files);
    let (report, _) = urzua(&dir, &["check", "docs/"]);
    assert!(
        findings(&report, "pointer.target-type").is_empty(),
        "{report}"
    );
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "pointer.target-type")
        .expect("listed");
    assert_eq!(rule["status"], "not-enabled", "{rule}");
}

#[test]
fn each_element_of_a_list_is_judged_and_a_dangling_one_is_resolutions_alone() {
    let cfg = config(
        "[{name: Parent, targets: [b]}]",
        "rules:\n  pointer.resolution: error\n",
    );
    let a1 = parent("B-3, A-2, B-9");
    let dir = repo(
        "list",
        &cfg,
        &[
            ("docs/a/A-1-one.md", &a1),
            ("docs/a/A-2-two.md", A2),
            ("docs/b/B-3-three.md", B3),
        ],
    );
    let (report, _) = urzua(&dir, &["check", "docs/"]);
    let typed = findings(&report, "pointer.target-type");
    assert_eq!(typed.len(), 1, "{report}");
    assert!(typed[0]["message"].as_str().unwrap().contains("A-2"));
    let dangling = findings(&report, "pointer.resolution");
    assert_eq!(dangling.len(), 1, "{report}");
    assert!(dangling[0]["message"].as_str().unwrap().contains("B-9"));
}

#[test]
fn a_pair_with_target_types_reports_a_link_to_another_type_once() {
    let pair = "relation_pairs:\n  - {name: link, forward: Up, inverse: Down, reciprocity: none, cross_type: false, types: [a], target_types: [a]}\nrules:\n  relation.cross-type-disallowed: error\n";
    let a1 = "---\nStatus: x\nUp: B-3, A-2\n---\n# 1 - One\n";
    let dir = repo(
        "pair",
        &config("[]", pair),
        &[
            ("docs/a/A-1-one.md", a1),
            ("docs/a/A-2-two.md", A2),
            ("docs/b/B-3-three.md", B3),
        ],
    );
    let (report, _) = urzua(&dir, &["check", "docs/"]);
    let typed = findings(&report, "pointer.target-type");
    assert_eq!(typed.len(), 1, "{report}");
    assert!(typed[0]["message"].as_str().unwrap().contains("B-3"));
    assert!(
        findings(&report, "relation.cross-type-disallowed").is_empty(),
        "{report}"
    );
}

#[test]
fn a_misdeclared_target_list_refuses_to_load() {
    for (pointers, tail, needle) in [
        ("[{name: Parent, targets: [zz]}]", "", "not a configured record type"),
        ("[{name: Parent, targets: []}]", "", "at least one record type"),
        (
            "[]",
            "relation_pairs:\n  - {name: link, forward: Up, inverse: Down, reciprocity: none, cross_type: false, types: [a], target_types: [b]}\n",
            "cross_type: false",
        ),
    ] {
        let dir = repo("bad", &config(pointers, tail), &[("docs/a/A-2-two.md", A2)]);
        let out = common::urzua()
            .args(["check", "docs/"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out.status.success(), "{text}");
        assert!(text.contains(needle), "{needle}: {text}");
    }
}

#[test]
fn graph_lists_constrained_pointer_and_relation_pair_edges() {
    let pair = "relation_pairs:\n  - {name: link, forward: Up, inverse: Down, reciprocity: none, target_types: [b]}\n";
    let a1 = "---\nStatus: x\nParent: B-3\nUp: B-3\n---\n# 1 - One\n";
    let dir = repo(
        "graph",
        &config("[{name: Parent, targets: [b]}]", pair),
        &[("docs/a/A-1-one.md", a1), ("docs/b/B-3-three.md", B3)],
    );
    let (report, _) = urzua(&dir, &["graph"]);
    let mut relations: Vec<&str> = report["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["relation"].as_str().unwrap())
        .collect();
    relations.sort();
    assert_eq!(relations, vec!["Parent", "Up"], "{report}");
}
