//! `status.delivery-agreement` and the `Status` repair in `urzua fix`
//! (`MILE-183`) against the real binary: a record whose delivery evidence says
//! it is built while its `Status` says it is not, and a success `Status` with no
//! evidence, are both reported; `fix` writes only the first.

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
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn parsed(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "json: {e}\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

const DELIVERY: &str = "      delivery:\n        delivered_tiers: [Implemented, Verified]\n        unevidenced_tiers: [\"Not started\"]\n        not_done: [Planned, InProgress, Blocked]\n        set_status: Done\n";

fn config(delivery: &str, extra_rules: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  status.delivery-agreement: error\n  embodiment.locator-exists: error\n{extra_rules}\
         record_types:\n  milestone:\n    dir: \"docs/milestones\"\n    prefix: \"MILE\"\n    \
         required_fields: [\"Status\"]\n    known_fields: [\"Realized-by\"]\n    \
         header_shape: \"yaml-frontmatter\"\n    statuses:\n      \
         values: [Planned, InProgress, Blocked, Done, WontDo]\n      success: [Done]\n      failure: [WontDo]\n{delivery}"
    )
}

fn milestone(status: &str, realized_by: Option<&str>) -> String {
    let locator = realized_by
        .map(|r| format!("Realized-by: {r}\n"))
        .unwrap_or_default();
    format!(
        "---\nStatus: {status}\n{locator}---\n# X\n\n## What\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n"
    )
}

