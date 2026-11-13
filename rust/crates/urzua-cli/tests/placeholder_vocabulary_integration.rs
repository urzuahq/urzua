//! MILE-153: a type's declared `pending` vocabulary makes `statuses.require`
//! treat those whole values as missing. Runs this repository's own
//! `.urzua/config.yaml` against planted records, so the shipped declaration is
//! what is exercised, not a fixture's.

mod common;

use std::path::Path;
use std::process::Command;

fn own_config() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::read_to_string(path).expect("repository config readable")
}

fn run_with_own_config(name: &str, adr: &str) -> serde_json::Value {
    let dir = common::TempDir::new(&format!("urzua-placeholder-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    std::fs::write(dir.join(".urzua/config.yaml"), own_config()).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-first.md"), adr).unwrap();
    assert!(Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .status()
        .unwrap()
        .success());
    let out = common::urzua()
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn adr(deciders: &str) -> String {
    format!(
        "---\nStatus: Accepted\nDate: 2026-11-04\nAccepted-on: 2026-11-04\nAuthor: Ana\nDeciders: {deciders}\n---\n# 1 - First\n"
    )
}

fn missing(v: &serde_json::Value) -> usize {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "status.required-field-missing")
        .count()
}

#[test]
fn an_accepted_adr_whose_deciders_is_a_declared_placeholder_is_reported() {
    for v in ["Unknown", "none recorded"] {
        let quoted = format!("'{v}'");
        let report = run_with_own_config("placeholder", &adr(&quoted));
        assert_eq!(missing(&report), 1, "{v}: {report}");
    }
}

#[test]
fn a_real_value_that_merely_contains_the_word_is_not_reported() {
    let report = run_with_own_config("real", &adr("Unknown Soldier"));
    assert_eq!(missing(&report), 0, "{report}");
}

/// MILE-162: the declared pattern entry reads a placeholder family with a
/// variable tail as pending, beside the whole values; a value matching no
/// entry and no pattern is a real value.
#[test]
fn a_declared_pending_pattern_reads_a_placeholder_family_observed_failing() {
    for v in ["TBD", "tbd -- awaiting the board", "To be decided by Q3"] {
        let quoted = format!("'{v}'");
        let report = run_with_own_config("pattern", &adr(&quoted));
        assert_eq!(missing(&report), 1, "{v}: {report}");
    }
    for v in ["Tbdesign Group", "Ana"] {
        let report = run_with_own_config("pattern-real", &adr(v));
        assert_eq!(missing(&report), 0, "{v}: {report}");
    }
}
