//! MILE-205 (ADR-91, SPEC-25): `urzua waive --from-report` previews one pending waiver per
//! rule and file for the current findings. This build writes nothing; each test plants a
//! corpus in a temporary git repository and reads the JSON the command prints.

mod common;

use common::waive::*;

/// Item: findings in two files and three rules give six drafts, numbered after the
/// highest existing waiver and fitted to the declared identity.
#[test]
fn two_files_and_three_rules_give_six_drafts_numbered_after_the_highest_waiver() {
    let mut setup = two_noisy();
    // A clean third record, and the highest existing waiver, which covers a real finding.
    setup
        .adrs
        .push(("ADR-3-c.md", adr("Proposed", "TBD", "ok")));
    setup.waivers.push((
        "WAIVER-0007-existing.md",
        waiver_file(
            "field.quality",
            "docs/adr/ADR-3-c.md",
            "Reviewed and accepted",
            "2099-01-01",
        ),
    ));
    let dir = fixture("six", setup);
    let (code, report) = waive(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["mode"], "dry-run");
    let drafts = report["drafts"].as_array().unwrap();
    let got: Vec<(String, String, String)> = drafts
        .iter()
        .map(|d| {
            (
                d["path"].as_str().unwrap().to_string(),
                str_of(d, "scope").to_string(),
                str_of(d, "rule").to_string(),
            )
        })
        .collect();
    assert_eq!(got.len(), 6, "{report}");
    // Ordered by file, then rule; numbered 0008.. past the existing 0007, padded to 4.
    let expected = [
        ("docs/adr/ADR-1-a.md", "field.pending"),
        ("docs/adr/ADR-1-a.md", "field.quality"),
        ("docs/adr/ADR-1-a.md", "header.required-fields"),
        ("docs/adr/ADR-2-b.md", "field.pending"),
        ("docs/adr/ADR-2-b.md", "field.quality"),
        ("docs/adr/ADR-2-b.md", "header.required-fields"),
    ];
    for (i, (path, scope, rule)) in got.iter().enumerate() {
        assert_eq!((scope.as_str(), rule.as_str()), expected[i], "{report}");
        assert!(
            path.starts_with(&format!("docs/waiver/WAIVER-{:04}-", 8 + i)),
            "{path}"
        );
    }
    for d in drafts {
        assert_eq!(d["written"], false);
        assert!(d["stable_id"].is_null());
        let count = d["findings"]["count"].as_u64().unwrap() as usize;
        assert_eq!(d["findings"]["lines"].as_array().unwrap().len(), count);
        assert!(count >= 1);
    }
    assert_eq!(report["skipped"].as_array().unwrap().len(), 0, "{report}");
    assert_eq!(report["unmatched_filters"].as_array().unwrap().len(), 0);
    untouched(&dir);
}

