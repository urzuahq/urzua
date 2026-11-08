//! `MILE-214` / `ADR-96`: a type that declares `statuses.delivery.items` has its
//! Done means read as a list of items that each name their evidence, and its
//! `Status` computed from them. Every test runs the real binary over a fixture
//! corpus whose config declares `items`; the undeclared config is the control.
//!
//! Verifies: MILE-214

mod common;

use std::path::Path;
use std::process::Command;

const ITEMS: &str = "      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
        items:
          section: \"Done means\"
          clause_separator: \" — \"
          evidence_marker: \"evidence:\"
          waiver_marker: \"waived:\"
          partial_status: \"InProgress\"
          resolving_tiers: [Specified, Implemented, Verified]
          waiver_categories:
            - {name: not-applicable}
            - {name: superseded-by-decision, cite: required}
            - {name: deferred-to, cite: required, expires: required}
            - {name: process-outcome}
";

const NO_ITEMS: &str = "      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [\"Not started\"]
        not_done: [Planned, InProgress]
        set_status: Done
";

fn config(delivery: &str, rule_level: &str) -> String {
    format!(
        "schema_version: 2
roadmap:
  pairs: [sequencing]
relation_pairs:
  - {{name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}}
rules:
  status.delivery-agreement: {rule_level}
  status.done-items-shape: error
  status.waived-item-stale: warn
  embodiment.locator-exists: error
  waiver.matches-nothing: warn
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Realized-by, Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    sections:
      from: headings
      required:
        - {{name: Done means}}
    statuses:
      values: [Planned, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      progress: [InProgress, Done]
{delivery}  waiver:
    dir: docs/waiver
    role: suppression
    required_fields: [Rule, Scope, Reason]
    known_fields: [Expires]
    header_shape: yaml-frontmatter
"
    )
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap()
        .success());
}

/// A milestone whose Done means is `done_means`, closed by a revision log.
fn milestone(status: &str, header_extra: &str, done_means: &str) -> String {
    format!(
        "---\nStatus: {status}\n{header_extra}---\n# X\n\n## Done means\n\n{done_means}\n## Revision log\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n"
    )
}

/// A repository holding `milestones` (`(number, content)`), the evidence files
/// the items cite and the config.
fn repo(name: &str, config: &str, milestones: &[(u32, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-items-{name}"));
    for d in [
        "docs/milestones",
        "docs/waiver",
        "docs/specs",
        "src",
        "tests",
        ".urzua",
    ] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    for (n, content) in milestones {
        std::fs::write(dir.join(format!("docs/milestones/MILE-{n}-m.md")), content).unwrap();
    }
    std::fs::write(dir.join("docs/specs/s.md"), "spec\n").unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("tests/a.rs"), "fn t() {}\n").unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
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

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|a| a.iter().filter(|f| f["rule"] == rule).collect())
        .unwrap_or_default()
}

fn on<'a>(found: &[&'a serde_json::Value], file: &str) -> Option<&'a serde_json::Value> {
    found
        .iter()
        .copied()
        .find(|f| f["file"].as_str().unwrap().contains(file))
}

const TWO_ITEMS_ONE_EVIDENCED: &str = "- It is built. — evidence: code:src/a.rs\n- It is tested.\n";
const BOTH_EVIDENCED: &str =
    "- It is built. — evidence: code:src/a.rs\n- It is tested. — evidence: test:tests/a.rs\n";

// ---------------------------------------------------------------- opt-in

#[test]
fn an_undeclared_config_judges_exactly_as_before() {
    // The same milestone: a Done status, one resolvable locator in `Realized-by`,
    // and Done means items that carry no evidence. Without `items` declared the
    // locator alone is delivery evidence and the items are text.
    let done = milestone(
        "Done",
        "Realized-by: code:src/a.rs\n",
        TWO_ITEMS_ONE_EVIDENCED,
    );
    let dir = repo(
        "undeclared",
        &config(NO_ITEMS, "error"),
        &[(1, done.clone())],
    );
    let out = run(&dir, &["check"]);
    let report = parsed(&out);
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert!(findings(&report, "status.delivery-agreement").is_empty());
    // The rules of the opt-in examine nothing.
    for rule in ["status.done-items-shape", "status.waived-item-stale"] {
        let exec = report["rules_executed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule"] == rule)
            .unwrap_or_else(|| panic!("{rule} absent: {report}"));
        assert_eq!(exec["population"]["eligible"], 0, "{exec}");
    }
    // Planned with the same Realized-by is the old disagreement, untouched.
    let planned = milestone(
        "Planned",
        "Realized-by: code:src/a.rs\n",
        TWO_ITEMS_ONE_EVIDENCED,
    );
    let dir = repo(
        "undeclared-planned",
        &config(NO_ITEMS, "error"),
        &[(1, planned)],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"].as_str().unwrap().contains("'Done'"));
}

// ------------------------------------------------------- status computation

