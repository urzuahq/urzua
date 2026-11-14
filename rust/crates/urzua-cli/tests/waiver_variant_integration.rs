//! A waiver may name the finding variant and the configuration key path it
//! excuses (`SPEC-10`, maintainer decision of 2026-11-14): scoped to the
//! configuration file it must, since it would otherwise silence every finding of
//! its rule there, present and future. `waive --from-report` drafts carry them
//! (`SPEC-25`). Real binary, real git, JSON on stdout.
//!
//! Verifies: SPEC-10, SPEC-25

mod common;

use std::path::Path;

use common::waive::{adr, run};

const RULE: &str = "type.dir-matches-nothing";
const CONFIG: &str = ".urzua/config.yaml";

/// Three findings of one rule in the config: the `ghost` and `phantom` directories
/// are missing (variant `dir-missing`, one per type) and `adr` declares an
/// `extra_dirs` entry that is not there (variant `extra-dir-missing`).
fn config(waiver_known: &str) -> String {
    format!(
        "schema_version: 2\nfail_on: warning\nrules: {{type.dir-matches-nothing: warn, waiver.matches-nothing: warn}}\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Date, Author, Owner]\n    extra_dirs: [docs/old-adr]\n  ghost:\n    dir: docs/ghost\n    prefix: GHOST\n    header_shape: yaml-frontmatter\n    required_fields: []\n  phantom:\n    dir: docs/phantom\n    prefix: PHANTOM\n    header_shape: yaml-frontmatter\n    required_fields: []\n  waiver:\n    dir: docs/waiver\n    prefix: WAIVER\n    role: suppression\n    header_shape: yaml-frontmatter\n    identity: {{from: filename, pattern: '^(?P<prefix>WAIVER)-(?P<number>\\d{{4}})-(?P<slug>.+)$'}}\n    required_fields: [Rule, Scope, Reason]\n    known_fields: [{waiver_known}]\n    effective_requires: [Reason, Expires]\n    revision_log:\n      marker: \"**Revision log**\"\n      required: true\n      classes: [substantive, structural]\n      class_column: Class\n      date_column: Date\n      tool_class: structural\n"
    )
}

fn waiver(n: u32, extra: &str) -> (String, String) {
    (
        format!("WAIVER-{n:04}-x.md"),
        format!(
            "---\nStable-Id: 01M7WAIVERAAAAAAAAAAAAAAA{n}\nRule: {RULE}\nScope: {CONFIG}\nReason: The directory is created by the next epic\nExpires: 2099-01-01\n{extra}---\n# WAIVER-{n:04} -- X\n\n## What this covers\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-11-01 | Filed. | **substantive** |\n"
        ),
    )
}