#[test]
fn several_findings_of_one_rule_in_one_file_share_a_draft_with_every_line() {
    let dir = fixture(
        "group",
        Setup {
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    let (_, report) = waive(&dir, &["--rule", "field.quality"]);
    let drafts = report["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 1, "{report}");
    let lines = drafts[0]["findings"]["lines"].as_array().unwrap();
    // `Date: TBD` is a placeholder and `Owner` is blank: two findings, one waiver.
    assert_eq!(lines.len(), 2, "{report}");
    assert!(lines[0]
        .as_str()
        .unwrap()
        .starts_with("docs/adr/ADR-1-a.md "));
}

#[test]
fn the_pending_value_is_the_declared_whole_value_else_pending() {
    let default = fixture("pend-default", two_noisy());
    let (_, report) = waive(&default, &[]);
    assert_eq!(report["pending_value"], "Pending");
    assert_eq!(report["pending_source"], "default");

    let declared = fixture(
        "pend-declared",
        Setup {
            waiver: "    effective_requires: [Reason, Expires]\n    statuses:\n      pending: [{pattern: '^later'}, Needs a reason]\n",
            ..two_noisy()
        },
    );
    let (code, report) = waive(&declared, &[]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["pending_value"], "Needs a reason");
    assert_eq!(report["pending_source"], "declared");
}

/// Declaring `permanent` is the other way past the permanence refusal.
#[test]
fn a_type_declaring_permanent_may_leave_expires_unrequired() {
    let dir = fixture(
        "permanent",
        Setup {
            waiver: "    effective_requires: [Reason]\n    permanent:\n      - rule: status.delivery-agreement\n        reason_field: Category\n        categories: [{name: docs-only}]\n",
            ..two_noisy()
        },
    );
    let (code, report) = waive(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["drafts"].as_array().unwrap().len(), 6);
}

fn refused(name: &str, waiver: &str, needle: &str) {
    let dir = fixture(
        name,
        Setup {
            waiver,
            ..two_noisy()
        },
    );
    let (code, report) = waive(&dir, &[]);
    assert_eq!(code, 2, "{report}");
    assert_eq!(report["status"], "not-run");
    let error = report["error"].as_str().unwrap();
    assert!(error.contains(needle), "{error}");
    untouched(&dir);
}

#[test]
fn it_refuses_when_reason_is_not_in_effective_requires() {
    refused(
        "no-reason",
        "    effective_requires: [Expires]\n",
        "does not list Reason in `effective_requires`",
    );
    refused(
        "no-requires",
        "",
        "does not list Reason in `effective_requires`",
    );
}

#[test]
fn it_refuses_when_pending_holds_only_patterns() {
    refused(
        "patterns",
        "    effective_requires: [Reason, Expires]\n    statuses:\n      pending: [{pattern: '^later'}]\n",
        "holds only patterns",
    );
}

#[test]
fn it_refuses_a_type_with_neither_permanent_nor_expires_required_and_names_both_fixes() {
    refused(
        "no-expiry",
        "    effective_requires: [Reason]\n",
        "declare `permanent` (ADR-86) or add Expires to `effective_requires`",
    );
}

#[test]
fn an_unknown_rule_exits_2_and_writes_nothing() {
    let dir = fixture("unknown-rule", two_noisy());
    let (code, report) = waive(&dir, &["--rule", "field.quality,no.such-rule"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("'no.such-rule' is not a rule this build ships"),
        "{report}"
    );
    untouched(&dir);
}

#[test]
fn from_report_is_required() {
    let dir = fixture("flag", two_noisy());
    let (code, stdout, stderr) = run(&dir, &["waive"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stdout.contains("--from-report"), "{stdout}");
}

fn skipped_of<'a>(report: &'a serde_json::Value, reason: &str) -> Vec<&'a serde_json::Value> {
    report["skipped"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["reason"] == reason)
        .collect()
}

/// A waiver with the same rule and scope names the finding already, whether or not it takes
/// effect; a pending Reason does not, so its finding is reported again and skipped here.
#[test]
fn a_finding_a_waiver_already_names_is_skipped_with_that_waiver() {
    let dir = fixture(
        "already",
        Setup {
            adrs: vec![("ADR-1-a.md", noisy())],
            waivers: vec![(
                "WAIVER-0001-q.md",
                waiver_file(
                    "field.quality",
                    "docs/adr/ADR-1-a.md",
                    "Pending",
                    "2099-01-01",
                ),
            )],
            ..Setup::default()
        },
    );
    let (code, report) = waive(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    let skipped = skipped_of(&report, "already-waived");
    assert_eq!(skipped.len(), 1, "{report}");
    assert_eq!(skipped[0]["rule"], "field.quality");
    assert_eq!(skipped[0]["file"], "docs/adr/ADR-1-a.md");
    assert_eq!(skipped[0]["waiver"], "docs/waiver/WAIVER-0001-q.md");
    let drafted: Vec<&str> = report["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap())
        .collect();
    assert!(!drafted.contains(&"field.quality"), "{report}");
}

#[test]
fn an_expired_waiver_for_the_same_rule_and_scope_is_skipped_as_expired() {
    let dir = fixture(
        "expired",
        Setup {
            adrs: vec![("ADR-1-a.md", noisy())],
            waivers: vec![(
                "WAIVER-0001-q.md",
                waiver_file(
                    "field.quality",
                    "docs/adr/ADR-1-a.md",
                    "Reviewed",
                    "2020-01-01",
                ),
            )],
            ..Setup::default()
        },
    );
    let (_, report) = waive(&dir, &[]);
    let skipped = skipped_of(&report, "expired");
    assert_eq!(skipped.len(), 1, "{report}");
    assert_eq!(skipped[0]["waiver"], "docs/waiver/WAIVER-0001-q.md");
    assert!(skipped_of(&report, "already-waived").is_empty());
}

/// A finding of `waiver.matches-nothing` is never waived: the waiver it reports is.
#[test]
fn a_waiver_matches_nothing_finding_is_skipped_not_drafted() {
    let dir = fixture(
        "matches-nothing",
        Setup {
            adrs: vec![("ADR-1-a.md", adr("Proposed", "2026-01-01", "a"))],
            waivers: vec![(
                "WAIVER-0001-q.md",
                waiver_file(
                    "field.quality",
                    "docs/adr/ADR-9-gone.md",
                    "Reviewed",
                    "2099-01-01",
                ),
            )],
            ..Setup::default()
        },
    );
    let (_, report) = waive(&dir, &[]);
    let skipped = skipped_of(&report, "matches-nothing");
    assert_eq!(skipped.len(), 1, "{report}");
    assert_eq!(skipped[0]["rule"], "waiver.matches-nothing");
    assert_eq!(skipped[0]["file"], "docs/waiver/WAIVER-0001-q.md");
    let drafted: Vec<&str> = report["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap())
        .collect();
    assert!(!drafted.contains(&"waiver.matches-nothing"), "{report}");
}

/// ADR-91 amendment of 2026-11-08: with no waiver directory yet, `type.dir-matches-nothing`
/// exists only because no waiver record does. Writing a draft resolves it, so it is not drafted.
#[test]
fn a_finding_the_drafts_themselves_resolve_is_skipped_and_listed() {
    let dir = fixture(
        "resolved",
        Setup {
            rules: "header.required-fields: error, type.dir-matches-nothing: warn",
            adrs: vec![("ADR-1-a.md", noisy())],
            ..Setup::default()
        },
    );
    std::fs::remove_dir_all(dir.join("docs/waiver")).unwrap();
    let (code, report) = waive(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    let skipped = skipped_of(&report, "resolved-by-drafts");
    assert_eq!(skipped.len(), 1, "{report}");
    assert_eq!(skipped[0]["rule"], "type.dir-matches-nothing");
    let drafted: Vec<&str> = report["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap())
        .collect();
    assert_eq!(drafted, ["header.required-fields"], "{report}");
    // The drafts' own findings (a pending Reason) do not hide the original ones.
    assert!(!dir.join("docs/waiver").exists());
}

/// Info findings and findings below `fail_on` are neither drafted nor listed.
#[test]
fn info_and_below_fail_on_findings_are_not_drafted_and_not_listed() {
    let info = fixture(
        "info",
        Setup {
            rules: "header.required-fields: error, field.quality: info, field.pending: info",
            ..two_noisy()
        },
    );
    let (_, report) = waive(&info, &[]);
    let rules: std::collections::BTreeSet<&str> = report["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap())
        .collect();
    assert_eq!(rules, ["header.required-fields"].into(), "{report}");
    assert_eq!(report["skipped"].as_array().unwrap().len(), 0, "{report}");

    // Under `fail_on: error` a warning is below the gate too.
    let warn = fixture(
        "below",
        Setup {
            fail_on: "error",
            ..two_noisy()
        },
    );
    let (_, report) = waive(&warn, &[]);
    let rules: std::collections::BTreeSet<&str> = report["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["rule"].as_str().unwrap())
        .collect();
    assert!(!rules.contains("field.pending"), "{report}");
    assert!(rules.contains("field.quality"), "{report}");
}

#[test]
fn rule_and_path_filters_narrow_and_unmatched_filters_are_listed() {
    let mut setup = two_noisy();
    setup.adrs.push(("ADR-3-c.md", noisy()));
    let dir = fixture("filters", setup);

    let (_, by_rule) = waive(&dir, &["--rule", "field.pending"]);
    let drafts = by_rule["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 3, "{by_rule}");
    assert!(drafts.iter().all(|d| d["rule"] == "field.pending"));

    let (_, by_file) = waive(
        &dir,
        &[
            "--path",
            "docs/adr/ADR-2-b.md",
            "--path",
            "./docs/adr/ADR-3-c.md",
        ],
    );
    let scopes: std::collections::BTreeSet<&str> = by_file["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["scope"].as_str().unwrap())
        .collect();
    assert_eq!(
        scopes,
        ["docs/adr/ADR-2-b.md", "docs/adr/ADR-3-c.md"].into()
    );

    let (_, by_dir) = waive(&dir, &["--path", "docs/adr/", "--rule", "field.quality"]);
    assert_eq!(by_dir["drafts"].as_array().unwrap().len(), 3, "{by_dir}");
    assert_eq!(by_dir["unmatched_filters"].as_array().unwrap().len(), 0);

    let (code, none) = waive(
        &dir,
        &[
            "--path",
            "docs/none",
            "--path",
            "docs/adr/ADR-1-a.md",
            "--rule",
            "status.delivery-agreement",
        ],
    );
    assert_eq!(code, 0, "{none}");
    assert_eq!(none["drafts"].as_array().unwrap().len(), 0, "{none}");
    let unmatched: Vec<(String, String)> = none["unmatched_filters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| {
            (
                str_of(u, "kind").to_string(),
                str_of(u, "value").to_string(),
            )
        })
        .collect();
    assert_eq!(
        unmatched,
        [
            ("rule".to_string(), "status.delivery-agreement".to_string()),
            ("path".to_string(), "docs/none".to_string())
        ]
    );
    untouched(&dir);
}

/// A directory prefix is by whole component, and a clean repository drafts nothing.
#[test]
fn a_clean_corpus_drafts_nothing_and_still_exits_0() {
    let dir = fixture(
        "clean",
        Setup {
            adrs: vec![(
                "ADR-1-a.md",
                "---\nStatus: Proposed\nDate: 2026-01-01\nAuthor: a\nOwner: b\n---\n# 1 - X\n"
                    .to_string(),
            )],
            ..Setup::default()
        },
    );
    let (code, report) = waive(&dir, &[]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["drafts"].as_array().unwrap().len(), 0);
    assert_eq!(report["status"], "ok");
}