#[test]
fn every_item_resolved_computes_delivered_and_fix_writes_done() {
    let dir = repo(
        "delivered",
        &config(ITEMS, "error"),
        &[(1, milestone("Planned", "", BOTH_EVIDENCED))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("set Status to 'Done'"));
    let fix = parsed(&run(&dir, &["fix"]));
    assert_eq!(fix["repairs"][0]["computed_value"], "Done", "{fix}");
    let out = run(
        &dir,
        &[
            "fix",
            "--apply",
            "--ids",
            "docs/milestones/MILE-1-m.md",
            "--by",
            "tester",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let text = std::fs::read_to_string(dir.join("docs/milestones/MILE-1-m.md")).unwrap();
    assert!(text.contains("Status: Done"), "{text}");
    assert!(
        text.contains("2 of 2 items resolved"),
        "the row cites the counts: {text}"
    );
}

#[test]
fn some_items_resolved_is_partial_under_planned_and_fix_writes_in_progress() {
    let dir = repo(
        "partial-planned",
        &config(ITEMS, "error"),
        &[(1, milestone("Planned", "", TWO_ITEMS_ONE_EVIDENCED))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("'InProgress'"),
        "{report}"
    );
    let out = run(
        &dir,
        &[
            "fix",
            "--apply",
            "--ids",
            "docs/milestones/MILE-1-m.md",
            "--by",
            "tester",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let text = std::fs::read_to_string(dir.join("docs/milestones/MILE-1-m.md")).unwrap();
    assert!(text.contains("Status: InProgress"), "{text}");
    assert!(text.contains("1 of 2 items resolved"), "{text}");
    // And InProgress is correct for a partial record: the next check is clean.
    let again = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&again, "status.delivery-agreement").is_empty(),
        "{again}"
    );
}

#[test]
fn partial_work_under_in_progress_is_not_reported() {
    let dir = repo(
        "partial-in-progress",
        &config(ITEMS, "error"),
        &[(1, milestone("InProgress", "", TWO_ITEMS_ONE_EVIDENCED))],
    );
    let out = run(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(0), "{}", parsed(&out));
}

#[test]
fn only_spec_evidence_is_specified_only_and_forces_neither_status() {
    let spec_only = "- It is decided. — evidence: spec:docs/specs/s.md\n- It is written down. — evidence: spec:docs/specs/s.md\n";
    for status in ["Planned", "Done"] {
        let dir = repo(
            "specified-only",
            &config(ITEMS, "error"),
            &[(1, milestone(status, "", spec_only))],
        );
        let out = run(&dir, &["check"]);
        let report = parsed(&out);
        assert_eq!(out.status.code(), Some(0), "{status}: {report}");
        assert!(
            findings(&report, "status.delivery-agreement").is_empty(),
            "{status}: {report}"
        );
    }
}

#[test]
fn a_spec_locator_alone_never_computes_a_partial_or_delivered_milestone() {
    // The `r_d >= 1` guard: item 1 is "implemented" and resolved only by a spec.
    let body = "- It is implemented. — evidence: spec:docs/specs/s.md\n- It is documented. — evidence: spec:docs/specs/s.md\n- It is built.\n";
    let dir = repo(
        "spec-guard",
        &config(ITEMS, "error"),
        &[(1, milestone("Planned", "", body))],
    );
    let report = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "{report}"
    );
}

#[test]
fn every_item_waived_is_none_and_a_done_milestone_has_no_evidence() {
    let body = "- Gone. — waived: not-applicable -- no longer applies\n";
    let done = repo(
        "all-waived-done",
        &config(ITEMS, "error"),
        &[(1, milestone("Done", "", body))],
    );
    let report = parsed(&run(&done, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("add the evidence that delivered it"));
    let planned = repo(
        "all-waived-planned",
        &config(ITEMS, "error"),
        &[(1, milestone("Planned", "", body))],
    );
    let report = parsed(&run(&planned, &["check"]));
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "{report}"
    );
}

#[test]
fn a_record_level_locator_never_makes_an_opted_in_milestone_delivered() {
    let dir = repo(
        "record-level",
        &config(ITEMS, "error"),
        &[(
            1,
            milestone(
                "Planned",
                "Realized-by: test:tests/a.rs, code:src/a.rs\n",
                "- It is built.\n",
            ),
        )],
    );
    let report = parsed(&run(&dir, &["check"]));
    assert!(
        findings(&report, "status.delivery-agreement").is_empty(),
        "{report}"
    );
}

// ----------------------------------------------- DoneWithUnresolvedItems

fn unresolved_repo(name: &str, rule_level: &str) -> common::TempDir {
    repo(
        name,
        &config(ITEMS, rule_level),
        &[(1, milestone("Done", "", "- It is built. — evidence: code:src/a.rs\n- It is tested. — evidence: test:tests/missing.rs\n- It is documented.\n"))],
    )
}

#[test]
fn done_with_unresolved_items_names_each_item_and_its_cause() {
    let dir = unresolved_repo("unresolved", "error");
    let out = run(&dir, &["check"]);
    let report = parsed(&out);
    assert_eq!(out.status.code(), Some(1), "{report}");
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    let message = found[0]["message"].as_str().unwrap();
    assert!(
        message.contains("MILE-1 is Done but Done means item 2 \"It is tested.\" is unresolved: locator tests/missing.rs does not exist"),
        "{message}"
    );
    assert!(
        message.contains("MILE-1 is Done but Done means item 3 \"It is documented.\" is unresolved: no evidence clause"),
        "{message}"
    );
    assert!(
        !message.contains("item 1"),
        "a resolved item is not named: {message}"
    );
    assert_eq!(found[0]["severity"], "error");
    // The action reopens to the computed partial status.
    assert_eq!(
        found[0]["suggested_action"]["kind"], "edit-field",
        "{report}"
    );
    assert_eq!(
        found[0]["suggested_action"]["target"]["field"], "Status",
        "{report}"
    );
    assert_eq!(
        found[0]["suggested_action"]["value"], "InProgress",
        "{report}"
    );
}

#[test]
fn a_reopen_the_lifecycle_forbids_is_not_offered_and_the_evidence_edit_is() {
    // Done may only go back to Planned here, so the computed InProgress is not a
    // move the finding can suggest: it offers the edit that adds the evidence.
    let cfg = config(ITEMS, "error").replace(
        "      progress: [InProgress, Done]\n",
        "      progress: [InProgress, Done]\n      initial: [Planned, Done]\n      transitions:\n        Planned: [InProgress, Done, WontDo]\n        InProgress: [Planned, Done, WontDo]\n        Done: [Planned]\n",
    );
    let dir = repo(
        "no-reopen",
        &cfg,
        &[(1, milestone("Done", "", TWO_ITEMS_ONE_EVIDENCED))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(
        found[0]["suggested_action"]["kind"], "edit-section",
        "{report}"
    );
    assert_eq!(
        found[0]["suggested_action"]["target"]["section"], "Done means",
        "{report}"
    );
}

#[test]
fn a_cause_names_a_locator_that_resolves_below_the_resolving_tiers() {
    let strict = ITEMS.replace(
        "[Specified, Implemented, Verified]",
        "[Implemented, Verified]",
    );
    let dir = repo(
        "below-tiers",
        &config(&strict, "error"),
        &[(1, milestone("Done", "", "- It is built. — evidence: code:src/a.rs\n- It is decided. — evidence: spec:docs/specs/s.md\n"))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    assert!(
        found[0]["message"].as_str().unwrap().contains(
            "locator docs/specs/s.md resolves only to Specified, which is not in resolving_tiers"
        ),
        "{report}"
    );
}

#[test]
fn the_variant_fails_check_under_every_fail_on_and_every_rule_level() {
    for rule_level in ["error", "warn", "info"] {
        let dir = unresolved_repo(&format!("floor-{rule_level}"), rule_level);
        for fail_on in ["error", "warning"] {
            let out = run(&dir, &["check", "--fail-on", fail_on]);
            let report = parsed(&out);
            assert_eq!(
                out.status.code(),
                Some(1),
                "level {rule_level}, fail_on {fail_on}: {report}"
            );
            let found = findings(&report, "status.delivery-agreement");
            assert_eq!(
                found[0]["severity"], "error",
                "level {rule_level}: {report}"
            );
        }
    }
}

#[test]
fn a_rule_level_waiver_does_not_clear_it_and_still_waives_the_other_variants() {
    let dir = unresolved_repo("unwaivable", "error");
    // A dated waiver whose Rule and Scope match the finding.
    std::fs::write(
        dir.join("docs/waiver/WAIVER-1-w.md"),
        "---\nRule: status.delivery-agreement\nScope: docs/milestones/MILE-1-m.md\nReason: held\nExpires: 2999-01-01\n---\n# w\n",
    )
    .unwrap();
    // The same rule's other variant, on another milestone, is still waivable.
    std::fs::write(
        dir.join("docs/milestones/MILE-2-m.md"),
        milestone("Planned", "", BOTH_EVIDENCED),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waiver/WAIVER-2-w.md"),
        "---\nRule: status.delivery-agreement\nScope: docs/milestones/MILE-2-m.md\nReason: held\nExpires: 2999-01-01\n---\n# w\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let out = run(&dir, &["check"]);
    let report = parsed(&out);
    assert_eq!(out.status.code(), Some(1), "{report}");
    let found = findings(&report, "status.delivery-agreement");
    let unresolved = on(&found, "MILE-1-m.md").expect("the unresolved finding stands");
    assert!(unresolved["waived"].is_null(), "{unresolved}");
    let other = on(&found, "MILE-2-m.md").expect("the other variant is reported");
    assert!(!other["waived"].is_null(), "still waived: {other}");
    // The waiver that matched nothing is reported as before.
    let nothing = findings(&report, "waiver.matches-nothing");
    assert!(
        nothing
            .iter()
            .any(|f| f["file"].as_str().unwrap().contains("WAIVER-1")),
        "{report}"
    );
}

#[test]
fn fix_apply_never_downgrades_a_success_status() {
    let dir = unresolved_repo("no-downgrade", "error");
    let before = std::fs::read_to_string(dir.join("docs/milestones/MILE-1-m.md")).unwrap();
    let detect = parsed(&run(&dir, &["fix"]));
    assert_eq!(detect["repairs"].as_array().unwrap().len(), 0, "{detect}");
    let notice = detect["notices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["message"].as_str().unwrap().contains("not downgraded"));
    assert!(notice, "{detect}");
    let out = run(&dir, &["fix", "--apply", "--force", "--by", "tester"]);
    assert!(out.status.success());
    let after = std::fs::read_to_string(dir.join("docs/milestones/MILE-1-m.md")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn rules_prints_both_actions_the_floor_and_that_no_waiver_matches() {
    let out = run(&std::env::temp_dir(), &["rules"]);
    let report = parsed(&out);
    let rule = report["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["rule"] == "status.delivery-agreement" || r["id"] == "status.delivery-agreement"
        })
        .unwrap();
    let actions = rule["actions"].as_array().unwrap();
    let reopen = actions
        .iter()
        .find(|a| a["variant"] == "done-with-unresolved-items")
        .unwrap();
    assert_eq!(reopen["kind"], "edit-field");
    assert_eq!(reopen["severity_floor"], "error");
    assert_eq!(reopen["waivable"], false);
    let evidence = actions
        .iter()
        .find(|a| a["variant"] == "done-with-unresolved-items-evidence")
        .unwrap();
    assert_eq!(evidence["kind"], "edit-section");
    // `delivery-not-done` keeps its meaning and stays waivable.
    let old = actions
        .iter()
        .find(|a| a["variant"] == "delivery-not-done")
        .unwrap();
    assert!(
        old.get("waivable").is_none() && old.get("severity_floor").is_none(),
        "{old}"
    );
}

#[test]
fn a_derived_blocked_planned_record_is_offered_the_first_non_progress_status() {
    // MILE-1 is Done with an unresolved item; MILE-2 depends on a Planned
    // MILE-3, so it is derived-blocked. Its reopen is `Planned`, not a progress
    // status `sequence.status-agreement` would refuse.
    let dir = repo(
        "blocked-reopen",
        &config(ITEMS, "error"),
        &[
            (3, milestone("Planned", "", "- Later.\n")),
            (
                2,
                milestone(
                    "Done",
                    "Depends-on: MILE-3\n",
                    "- It is built. — evidence: code:src/a.rs\n- It is tested.\n",
                ),
            ),
        ],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    let finding = on(&found, "MILE-2-m.md").expect("reported");
    assert_eq!(finding["suggested_action"]["value"], "Planned", "{report}");
}

#[test]
fn partial_evidence_on_a_derived_blocked_planned_record_is_not_repaired() {
    let dir = repo(
        "blocked-partial",
        &config(ITEMS, "error"),
        &[
            (3, milestone("Planned", "", "- Later.\n")),
            (
                2,
                milestone("Planned", "Depends-on: MILE-3\n", TWO_ITEMS_ONE_EVIDENCED),
            ),
        ],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    let finding = on(&found, "MILE-2-m.md").expect("the finding stays");
    assert!(
        finding["message"]
            .as_str()
            .unwrap()
            .contains("does not write 'InProgress'"),
        "{report}"
    );
    let fix = parsed(&run(&dir, &["fix"]));
    assert_eq!(fix["repairs"].as_array().unwrap().len(), 0, "{fix}");
}

// ------------------------------------------- dependencies cap the status

/// The fixture config (`InProgress` and `Done` both `progress` statuses a blocked
/// record cannot claim) with `sequence.status-agreement` on, so both rules judge
/// the same record (`ADR-96`, amendment of 2026-11-07).
fn capped_config(delivery: &str) -> String {
    config(delivery, "error").replace("rules:\n", "rules:\n  sequence.status-agreement: error\n")
}

fn capped_repo(name: &str, status: &str, dependency_status: &str) -> common::TempDir {
    repo(
        name,
        &capped_config(ITEMS),
        &[
            (3, milestone(dependency_status, "", "- Later.\n")),
            (2, milestone(status, "Depends-on: MILE-3\n", BOTH_EVIDENCED)),
        ],
    )
}

#[test]
fn all_items_resolved_with_an_unfinished_dependency_computes_in_progress() {
    let dir = capped_repo("cap-in-progress", "InProgress", "Planned");
    let out = run(&dir, &["check"]);
    let report = parsed(&out);
    // Neither rule rejects it: not `DoneWithUnresolvedItems`, not the
    // delivery disagreement, not the derived blocked value (`ADR-83`).
    assert!(
        findings(&report, "status.delivery-agreement")
            .iter()
            .all(|f| !f["file"].as_str().unwrap().contains("MILE-2-m.md")),
        "{report}"
    );
    let sequence = findings(&report, "sequence.status-agreement");
    assert!(sequence.iter().all(|f| f["severity"] == "info"), "{report}");
    // It says why it skipped the record, as an info notice naming the dependency.
    let notice = on(&sequence, "MILE-2-m.md").expect("an info notice");
    assert!(
        notice["message"]
            .as_str()
            .unwrap()
            .contains("all items proven ahead of dependency MILE-3 (Planned)"),
        "{notice}"
    );
    assert_eq!(out.status.code(), Some(0), "{report}");
    // `fix` agrees: nothing to write, and the notice names the capping dependency.
    let fix = parsed(&run(&dir, &["fix"]));
    assert_eq!(fix["repairs"].as_array().unwrap().len(), 0, "{fix}");
    let notices = fix["notices"].to_string();
    assert!(
        notices.contains("MILE-3 (Planned)") && notices.contains("InProgress"),
        "{fix}"
    );
}

#[test]
fn an_in_progress_record_genuinely_out_of_order_is_still_flagged() {
    // One item is unresolved, so `InProgress` is real progress made while the
    // dependency is unfinished, not the cap: `sequence.status-agreement` flags it.
    let dir = repo(
        "cap-out-of-order",
        &capped_config(ITEMS),
        &[
            (3, milestone("Planned", "", "- Later.\n")),
            (
                2,
                milestone(
                    "InProgress",
                    "Depends-on: MILE-3\n",
                    TWO_ITEMS_ONE_EVIDENCED,
                ),
            ),
        ],
    );
    let out = run(&dir, &["check"]);
    let report = parsed(&out);
    let sequence = findings(&report, "sequence.status-agreement");
    let finding = on(&sequence, "MILE-2-m.md").expect("flagged");
    assert_eq!(finding["severity"], "error", "{report}");
    assert!(
        finding["message"]
            .as_str()
            .unwrap()
            .contains("claims progress"),
        "{finding}"
    );
    assert!(
        !finding["message"]
            .as_str()
            .unwrap()
            .contains("proven ahead"),
        "{finding}"
    );
    assert_eq!(out.status.code(), Some(1), "{report}");
}

#[test]
fn a_planned_record_whose_items_all_resolve_is_set_to_in_progress_not_done() {
    let dir = capped_repo("cap-planned", "Planned", "InProgress");
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    let finding = on(&found, "MILE-2-m.md").expect("the disagreement is reported");
    let message = finding["message"].as_str().unwrap();
    assert!(
        message.contains("set Status to 'InProgress'")
            && message.contains("MILE-3 (InProgress)")
            && !message.contains("'Done'"),
        "{message}"
    );
    let fix = parsed(&run(&dir, &["fix"]));
    let repairs = fix["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{fix}");
    assert_eq!(repairs[0]["computed_value"], "InProgress", "{fix}");
}

#[test]
fn a_done_record_with_an_unfinished_dependency_is_not_downgraded_and_the_notice_names_it() {
    let dir = capped_repo("cap-done", "Done", "Planned");
    let before = std::fs::read_to_string(dir.join("docs/milestones/MILE-2-m.md")).unwrap();
    let fix = parsed(&run(&dir, &["fix"]));
    assert_eq!(fix["repairs"].as_array().unwrap().len(), 0, "{fix}");
    let notices = fix["notices"].to_string();
    assert!(
        notices.contains("MILE-3 (Planned)") && notices.contains("not downgraded"),
        "{fix}"
    );
    run(&dir, &["fix", "--apply", "--force", "--by", "tester"]);
    let after = std::fs::read_to_string(dir.join("docs/milestones/MILE-2-m.md")).unwrap();
    assert_eq!(before, after);
    // The derived blocked value still refuses `Done`, naming the dependency.
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "sequence.status-agreement");
    assert!(
        on(&found, "MILE-2-m.md")
            .is_some_and(|f| f["message"].as_str().unwrap().contains("MILE-3")),
        "{report}"
    );
}

#[test]
fn a_finished_dependency_does_not_cap_the_status() {
    let dir = capped_repo("cap-finished", "Planned", "Done");
    let fix = parsed(&run(&dir, &["fix"]));
    let repairs = fix["repairs"].as_array().unwrap();
    let mile2 = repairs
        .iter()
        .find(|r| {
            r["record"]
                .as_str()
                .unwrap_or_default()
                .contains("MILE-2-m.md")
        })
        .unwrap_or_else(|| panic!("{fix}"));
    assert_eq!(mile2["computed_value"], "Done", "{fix}");
}

#[test]
fn a_dependency_in_a_failure_status_or_with_no_status_caps_like_an_unfinished_one() {
    let dir = capped_repo("cap-wontdo", "Planned", "WontDo");
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    let message = on(&found, "MILE-2-m.md").unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        message.contains("'InProgress'") && message.contains("MILE-3 (WontDo)"),
        "{message}"
    );
}

#[test]
fn a_config_without_items_is_not_capped() {
    // `Realized-by` makes the record delivered; the open dependency does not
    // change what the delivery rule computes when `items` is not declared.
    let dir = repo(
        "cap-no-items",
        &config(NO_ITEMS, "error"),
        &[
            (3, milestone("Planned", "", "- Later.\n")),
            (
                2,
                milestone(
                    "Planned",
                    "Depends-on: MILE-3\nRealized-by: code:src/a.rs\n",
                    BOTH_EVIDENCED,
                ),
            ),
        ],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    let message = on(&found, "MILE-2-m.md").unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains("set Status to 'Done'"), "{message}");
}

#[test]
fn without_declared_dependencies_the_items_alone_decide() {
    // The derived blocked value needs a declared pair, so its rule goes too.
    let config = capped_config(ITEMS)
        .replace("roadmap:\n  pairs: [sequencing]\n", "")
        .replace("  sequence.status-agreement: error\n", "");
    let dir = repo(
        "cap-no-pairs",
        &config,
        &[
            (3, milestone("Planned", "", "- Later.\n")),
            (
                2,
                milestone("Planned", "Depends-on: MILE-3\n", BOTH_EVIDENCED),
            ),
        ],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.delivery-agreement");
    let message = on(&found, "MILE-2-m.md").unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains("set Status to 'Done'"), "{message}");
}

// --------------------------------------------------------- shape findings

fn shape(name: &str, body: &str) -> Vec<serde_json::Value> {
    let dir = repo(
        name,
        &config(ITEMS, "error"),
        &[(1, milestone("Planned", "", body))],
    );
    let report = parsed(&run(&dir, &["check"]));
    findings(&report, "status.done-items-shape")
        .into_iter()
        .cloned()
        .collect()
}

#[test]
fn the_worked_examples_that_are_well_formed_have_no_shape_finding() {
    let body = "- A declared alias. — evidence: code:src/a.rs, test:tests/a.rs\n\
- Comma path. — evidence: code:src/a.rs, test:tests/a.rs\n\
- The report prints evidence: per item, with no separator before the word.\n\
- Every rule reads through one function.\n  - one\n  - two\n\n  evidence: code:src/a.rs, test:tests/a.rs\n\
1. The notes name the change. — waived: process-outcome -- written at release, see evidence: CI and #512\n\
- A second platform. — evidence: test:tests/a.rs — waived: deferred-to (MILE-9, 2999-01-31) -- no runner until MILE-9\n";
    let found = shape("shape-ok", body);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn examples_seven_eight_and_nine_are_each_a_shape_finding() {
    let body = "- The reader accepts `code:` locators. — evidence: `code:src/a.rs`\n\
- The reader is fast. — evidence: test:tests/a.rs because it measures it\n\
- [x] The doc names the command. — evidence: spec:docs/specs/s.md\n";
    let found = shape("shape-bad", body);
    assert_eq!(found.len(), 3, "{found:?}");
    let all: String = found
        .iter()
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    assert!(
        all.contains("item 1") && all.contains("no backticks"),
        "{all}"
    );
    assert!(
        all.contains("item 2") && all.contains("because it measures it"),
        "{all}"
    );
    assert!(all.contains("item 3") && all.contains("checkbox"), "{all}");
    // Each finding offers an edit of the section, on its own line.
    for f in &found {
        assert_eq!(f["suggested_action"]["kind"], "edit-section", "{f}");
        assert_eq!(
            f["suggested_action"]["target"]["section"], "Done means",
            "{f}"
        );
        assert!(f["line"].is_number(), "{f}");
    }
}

#[test]
fn prose_only_and_misplaced_clauses_are_shape_findings() {
    let found = shape("shape-prose", "One paragraph; it should be a list.\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0]["message"].as_str().unwrap().contains("prose only"));
    let found = shape(
        "shape-both",
        "- Item — evidence: code:src/a.rs\n  evidence: test:tests/a.rs\n",
    );
    assert!(
        found[0]["message"].as_str().unwrap().contains("at once"),
        "{found:?}"
    );
}

#[test]
fn waiver_clause_errors_are_shape_findings() {
    for (body, want) in [
        (
            "- t — waived: made-up -- r\n",
            "not a declared waiver category",
        ),
        ("- t — waived: not-applicable -- \n", "no ` -- <reason>`"),
        (
            "- t — waived: superseded-by-decision -- r\n",
            "requires a cite",
        ),
        (
            "- t — waived: deferred-to (MILE-9) -- r\n",
            "requires an expiry date",
        ),
        (
            "- t — waived: deferred-to (PR 5, 2999-01-01) -- r\n",
            "not a record id",
        ),
    ] {
        let found = shape("shape-waiver", body);
        assert!(
            found
                .iter()
                .any(|f| f["message"].as_str().unwrap().contains(want)),
            "{body:?} wanted {want:?}: {found:?}"
        );
    }
}

// ------------------------------------------------------------- stale waivers

fn stale(name: &str, body: &str, extra: &[(u32, String)]) -> Vec<serde_json::Value> {
    let mut all = vec![(1, milestone("InProgress", "", body))];
    all.extend(extra.iter().cloned());
    let dir = repo(name, &config(ITEMS, "error"), &all);
    let report = parsed(&run(&dir, &["check"]));
    findings(&report, "status.waived-item-stale")
        .into_iter()
        .cloned()
        .collect()
}

#[test]
fn a_waived_item_whose_evidence_now_resolves_is_stale() {
    let body =
        "- It builds. — evidence: code:src/a.rs — waived: not-applicable -- was not buildable\n";
    let found = stale("stale-resolves", body, &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("its evidence now resolves"));
    assert_eq!(found[0]["severity"], "warning");
    // The waiver holds while the locator does not resolve.
    let body = "- It builds. — evidence: code:src/missing.rs — waived: not-applicable -- not yet\n";
    assert!(stale("stale-holds", body, &[]).is_empty());
}

#[test]
fn a_deferral_whose_target_is_missing_or_finished_is_stale() {
    let body = "- Later. — waived: deferred-to (MILE-9, 2999-01-31) -- carried by MILE-9\n";
    let missing = stale("stale-missing", body, &[]);
    assert!(
        missing.iter().any(|f| f["message"]
            .as_str()
            .unwrap()
            .contains("MILE-9 does not exist")),
        "{missing:?}"
    );
    let done = (9, milestone("Done", "", BOTH_EVIDENCED));
    let finished = stale("stale-finished", body, &[done]);
    assert!(
        finished
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("is finished")),
        "{finished:?}"
    );
    let open = (9, milestone("Planned", "", "- Open.\n"));
    assert!(stale("stale-open", body, &[open]).is_empty());
}

#[test]
fn a_waiver_past_its_expiry_is_stale_and_each_waived_item_is_listed_by_category() {
    let body = "- Later. — waived: deferred-to (MILE-9, 2020-01-31) -- carried by MILE-9\n- Never. — waived: not-applicable -- gone\n";
    let dir = repo(
        "stale-expired",
        &config(ITEMS, "error"),
        &[
            (1, milestone("InProgress", "", body)),
            (9, milestone("Planned", "", "- Open.\n")),
        ],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "status.waived-item-stale");
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("its expiry 2020-01-31 has come"));
    let notices: Vec<&str> = report["notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["message"].as_str().unwrap())
        .collect();
    assert!(
        notices
            .iter()
            .any(|m| m.contains("item 2") && m.contains("not-applicable")),
        "{notices:?}"
    );
    assert!(
        notices.iter().any(|m| m
            .contains("2 waived Done means item(s) by category: deferred-to 1, not-applicable 1")),
        "{notices:?}"
    );
}

// -------------------------------------------------------------- config load

#[test]
fn invalid_items_declarations_are_refused_at_load() {
    let cases: Vec<(String, &str)> = vec![
        (
            ITEMS.replace("section: \"Done means\"", "section: \"Why\""),
            "does not require",
        ),
        (
            ITEMS.replace("partial_status: \"InProgress\"", "partial_status: \"Nope\""),
            "not in `values`",
        ),
        (
            ITEMS.replace("partial_status: \"InProgress\"", "partial_status: \"Done\""),
            "not in `delivery.not_done`",
        ),
        (
            ITEMS.replace("[Specified, Implemented, Verified]", "[Imaginary]"),
            "not a declared embodiment tier",
        ),
        (
            ITEMS.replace("[Specified, Implemented, Verified]", "[Implemented]"),
            "`delivery.delivered_tiers` names 'Verified', which is not in `delivery.items.resolving_tiers` (Implemented)",
        ),
        (
            ITEMS.replace(
                "evidence_marker: \"evidence:\"",
                "evidence_marker: \"waived:\"",
            ),
            "are both",
        ),
        (
            ITEMS.replace(
                "waiver_marker: \"waived:\"",
                "waiver_marker: \"evidence:x\"",
            ),
            "prefix of the other",
        ),
        (
            ITEMS.replace("clause_separator: \" — \"", "clause_separator: \"\""),
            "clause_separator",
        ),
        (
            ITEMS.replace("{name: process-outcome}", "{name: not-applicable}"),
            "twice",
        ),
        (
            ITEMS.replace(
                "{name: deferred-to, cite: required, expires: required}",
                "{name: deferred-to, expires: required}",
            ),
            "requires an expiry but no cite",
        ),
    ];
    for (items, want) in cases {
        let dir = common::TempDir::new("urzua-items-badcfg");
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(dir.join(".urzua/config.yaml"), config(&items, "error")).unwrap();
        git(&dir, &["init", "-q"]);
        let out = run(&dir, &["check"]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_ne!(out.status.code(), Some(0), "{want}: {text}");
        assert!(text.contains(want), "wanted {want:?}: {text}");
    }
}

#[test]
fn items_on_a_type_whose_delivery_rule_is_off_are_refused() {
    for level in ["off", "none"] {
        let cfg = if level == "off" {
            config(ITEMS, "off")
        } else {
            config(ITEMS, "error").replace("  status.delivery-agreement: error\n", "")
        };
        let dir = common::TempDir::new("urzua-items-ruleoff");
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
        git(&dir, &["init", "-q"]);
        let out = run(&dir, &["check"]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_ne!(out.status.code(), Some(0), "{text}");
        assert!(
            text.contains("status.delivery-agreement"),
            "{level}: {text}"
        );
    }
}

// ---------------------------------------------------- evidence readers

#[test]
fn an_item_locator_is_checked_by_locator_exists() {
    let dir = repo(
        "locator-exists",
        &config(ITEMS, "error"),
        &[(
            1,
            milestone(
                "InProgress",
                "",
                "- It builds. — evidence: code:src/gone.rs\n",
            ),
        )],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "embodiment.locator-exists");
    assert_eq!(found.len(), 1, "{report}");
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("src/gone.rs"),
        "{report}"
    );
    assert!(
        found[0]["message"]
            .as_str()
            .unwrap()
            .contains("a Done means item"),
        "{report}"
    );
    assert_eq!(
        found[0]["suggested_action"]["kind"], "edit-section",
        "{report}"
    );
}

#[test]
fn a_pin_inside_an_item_clause_is_listed_and_rewritten_by_a_re_pin() {
    let stale_pin = format!("@sha256:{}", "0".repeat(64));
    let body = format!("- It builds. — evidence: code:src/a.rs{stale_pin}\n");
    let dir = repo(
        "rebaseline",
        &config(ITEMS, "error"),
        &[(1, milestone("InProgress", "", &body))],
    );
    let detect = parsed(&run(&dir, &["fix", "--rebaseline"]));
    let listed = detect["rebaselines"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0)
        + detect["repairs"].as_array().map(|a| a.len()).unwrap_or(0);
    assert!(listed >= 1, "{detect}");
    let out = run(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            "docs/milestones/MILE-1-m.md",
            "--by",
            "tester",
            "--reason",
            "the file changed",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let text = std::fs::read_to_string(dir.join("docs/milestones/MILE-1-m.md")).unwrap();
    assert!(!text.contains(&stale_pin), "{text}");
    assert!(text.contains("code:src/a.rs@sha256:"), "{text}");
    assert!(text.contains("Re-baselined"), "{text}");
}

// ---------------------------------------------------------------- roadmap

#[test]
fn the_roadmap_reports_items_and_a_partial_record_keeps_its_dependents_blocked() {
    let dir = repo(
        "roadmap",
        &config(ITEMS, "error"),
        &[
            (
                1,
                milestone(
                    "InProgress",
                    "",
                    "- A. — evidence: code:src/a.rs\n- B.\n- C. — waived: not-applicable -- gone\n",
                ),
            ),
            (2, milestone("Planned", "Depends-on: MILE-1\n", "- D.\n")),
        ],
    );
    let out = run(&dir, &["roadmap"]);
    let report = parsed(&out);
    let records = &report["roadmap"]["records"];
    assert_eq!(
        records["MILE-1"]["items"],
        serde_json::json!({"total": 3, "resolved": 1, "waived": 1}),
        "{report}"
    );
    assert_eq!(records["MILE-1"]["wave"], 0, "{report}");
    assert_eq!(records["MILE-2"]["blocked"], true, "{report}");
    assert_eq!(
        records["MILE-2"]["blocked_by"],
        serde_json::json!(["MILE-1"]),
        "{report}"
    );
    assert_eq!(
        report["roadmap"]["ready"],
        serde_json::json!(["MILE-1"]),
        "{report}"
    );
}

#[test]
fn the_roadmap_leaves_items_off_a_record_of_an_undeclared_type() {
    let dir = repo(
        "roadmap-undeclared",
        &config(NO_ITEMS, "error"),
        &[(1, milestone("InProgress", "", "- A.\n"))],
    );
    let report = parsed(&run(&dir, &["roadmap"]));
    assert!(
        report["roadmap"]["records"]["MILE-1"]
            .get("items")
            .is_none(),
        "{report}"
    );
}

// ------------------------------------------------ round 4: delivery items

#[test]
fn a_waived_item_whose_evidence_does_not_exist_yet_is_not_a_locator_defect() {
    let body = "- A second chain builds. — evidence: test:tests/nope.rs — waived: deferred-to (MILE-9, 2999-01-31) -- no runner until MILE-9\n- It builds. — evidence: code:src/gone.rs\n";
    let dir = repo(
        "waived-missing",
        &config(ITEMS, "error"),
        &[(1, milestone("InProgress", "", body))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "embodiment.locator-exists");
    assert_eq!(
        found.len(),
        1,
        "only the unwaived item is a defect: {report}"
    );
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("src/gone.rs"), "{message}");
    assert!(!message.contains("nope.rs"), "{message}");
}

/// A type that names its section something else gets messages in that name.
#[test]
fn messages_use_the_declared_section_name() {
    let cfg = config(&ITEMS.replace("\"Done means\"", "\"Done when\""), "error")
        .replace("{name: Done means}", "{name: Done when}");
    let content = "---\nStatus: Done\nRealized-by: code:src/a.rs\n---\n# X\n\n## Done when\n\n- It builds. — evidence: code:src/gone.rs\n- It is waived. — waived: not-applicable -- nope\n\n## Revision log\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-04 | Filed. | **substantive** |\n";
    let dir = repo("section-name", &cfg, &[(1, content.to_string())]);
    let report = parsed(&run(&dir, &["check"]));
    let mut texts: Vec<String> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    texts.extend(
        report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["message"].as_str().unwrap().to_string()),
    );
    assert!(
        texts
            .iter()
            .any(|m| m.contains("a Done when item (line 9)")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|m| m.contains("is Done but Done when item 1")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|m| m.contains("Done when item 2") && m.contains("is waived")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|m| m.contains("waived Done when item(s)")),
        "{texts:?}"
    );
    assert!(texts.iter().all(|m| !m.contains("Done means")), "{texts:?}");
}

/// An absolute or `..` locator is never probed on disk: it is "not in the
/// working tree", not "exists but is not tracked".
#[test]
fn a_locator_outside_the_repository_is_not_probed() {
    // A target the test owns, in a directory beside the repository, removed on drop.
    let outside = common::TempDir::new("urzua-items-outside-target");
    std::fs::write(outside.join("target.txt"), "x\n").unwrap();
    let absolute = outside.join("target.txt");
    let name = outside.file_name().unwrap().to_string_lossy().to_string();
    let body = format!(
        "- A. — evidence: code:{}\n- B. — evidence: code:../{name}/target.txt\n",
        absolute.display()
    );
    let dir = repo(
        "outside-probe",
        &config(ITEMS, "error"),
        &[(1, milestone("InProgress", "", &body))],
    );
    let report = parsed(&run(&dir, &["check"]));
    let found = findings(&report, "embodiment.locator-exists");
    assert_eq!(found.len(), 2, "{report}");
    for f in found {
        let m = f["message"].as_str().unwrap();
        assert!(m.contains("not in the working tree"), "{m}");
        assert!(!m.contains("not tracked"), "{m}");
    }
}

/// A waiver that has expired no longer excuses an item whose evidence is missing.
#[test]
fn an_expired_waiver_does_not_excuse_missing_evidence() {
    let live = "- Later. — evidence: code:src/gone.rs — waived: deferred-to (MILE-9, 2999-01-31) -- carried\n";
    let expired = "- Later. — evidence: code:src/gone.rs — waived: deferred-to (MILE-9, 2020-01-31) -- carried\n";
    let count = |name: &str, body: &str| {
        let dir = repo(
            name,
            &config(ITEMS, "error"),
            &[
                (1, milestone("InProgress", "", body)),
                (9, milestone("Planned", "", "- Open.\n")),
            ],
        );
        let report = parsed(&run(&dir, &["check"]));
        findings(&report, "embodiment.locator-exists").len()
    };
    assert_eq!(count("waiver-live", live), 0);
    assert_eq!(count("waiver-expired", expired), 1);
}