fn repo(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-delivery-{name}"));
    for d in ["docs/milestones", "src", "tests", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (n, status, locator) in [
        (1, "Planned", Some("code:src/a.rs")),
        (2, "Planned", None),
        (3, "Done", Some("code:src/a.rs, test:tests/a.rs")),
        (4, "Done", None),
        (5, "Done", Some("spec:docs/specs/s.md")),
        (6, "InProgress", Some("test:tests/a.rs")),
        (7, "WontDo", None),
        (8, "Blocked", Some("spec:docs/specs/s.md")),
    ] {
        std::fs::write(
            dir.join(format!("docs/milestones/MILE-{n}-x.md")),
            milestone(status, locator),
        )
        .unwrap();
    }
    std::fs::create_dir_all(dir.join("docs/specs")).unwrap();
    std::fs::write(dir.join("docs/specs/s.md"), "spec\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("tests/a.rs"), "fn t() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

fn names(found: &[&serde_json::Value]) -> Vec<String> {
    let mut v: Vec<String> = found
        .iter()
        .map(|f| {
            let file = f["file"].as_str().unwrap();
            file.rsplit('/').next().unwrap()[..6]
                .trim_end_matches('-')
                .to_string()
        })
        .collect();
    v.sort();
    v
}

#[test]
fn both_directions_are_reported_and_the_rest_are_clean() {
    let dir = repo("both", &config(DELIVERY, ""));
    let report = parsed(&run_urzua(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(names(&found), ["MILE-1", "MILE-4", "MILE-6"], "{report}");
    let by = |id: &str| -> String {
        found
            .iter()
            .find(|f| f["file"].as_str().unwrap().contains(id))
            .unwrap()["message"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(by("MILE-1-").contains("Planned") && by("MILE-1-").contains("Implemented"));
    assert!(by("MILE-6-").contains("Verified"));
    assert!(by("MILE-4-").contains("Done") && by("MILE-4-").contains("Not started"));
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "status.delivery-agreement")
        .unwrap();
    assert_eq!(rule["population"]["examined"], 8, "{rule}");
}

#[test]
fn a_source_back_pointer_is_delivery_evidence_for_a_record_with_no_realized_by() {
    let rules = "  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n";
    let dir = repo("backptr", &config(DELIVERY, rules));
    std::fs::write(dir.join("src/b.rs"), "// Implements: MILE-2\nfn b() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "pointer"]);
    let report = parsed(&run_urzua(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(
        names(&found),
        ["MILE-1", "MILE-2", "MILE-4", "MILE-6"],
        "{report}"
    );
}

#[test]
fn fix_proposes_the_success_status_for_delivered_records_and_never_downgrades() {
    let dir = repo("fix", &config(DELIVERY, ""));
    let report = parsed(&run_urzua(&dir, &["fix"]));
    let repairs = report["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 2, "{report}");
    for r in repairs {
        assert_eq!(r["field"], "Status");
        assert_eq!(r["computed_value"], "Done");
        assert!(r["evidence"].as_str().unwrap().contains("Realized-by"));
    }
    // The reverse direction is a notice naming the record, not a repair.
    let notices = serde_json::to_string(&report["notices"]).unwrap();
    assert!(
        notices.contains("MILE-4-") && notices.contains("no delivery evidence"),
        "{report}"
    );
}

#[test]
fn fix_apply_writes_the_status_and_a_structural_revision_row() {
    let dir = repo("apply", &config(DELIVERY, ""));
    let out = run_urzua(&dir, &["fix", "--apply", "--force", "--by", "tester"]);
    let report = parsed(&out);
    assert_eq!(report["status"], "applied", "{report}");
    for n in [1, 6] {
        let text =
            std::fs::read_to_string(dir.join(format!("docs/milestones/MILE-{n}-x.md"))).unwrap();
        assert!(text.contains("\nStatus: Done\n"), "{text}");
        assert!(!text.contains("Status: Planned") && !text.contains("Status: InProgress"));
        let row = text
            .lines()
            .rev()
            .find(|l| l.contains("Tool-authored"))
            .unwrap();
        assert!(
            row.contains("Status") && row.contains("Done") && row.contains("**structural**"),
            "{row}"
        );
    }
    let untouched = std::fs::read_to_string(dir.join("docs/milestones/MILE-4-x.md")).unwrap();
    assert!(untouched.contains("Status: Done") && !untouched.contains("Tool-authored"));
    let again = parsed(&run_urzua(&dir, &["check"]));
    assert_eq!(
        names(&findings(&again, "status.delivery-agreement")),
        ["MILE-4"],
        "{again}"
    );
}

#[test]
fn a_delivery_declaration_naming_a_status_outside_the_type_is_refused_at_load() {
    for (bad, what) in [
        (
            DELIVERY.replace("set_status: Done", "set_status: Planned"),
            "set_status",
        ),
        (
            DELIVERY.replace("[Planned, InProgress, Blocked]", "[Planned, Done]"),
            "not_done",
        ),
        (
            DELIVERY.replace("[Implemented, Verified]", "[Implemented, Nope]"),
            "delivered_tiers",
        ),
        (
            DELIVERY.replace("[\"Not started\"]", "[Implemented]"),
            "unevidenced_tiers",
        ),
    ] {
        let dir = repo("bad", &config(&bad, ""));
        let out = run_urzua(&dir, &["check"]);
        assert!(!out.status.success(), "{what}");
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(text.contains(what), "{what}: {text}");
    }
}

#[test]
fn the_rule_enabled_on_a_corpus_with_no_declaration_examines_nothing_and_says_so() {
    let dir = repo("undeclared", &config("", ""));
    let report = parsed(&run_urzua(&dir, &["check"]));
    assert!(findings(&report, "status.delivery-agreement").is_empty());
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "status.delivery-agreement")
        .unwrap();
    assert_eq!(rule["population"]["examined"], 0, "{rule}");
}

#[test]
fn a_type_can_switch_a_source_back_pointer_off_as_delivery_evidence_observed_failing() {
    let rules = "  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n";
    let off = format!("{DELIVERY}        back_pointer_evidence: false\n");
    let dir = repo("backptr-off", &config(&off, rules));
    std::fs::write(dir.join("src/b.rs"), "// Implements: MILE-2\nfn b() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "pointer"]);
    let report = parsed(&run_urzua(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    // MILE-2 is cited only by a back-pointer: with the option off it is not delivered.
    assert_eq!(names(&found), ["MILE-1", "MILE-4", "MILE-6"], "{report}");
    // Realized-by evidence still counts, and fix proposes only those.
    let fixed = parsed(&run_urzua(&dir, &["fix"]));
    assert_eq!(fixed["repairs"].as_array().unwrap().len(), 2, "{fixed}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_back_pointer_option_defaults_to_counting_and_accepts_true() {
    let rules = "  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n";
    let on = format!("{DELIVERY}        back_pointer_evidence: true\n");
    let dir = repo("backptr-on", &config(&on, rules));
    std::fs::write(dir.join("src/b.rs"), "// Implements: MILE-2\nfn b() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "pointer"]);
    let report = parsed(&run_urzua(&dir, &["check"]));
    assert_eq!(
        names(&findings(&report, "status.delivery-agreement")),
        ["MILE-1", "MILE-2", "MILE-4", "MILE-6"],
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_test_locator_list_in_realized_by_is_delivery_evidence_with_a_run_command_action() {
    let dir = repo("waiver", &config(DELIVERY, ""));
    std::fs::write(
        dir.join("docs/milestones/MILE-9-x.md"),
        milestone("InProgress", Some("test:tests/a.rs#t, test:src/a.rs#a")),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "nine"]);
    let report = parsed(&run_urzua(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert!(names(&found).contains(&"MILE-9".to_string()), "{report}");
    let nine = found
        .iter()
        .find(|f| f["file"].as_str().unwrap().contains("MILE-9-"))
        .unwrap();
    assert!(nine["message"].as_str().unwrap().contains("Realized-by"));
    assert_eq!(nine["suggested_action"]["kind"], "run-command", "{nine}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_type_declaring_no_unevidenced_tiers_never_reports_a_success_status_without_evidence_observed_failing(
) {
    let none = DELIVERY.replace(
        "unevidenced_tiers: [\"Not started\"]",
        "unevidenced_tiers: []",
    );
    let dir = repo("no-unevidenced", &config(&none, ""));
    let out = run_urzua(&dir, &["check"]);
    let report = parsed(&out);
    // MILE-4 is Done with no evidence: the opposite direction is declared off.
    assert_eq!(
        names(&findings(&report, "status.delivery-agreement")),
        ["MILE-1", "MILE-6"],
        "{report}"
    );
    let fixed = parsed(&run_urzua(&dir, &["fix"]));
    let notices = serde_json::to_string(&fixed["notices"]).unwrap();
    assert!(!notices.contains("no delivery evidence"), "{fixed}");
    std::fs::remove_dir_all(&dir).ok();
}