fn repo(label: &str, waiver_known: &str, waivers: &[(String, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-waiver-variant-{label}"));
    for d in ["docs/adr", "docs/waiver", ".urzua/templates"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(CONFIG), config(waiver_known)).unwrap();
    std::fs::write(
        dir.join(".urzua/templates/waiver.md"),
        "# NNNN -- Title\n\n> Rule: the rule\n> Scope: a path\n> Reason: why\n> Expires: YYYY-MM-DD\n\n## What this covers\n\nThe finding.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        adr("Accepted", "2026-01-01", "a"),
    )
    .unwrap();
    for (name, content) in waivers {
        std::fs::write(dir.join("docs/waiver").join(name), content).unwrap();
    }
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn check(dir: &Path) -> serde_json::Value {
    let (_, stdout, stderr) = run(dir, &["check"]);
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout} {stderr}"))
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .collect()
}

fn key_path_of(finding: &serde_json::Value) -> String {
    finding["suggested_action"]["target"]["key_path"]
        .as_array()
        .map(|k| {
            k.iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default()
}

const ADMITTING: &str = "Expires, Variant, Key-path";

/// The waiver excuses the finding it names and no other of the rule in the file.
#[test]
fn a_waiver_naming_a_variant_and_key_path_excuses_that_finding_alone() {
    let w = waiver(
        1,
        "Variant: dir-missing\nKey-path: [record_types, ghost, dir]\n",
    );
    let dir = repo("narrow", ADMITTING, &[w]);
    let report = check(&dir);
    let found = findings(&report, RULE);
    assert_eq!(found.len(), 3, "{report}");
    for f in &found {
        let key = key_path_of(f);
        if key == "record_types.ghost.dir" {
            assert!(f["waived"].as_str().unwrap().contains("WAIVER-0001"), "{f}");
        } else {
            assert!(f.get("waived").is_none(), "{key} must not be hidden: {f}");
        }
    }
    // The waiver matches its finding and is narrowed: nothing to report on it.
    assert!(findings(&report, "waiver.matches-nothing").is_empty());
}

/// A variant alone is narrower than the rule and wider than a key path.
#[test]
fn a_waiver_naming_only_a_variant_covers_that_variant_everywhere_in_the_file() {
    let dir = repo(
        "variant-only",
        ADMITTING,
        &[waiver(1, "Variant: dir-missing\n")],
    );
    let report = check(&dir);
    let mut waived: Vec<String> = findings(&report, RULE)
        .iter()
        .filter(|f| f.get("waived").is_some())
        .map(|f| key_path_of(f))
        .collect();
    waived.sort();
    assert_eq!(
        waived,
        ["record_types.ghost.dir", "record_types.phantom.dir"],
        "{report}"
    );
}

/// Existing waivers keep working, and a configuration-scoped one that names
/// neither field is a finding that says how to narrow it.
#[test]
fn a_config_scoped_waiver_naming_neither_still_excuses_all_and_is_reported() {
    let dir = repo("broad", ADMITTING, &[waiver(1, "")]);
    let report = check(&dir);
    assert!(
        findings(&report, RULE)
            .iter()
            .all(|f| f.get("waived").is_some()),
        "an existing waiver keeps covering every finding: {report}"
    );
    let on_waiver = findings(&report, "waiver.matches-nothing");
    assert_eq!(on_waiver.len(), 1, "{report}");
    let message = on_waiver[0]["message"].as_str().unwrap();
    assert!(
        message.contains("Variant")
            && message.contains("Key-path")
            && message.contains("every finding"),
        "{message}"
    );
    assert_eq!(
        on_waiver[0]["suggested_action"]["target"]["field"], "Variant",
        "{}",
        on_waiver[0]
    );
    assert_eq!(on_waiver[0]["file"], "docs/waiver/WAIVER-0001-x.md");
}

/// A waiver scoped to a record, not the configuration, is not asked to narrow.
#[test]
fn a_waiver_scoped_to_a_record_needs_no_variant() {
    let (name, content) = waiver(1, "");
    let content = content
        .replace(&format!("Rule: {RULE}"), "Rule: header.required-fields")
        .replace(&format!("Scope: {CONFIG}"), "Scope: docs/adr/ADR-1-x.md");
    let dir = repo("record-scope", ADMITTING, &[(name, content)]);
    let report = check(&dir);
    assert!(
        findings(&report, "waiver.matches-nothing")
            .iter()
            .all(|f| !f["message"].as_str().unwrap().contains("every finding")),
        "{report}"
    );
}

fn waive(dir: &Path, extra: &[&str]) -> (i32, serde_json::Value) {
    common::waive::waive(dir, extra)
}

/// `waive --from-report` drafts one waiver per rule, file, variant and key path,
/// each carrying `Variant` and `Key-path` when the type admits them.
#[test]
fn drafts_name_the_variant_and_key_path_and_split_by_them() {
    let dir = repo("drafts", ADMITTING, &[]);
    let (code, report) = waive(&dir, &["--rule", RULE]);
    assert_eq!(code, 0, "{report}");
    let drafts = report["drafts"].as_array().unwrap();
    let got: Vec<(String, String)> = drafts
        .iter()
        .map(|d| {
            (
                d["variant"].as_str().unwrap().to_string(),
                d["key_path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| k.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join("."),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (
                "dir-missing".to_string(),
                "record_types.ghost.dir".to_string()
            ),
            (
                "dir-missing".to_string(),
                "record_types.phantom.dir".to_string()
            ),
            (
                "extra-dir-missing".to_string(),
                "record_types.adr.extra_dirs".to_string()
            ),
        ],
        "{report}"
    );

    let (code, applied) = waive(&dir, &["--rule", RULE, "--apply"]);
    assert_eq!(code, 0, "{applied}");
    let path = applied["drafts"][0]["path"].as_str().unwrap();
    let text = std::fs::read_to_string(dir.join(path)).unwrap();
    assert!(text.contains("Variant: dir-missing"), "{text}");
    assert!(
        text.contains("Key-path: record_types, ghost, dir"),
        "{text}"
    );
    // The two drafts of one rule and file are titled apart.
    let titles: std::collections::HashSet<&str> = applied["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["path"].as_str().unwrap())
        .collect();
    assert_eq!(titles.len(), 3);

    // The drafts name their findings, so a second run drafts nothing new.
    common::git(&dir, &["add", "-A"]);
    let (_, again) = waive(&dir, &["--rule", RULE]);
    assert_eq!(again["drafts"].as_array().unwrap().len(), 0, "{again}");
    let skipped = again["skipped"].as_array().unwrap();
    assert!(
        skipped
            .iter()
            .filter(|s| s["reason"] == "already-waived")
            .count()
            >= 3,
        "{again}"
    );
}

/// A type with a closed field set that lists neither field drafts as before: one
/// per rule and file, no `Variant`, and says so on stderr.
#[test]
fn a_type_that_admits_neither_field_drafts_one_per_rule_and_file() {
    let dir = repo("closed", "Expires", &[]);
    let (code, report) = waive(&dir, &["--rule", RULE]);
    assert_eq!(code, 0, "{report}");
    let drafts = report["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 1, "{report}");
    assert!(drafts[0].get("variant").is_none() && drafts[0].get("key_path").is_none());
    assert_eq!(drafts[0]["findings"]["count"], 3);
}

/// This repository's own `WAIVER-0052` was written for the milestone `WontDo`
/// exemption finding and hid every other `type.dir-matches-nothing` finding in
/// the config, a misdeclared `condition` directory included. It names its finding.
#[test]
fn the_own_wontdo_waiver_does_not_hide_a_misdeclared_directory() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let own = std::fs::read_to_string(root.join(".urzua/config.yaml")).unwrap();
    let anchor = "dir: \"docs/conditions\"";
    assert_eq!(own.matches(anchor).count(), 1, "the anchor line moved");
    let waiver_record = std::fs::read_dir(root.join("docs/waiver"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("WAIVER-0052-")
        })
        .expect("WAIVER-0052 exists");

    let dir = common::TempDir::new("urzua-waiver-variant-own");
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    for d in ["docs/waiver", "docs/milestones", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(
        dir.join(CONFIG),
        own.replace(anchor, "dir: \"docs/conditionz\""),
    )
    .unwrap();
    std::fs::copy(
        &waiver_record,
        dir.join("docs/waiver")
            .join(waiver_record.file_name().unwrap()),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/milestones/MILE-1-x.md"),
        "---\nStatus: Open\n---\n# 1\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);

    let (_, stdout, stderr) = run(
        &dir,
        &[
            "--as-of",
            common::OWN_CORPUS_AS_OF,
            "check",
            "--rules",
            RULE,
        ],
    );
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {stdout} {stderr}"));
    let found = findings(&report, RULE);
    let by_key = |key: &str| {
        found
            .iter()
            .find(|f| key_path_of(f) == key)
            .unwrap_or_else(|| panic!("no finding at {key}: {report}"))
    };
    assert!(
        by_key("record_types.milestone.sections.exempt_statuses")["waived"]
            .as_str()
            .unwrap()
            .contains("WAIVER-0052"),
        "{report}"
    );
    assert!(
        by_key("record_types.condition.dir").get("waived").is_none(),
        "WAIVER-0052 must not hide the condition directory finding: {report}"
    );
}
