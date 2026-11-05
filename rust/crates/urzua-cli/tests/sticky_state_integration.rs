//! Sticky `Embodiment` states (`MILE-148`) against the real binary: a state
//! the repository declares as human-set is not reported as drift by `check`
//! and not rewritten by `fix`, while a neighbour that genuinely disagrees is.

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

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

/// ADR-1 states `Deprecated`, ADR-2 `Inactive`, ADR-3 `Verified`; all three
/// carry only a `spec:` locator, so ADR-3 genuinely disagrees.
fn repo(name: &str, sticky: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-sticky-{name}"));
    for d in ["docs/adr", "docs/spec", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  embodiment.consistency:\n    level: error\n{sticky}\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\", \"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    for (n, state) in [(1, "Deprecated"), (2, "Inactive"), (3, "Verified")] {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!(
                "---\nStatus: Accepted\nEmbodiment: {state}\nRealized-by: spec:docs/spec/s.md\n---\n# {n} — X\n"
            ),
        )
        .unwrap();
    }
    std::fs::write(dir.join("docs/spec/s.md"), "spec\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn parsed(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("json")
}

#[test]
fn declared_sticky_states_are_not_drift_and_a_neighbour_still_is() {
    let dir = repo("on", "    sticky_states: [Deprecated, Inactive]\n");
    let report = parsed(&run_urzua(&dir, &["check"]));
    let found: Vec<&serde_json::Value> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .collect();
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["file"].as_str().unwrap().contains("ADR-3"));
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "embodiment.consistency")
        .unwrap();
    assert_eq!(rule["population"]["examined"], 3, "{rule}");
}

#[test]
fn without_a_declaration_every_stated_value_is_held_to_the_computed_one() {
    let dir = repo("off", "");
    let report = parsed(&run_urzua(&dir, &["check"]));
    let count = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .count();
    assert_eq!(count, 3, "{report}");
}

#[test]
fn fix_leaves_a_sticky_state_alone() {
    let dir = repo("fix", "    sticky_states: [Deprecated, Inactive]\n");
    let report = parsed(&run_urzua(&dir, &["fix"]));
    let repairs = report["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{report}");
    assert!(repairs[0]["record"].as_str().unwrap().contains("ADR-3"));
}

#[test]
fn a_sticky_state_that_evidence_computes_is_refused_at_load() {
    let dir = repo("bad", "    sticky_states: [Deprecated, Verified]\n");
    let out = run_urzua(&dir, &["check"]);
    assert!(!out.status.success());
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("sticky_states"), "{text}");
}
