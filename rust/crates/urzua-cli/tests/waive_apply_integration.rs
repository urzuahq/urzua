//! MILE-205 (ADR-91, SPEC-25): `urzua waive --from-report --apply` writes the drafts. A draft is
//! an ordinary waiver with a pending `Reason` and no `Expires`, so `check` reports the original
//! finding again beside `waiver-ineffective` until a person fills it in.

mod common;

use common::waive::*;
use std::path::Path;

fn apply(dir: &Path, extra: &[&str]) -> (i32, serde_json::Value) {
    let mut args = vec!["--apply"];
    args.extend_from_slice(extra);
    waive(dir, &args)
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let (code, stdout, stderr) = run(dir, &["check"]);
    let json = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout} {stderr}"));
    (code, json)
}

fn stage(dir: &Path) {
    git(dir, &["add", "-A"]);
}

fn written_paths(report: &serde_json::Value) -> Vec<String> {
    report["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| str_of(d, "path").to_string())
        .collect()
}

fn unwaived<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule && f.get("waived").is_none())
        .collect()
}

#[test]
fn apply_writes_the_six_drafts_and_each_is_written_true() {
    let dir = fixture("apply-six", two_noisy());
    let (code, report) = apply(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["mode"], "apply");
    let drafts = report["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 6, "{report}");
    for d in drafts {
        assert_eq!(d["written"], true, "{d}");
        assert!(d["stable_id"].as_str().unwrap().len() >= 20, "{d}");
        assert!(dir.join(str_of(d, "path")).is_file(), "{d}");
    }
    let on_disk = std::fs::read_dir(dir.join("docs/waiver")).unwrap().count();
    assert_eq!(on_disk, 6);
}

/// Done-when: `Expires` is omitted (the key is not written), `Category` and `Justified-by`
/// are null; the body lists the finding messages; the revision log records who ran it.
#[test]
fn a_written_draft_has_a_pending_reason_no_expires_key_and_the_findings_as_its_body() {
    let dir = fixture("apply-shape", two_noisy());
    let (_, report) = apply(
        &dir,
        &[
            "--rule",
            "header.required-fields",
            "--path",
            "docs/adr/ADR-1-a.md",
        ],
    );
    let path = &written_paths(&report)[0];
    let text = std::fs::read_to_string(dir.join(path)).unwrap();
    let head = text.split("\n---\n").next().unwrap();
    assert!(head.contains("\nRule: header.required-fields\n"), "{text}");
    assert!(head.contains("\nScope: docs/adr/ADR-1-a.md\n"), "{text}");
    assert!(head.contains("\nReason: Pending\n"), "{text}");
    assert!(head.contains("\nCategory: null\n"), "{text}");
    assert!(head.contains("\nJustified-by: null"), "{text}");
    assert!(!head.contains("Expires"), "Expires must be omitted: {text}");
    assert!(
        !head.contains("Owner"),
        "no Owner without a template line: {text}"
    );
    assert!(
        text.contains("\n## Findings\n\n```text\ndocs/adr/ADR-1-a.md missing required header field 'Owner' for record type 'adr'\n```\n"),
        "{text}"
    );
    assert!(
        text.contains("Drafted by `urzua waive --from-report` (run by tester)"),
        "{text}"
    );
    assert!(text.contains("| **structural** |"), "{text}");
}

#[test]
fn owner_is_written_only_when_the_template_lists_it_and_the_type_declares_it() {
    let with = fixture(
        "apply-owner",
        Setup {
            owner: true,
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    let (_, report) = apply(&with, &["--rule", "field.quality"]);
    let text = std::fs::read_to_string(with.join(&written_paths(&report)[0])).unwrap();
    assert!(
        text.split("\n---\n")
            .next()
            .unwrap()
            .contains("\nOwner: tester"),
        "{text}"
    );

    let without = fixture(
        "apply-no-owner",
        Setup {
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    let (_, report) = apply(&without, &["--rule", "field.quality"]);
    let text = std::fs::read_to_string(without.join(&written_paths(&report)[0])).unwrap();
    assert!(
        !text.split("\n---\n").next().unwrap().contains("Owner"),
        "{text}"
    );
}

#[test]
fn a_second_apply_writes_nothing() {
    let dir = fixture("apply-twice", two_noisy());
    let (_, first) = apply(&dir, &[]);
    assert_eq!(first["drafts"].as_array().unwrap().len(), 6);
    stage(&dir);
    let before = std::fs::read_dir(dir.join("docs/waiver")).unwrap().count();
    let (code, second) = apply(&dir, &[]);
    assert_eq!(code, 0, "{second}");
    assert_eq!(second["drafts"].as_array().unwrap().len(), 0, "{second}");
    assert_eq!(
        std::fs::read_dir(dir.join("docs/waiver")).unwrap().count(),
        before
    );
    let reasons: std::collections::BTreeSet<&str> = second["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["reason"].as_str().unwrap())
        .collect();
    assert!(reasons.contains("already-waived"), "{second}");
    // The drafts' own findings (a pending Reason) are not drafted for either.
    assert!(
        second["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["reason"] != "resolved-by-drafts"),
        "{second}"
    );
}

#[test]
fn check_after_apply_reports_the_original_findings_and_waiver_ineffective_and_exits_1() {
    let dir = fixture("apply-check", two_noisy());
    let (code, before) = check(&dir);
    assert_eq!(code, 1);
    let originals = before["findings"].as_array().unwrap().len();
    apply(&dir, &[]);
    stage(&dir);
    let (code, after) = check(&dir);
    assert_eq!(code, 1, "{after}");
    assert!(after["findings"].as_array().unwrap().len() > originals);
    assert_eq!(unwaived(&after, "field.quality").len(), 4, "{after}");
    let ineffective: Vec<&serde_json::Value> = unwaived(&after, "waiver.matches-nothing");
    assert_eq!(ineffective.len(), 6, "{after}");
    for f in ineffective {
        let m = f["message"].as_str().unwrap();
        assert!(m.contains("is not in effect"), "{m}");
        assert!(m.contains("Reason"), "{m}");
    }
}

/// A real `Reason` and an `Expires` make the finding waived.
#[test]
fn a_real_reason_and_expires_make_the_finding_waived() {
    let dir = fixture(
        "apply-fill",
        Setup {
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    let (_, report) = apply(&dir, &["--rule", "header.required-fields"]);
    let path = dir.join(&written_paths(&report)[0]);
    let text = std::fs::read_to_string(&path).unwrap();
    let text = text.replace(
        "Reason: Pending\n",
        "Reason: Owner field is not used in this repository\nExpires: 2099-01-01\n",
    );
    std::fs::write(&path, text).unwrap();
    stage(&dir);
    let (_, after) = check(&dir);
    assert!(
        unwaived(&after, "header.required-fields").is_empty(),
        "{after}"
    );
    assert!(
        after["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "header.required-fields" && f.get("waived").is_some()),
        "{after}"
    );
}

/// ADR-86: where `permanent` is declared, a draft given only a `Reason` is still ineffective.
#[test]
fn on_a_permanent_type_a_reason_alone_leaves_the_draft_ineffective() {
    let dir = fixture(
        "apply-permanent",
        Setup {
            waiver: "    effective_requires: [Reason]\n    permanent:\n      - rule: status.delivery-agreement\n        reason_field: Category\n        categories: [{name: docs-only}]\n",
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    let (_, report) = apply(&dir, &["--rule", "header.required-fields"]);
    let path = dir.join(&written_paths(&report)[0]);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace("Reason: Pending\n", "Reason: Reviewed by the maintainers\n"),
    )
    .unwrap();
    stage(&dir);
    let (code, after) = check(&dir);
    assert_eq!(code, 1, "{after}");
    assert_eq!(
        unwaived(&after, "header.required-fields").len(),
        1,
        "{after}"
    );
    let message = unwaived(&after, "waiver.matches-nothing")[0]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        message.contains("declares `permanent` without an entry for 'header.required-fields'"),
        "{message}"
    );
}

/// `resolved-by-drafts`: with the waiver directory missing, `type.dir-matches-nothing` is resolved
/// by the drafts, and after `--apply` it is really gone.
#[test]
fn a_finding_reported_resolved_is_gone_after_apply() {
    let dir = fixture(
        "apply-resolved",
        Setup {
            rules: "header.required-fields: error, type.dir-matches-nothing: warn",
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    std::fs::remove_dir_all(dir.join("docs/waiver")).unwrap();
    let (_, before) = check(&dir);
    assert_eq!(unwaived(&before, "type.dir-matches-nothing").len(), 1);
    let (_, report) = apply(&dir, &[]);
    assert_eq!(
        report["skipped"][0]["reason"], "resolved-by-drafts",
        "{report}"
    );
    stage(&dir);
    let (_, after) = check(&dir);
    assert!(
        unwaived(&after, "type.dir-matches-nothing").is_empty(),
        "{after}"
    );
}

/// A finding that disappears only because its own draft exists, with nothing else drafted, still
/// persists after `--apply`: it is not reported as resolved, and nothing is written.
#[test]
fn a_finding_resolved_only_by_its_own_draft_is_not_reported_as_resolved() {
    let dir = fixture(
        "apply-own-draft",
        Setup {
            rules: "type.dir-matches-nothing: warn",
            adrs: vec![("ADR-1-a.md", adr("Proposed", "2026-01-01", "a"))],
            ..Setup::default()
        },
    );
    std::fs::remove_dir_all(dir.join("docs/waiver")).unwrap();
    let (code, report) = apply(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["drafts"].as_array().unwrap().len(), 0, "{report}");
    let skipped = report["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{report}");
    assert_eq!(skipped[0]["reason"], "resolved-by-own-draft", "{report}");
    assert!(!dir.join("docs/waiver").exists());
    let (_, after) = check(&dir);
    assert_eq!(
        unwaived(&after, "type.dir-matches-nothing").len(),
        1,
        "{after}"
    );
}

/// A write error stops the run: the report lists what was written and what was not, exit 2.
#[test]
fn a_write_error_stops_the_run_and_lists_what_was_not_written() {
    let dir = fixture("apply-write-error", two_noisy());
    let outside = common::TempDir::new("urzua-waive-outside");
    std::fs::remove_dir_all(dir.join("docs/waiver")).unwrap();
    std::os::unix::fs::symlink(&*outside, dir.join("docs/waiver")).unwrap();
    let (code, report) = apply(&dir, &[]);
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["status"], "not-run");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("outside the repository"),
        "{report}"
    );
    let drafts = report["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 6, "{report}");
    assert!(drafts.iter().all(|d| d["written"] == false), "{report}");
    assert_eq!(std::fs::read_dir(&*outside).unwrap().count(), 0);
}

/// A draft quotes the findings it covers; a finding about a dangling citation
/// names the dangling id, so the quote must not be read as a citation itself
/// (the adopter switchover, item 2).
#[test]
fn applying_drafts_for_a_dangling_citation_adds_no_citation_finding() {
    let setup = Setup {
        rules: "header.required-fields: error, citation.resolution: error",
        adrs: vec![(
            "ADR-1-a.md",
            "---\nStatus: Proposed\nDate: 2026-01-01\nAuthor: a\nOwner: o\n---\n# 1 - X\n\nSee ADR-99 for why.\n"
                .to_string(),
        )],
        ..Setup::default()
    };
    let dir = fixture("apply-citation", setup);
    let (_, before) = check(&dir);
    let dangling = unwaived(&before, "citation.resolution");
    assert_eq!(dangling.len(), 1, "{before}");
    assert!(dangling[0]["message"].as_str().unwrap().contains("ADR-99"));

    let (code, report) = apply(&dir, &["--rule", "citation.resolution"]);
    assert_eq!(code, 0, "{report}");
    let paths = written_paths(&report);
    assert_eq!(paths.len(), 1, "{report}");
    stage(&dir);

    let (_, after) = check(&dir);
    let cites = unwaived(&after, "citation.resolution");
    assert_eq!(
        cites.len(),
        1,
        "the draft added a citation finding: {after}"
    );
    assert_eq!(cites[0]["file"], "docs/adr/ADR-1-a.md", "{after}");
    // The quoted finding is still in the draft, as fenced text.
    let text = std::fs::read_to_string(dir.join(&paths[0])).unwrap();
    assert!(text.contains("ADR-99"), "{text}");
    assert!(text.contains("\n## Findings\n\n```text\n"), "{text}");
}
