//! MILE-189: `embodiment.locator-on-back-pointer-path` nudges a `Realized-by`
//! locator on a file the back-pointer scan already reads toward an
//! `Implements:` / `Verifies:` comment. Real git-repo fixtures, real binary.

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

const CONFIG: &str = "schema_version: 2\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrules:\n  embodiment.locator-on-back-pointer-path:\n    level: warn\n    back_pointer_paths: [src]\n    test_paths: [src/tests]\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n";

fn fixture(name: &str, config: &str, realized_by: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-mechanism-{name}"));
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
    let out = common::urzua()
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
fn no_edit_is_offered_because_the_comment_must_exist_before_the_locator_goes() {
    // Dropping the locator first would delete the evidence and leave Embodiment
    // disagreeing with what is computed; the other half of the repair is a
    // comment in a source file, which no action kind addresses (ADR-73).
    for (name, value) in [
        ("action", "code:src/a.rs, code:docs/guide/g.md"),
        ("sole", "code:src/a.rs"),
    ] {
        let dir = fixture(name, CONFIG, value);
        let (_, v) = check(&dir);
        let found = nudges(&v);
        assert_eq!(found.len(), 1, "{v}");
        assert!(
            found[0]
                .as_object()
                .unwrap()
                .get("suggested_action")
                .is_none(),
            "{name}: {}",
            found[0]
        );
        let msg = found[0]["message"].as_str().unwrap();
        assert!(msg.contains("add the comment first"), "{name}: {msg}");
    }
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

#[test]
fn a_file_that_already_carries_the_comment_is_told_to_drop_the_locator() {
    let dir = fixture("present", CONFIG, "code:src/a.rs, test:src/tests/t.rs");
    std::fs::write(dir.join("src/a.rs"), "// Implements: ADR-1\nfn a() {}\n").unwrap();
    std::fs::write(
        dir.join("src/tests/t.rs"),
        "// Verifies: ADR-1\nfn t() {}\n",
    )
    .unwrap();
    let (_, v) = check(&dir);
    let found = nudges(&v);
    assert_eq!(found.len(), 2, "{v}");
    for f in &found {
        let msg = f["message"].as_str().unwrap();
        assert!(msg.contains("already carries it"), "{msg}");
        assert!(!msg.contains("add the comment first"), "{msg}");
    }
}

#[test]
fn a_comment_for_another_record_does_not_count_as_the_comment() {
    let dir = fixture("other", CONFIG, "code:src/a.rs");
    std::fs::write(dir.join("src/a.rs"), "// Implements: ADR-2\nfn a() {}\n").unwrap();
    let (_, v) = check(&dir);
    let found = nudges(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("add the comment first"));
}

#[test]
fn a_type_that_counts_no_back_pointer_as_evidence_is_not_nudged() {
    // `back_pointer_evidence: false` (ADR-84): the locator is the only counted
    // evidence, so advice to drop it would leave a Fixed record unevidenced.
    let config = CONFIG.replace(
        "    known_fields: [\"Embodiment\", \"Realized-by\"]\n",
        "    known_fields: [\"Embodiment\", \"Realized-by\"]\n    statuses:\n      values: [Open, Fixed]\n      success: [Fixed]\n      delivery:\n        delivered_tiers: [Implemented, Verified]\n        unevidenced_tiers: [Not started]\n        not_done: [Open]\n        set_status: Fixed\n        back_pointer_evidence: false\n",
    );
    let dir = fixture("nocount", &config, "code:src/a.rs, test:src/tests/t.rs");
    let (code, v) = check(&dir);
    assert_ne!(code, 2, "{v}");
    assert!(nudges(&v).is_empty(), "{v}");
    let exec = v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == "embodiment.locator-on-back-pointer-path")
        .unwrap();
    assert_eq!(exec["status"], "ran");
    assert_eq!(exec["population"]["examined"], 1);

    // The default keeps the nudge: the same type with the option omitted.
    let counted = config.replace("        back_pointer_evidence: false\n", "");
    let dir = fixture("count", &counted, "code:src/a.rs");
    let (_, v) = check(&dir);
    assert_eq!(nudges(&v).len(), 1, "{v}");
}
