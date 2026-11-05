//! MILE-189: `embodiment.locator-on-back-pointer-path` nudges a `Realized-by`
//! locator on a file the back-pointer scan already reads toward an
//! `Implements:` / `Verifies:` comment. Real git-repo fixtures, real binary.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const CONFIG: &str = "schema_version: 2\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrules:\n  embodiment.locator-on-back-pointer-path:\n    level: warn\n    back_pointer_paths: [src]\n    test_paths: [src/tests]\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n";

fn fixture(name: &str, config: &str, realized_by: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-mechanism-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for d in ["docs/adr", "src/tests", "docs/guide", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for f in [
        "src/a.rs",
        "src/other.rs",
        "src/tests/t.rs",
        "docs/guide/g.md",
    ] {
        std::fs::write(dir.join(f), "fn a() {}\n").unwrap();
    }
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        format!("---\nEmbodiment: Verified\nRealized-by: {realized_by}\n---\n# ADR-1: T\n"),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn nudges(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.locator-on-back-pointer-path")
        .collect()
}

#[test]
fn a_code_and_a_test_locator_under_the_scanned_paths_are_each_nudged() {
    let dir = fixture(
        "both",
        CONFIG,
        "code:src/a.rs, test:src/tests/t.rs, code:docs/guide/g.md",
    );
    let (_, v) = check(&dir);
    let found = nudges(&v);
    assert_eq!(found.len(), 2, "{v}");
    assert!(found.iter().all(|f| f["severity"] == "warning"));
    let code = found
        .iter()
        .find(|f| f["message"].as_str().unwrap().contains("src/a.rs"))
        .unwrap();
    assert!(
        code["message"]
            .as_str()
            .unwrap()
            .contains("Implements: ADR-1"),
        "{code}"
    );
    let test = found
        .iter()
        .find(|f| f["message"].as_str().unwrap().contains("src/tests/t.rs"))
        .unwrap();
    assert!(
        test["message"]
            .as_str()
            .unwrap()
            .contains("Verifies: ADR-1"),
        "{test}"
    );
    // Not under a scanned path: no comment would be read there.
    assert!(!found
        .iter()
        .any(|f| f["message"].as_str().unwrap().contains("g.md")));
}

#[test]
fn the_action_is_an_edit_field_that_drops_the_locator_and_keeps_the_rest() {
    let dir = fixture("action", CONFIG, "code:src/a.rs, code:docs/guide/g.md");
    let (_, v) = check(&dir);
    let f = nudges(&v)[0].clone();
    let action = &f["suggested_action"];
    assert_eq!(action["kind"], "edit-field", "{f}");
    assert_eq!(action["op"], "set", "{f}");
    assert_eq!(action["target"]["field"], "Realized-by", "{f}");
    assert_eq!(action["value"], "code:docs/guide/g.md", "{f}");
}

#[test]
fn a_sole_locator_is_removed_with_the_field() {
    let dir = fixture("sole", CONFIG, "code:src/a.rs");
    let (_, v) = check(&dir);
    let f = nudges(&v)[0].clone();
    assert_eq!(f["suggested_action"]["op"], "remove", "{f}");
}

#[test]
fn what_a_comment_cannot_say_is_not_nudged() {
    // A pinned or scoped locator, a spec: locator, and a test: locator on a
    // file outside test_paths have no comment of the same tier.
    let dir = fixture(
        "exempt",
        CONFIG,
        "code:src/a.rs#L1-L1, spec:src/other.rs, test:src/other.rs",
    );
    let (_, v) = check(&dir);
    assert!(nudges(&v).is_empty(), "{v}");
    let exec = v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == "embodiment.locator-on-back-pointer-path")
        .unwrap();
    assert_eq!(exec["status"], "ran");
    assert_eq!(exec["population"]["examined"], 1);
}

#[test]
fn without_back_pointer_paths_the_rule_does_not_load() {
    let config = CONFIG.replace(
        "    back_pointer_paths: [src]\n    test_paths: [src/tests]\n",
        "",
    );
    let dir = fixture("nopaths", &config, "code:src/a.rs");
    let (code, v) = check(&dir);
    assert_eq!(code, 2, "{v}");
    assert!(
        v["error"].as_str().unwrap().contains("back_pointer_paths"),
        "{v}"
    );
}
