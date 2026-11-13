//! This repository's own `.urzua/config.yaml`, exercised end to end: the
//! declarations it ships are what these tests run, not a fixture's.
//! BUG-123 closes here; BUG-65 and the ADR-84 bug evidence are covered end to end.

mod common;

use std::path::Path;
use std::process::Command;

fn own_config() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    std::fs::read_to_string(path).expect("repository config readable")
}

/// MILE-80: the repository's config declares a level for every rule the build
/// ships, including the ones it declares `off`, so none is merely absent.
#[test]
fn own_config_declares_a_level_for_every_shipped_rule() {
    let config = urzua_core::config::parse(&own_config()).expect("repository config parses");
    let undeclared: Vec<&str> = urzua_core::rules::ALL_RULES
        .iter()
        .copied()
        .filter(|id| !config.rules.contains_key(*id))
        .collect();
    assert!(
        undeclared.is_empty(),
        "rules with no declared level in .urzua/config.yaml: {undeclared:?}"
    );
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run")
        .success());
}

/// A fresh git repo holding the repository's own config and `files`.
fn repo(name: &str, files: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-own-config-{name}"));
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    for ty_dir in ["adr", "rfc", "specs", "bugs", "milestones", "waiver"] {
        std::fs::create_dir_all(dir.join("docs").join(ty_dir)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), own_config()).unwrap();
    // The own config declares `sections.order: {from: template}`, which reads
    // each type's template, so the fixture carries them.
    let templates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/templates");
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    for entry in std::fs::read_dir(templates).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(
            entry.path(),
            dir.join(".urzua/templates").join(entry.file_name()),
        )
        .unwrap();
    }
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

/// The H1 of the one file `urzua new <ty>` wrote under `dir_name`.
fn new_title(dir: &Path, ty: &str, dir_name: &str) -> String {
    let out = urzua(dir, &["new", ty, "A fresh record", "--by", "tester"]);
    assert!(
        out.status.success(),
        "{ty}: {} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_dir(dir.join(dir_name))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().contains("a-fresh-record"))
        .unwrap_or_else(|| panic!("{ty}: no file written under {dir_name}"));
    std::fs::read_to_string(written)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("# "))
        .expect("an H1")
        .to_string()
}

/// BUG-123, observed failing: `new` opened every title with the bare number,
/// while every spec in the corpus carries `SPEC-N`. The spec type now declares
/// its title form through its identity pattern; the other types keep the
/// bare number.
#[test]
fn new_opens_a_spec_title_with_its_declared_prefix_and_other_types_with_the_bare_number() {
    let dir = repo("h1", &[]);
    assert!(
        new_title(&dir, "spec", "docs/specs").starts_with("# SPEC-1 "),
        "spec"
    );
    assert!(
        new_title(&dir, "adr", "docs/adr").starts_with("# 1 "),
        "adr"
    );
    assert!(
        new_title(&dir, "rfc", "docs/rfc").starts_with("# 1 "),
        "rfc"
    );
    assert!(
        new_title(&dir, "bug", "docs/bugs").starts_with("# 1 "),
        "bug"
    );
    assert!(
        new_title(&dir, "milestone", "docs/milestones").starts_with("# 1 "),
        "milestone"
    );
}

/// MILE-74, observed failing: `urzua new spec` wrote `Version` and `Subject`
/// blank, so the record it made failed `check` before anyone touched it. The spec
/// type now declares the field `new` fills with the title and the version a
/// record starts at, and the scaffold passes `check` as written.
#[test]
fn a_spec_scaffolded_by_new_passes_check_without_an_edit_observed_failing() {
    let template = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/templates/spec.md"),
    )
    .expect("the spec template is readable");
    let dir = repo("spec-scaffold", &[(".urzua/templates/spec.md", &template)]);
    let out = urzua(&dir, &["new", "spec", "A fresh record", "--by", "tester"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_dir(dir.join("docs/specs"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().contains("a-fresh-record"))
        .expect("a spec file");
    let content = std::fs::read_to_string(&written).unwrap();
    assert!(content.contains("\nSubject: A fresh record\n"), "{content}");
    assert!(content.contains("\nVersion: '0.1'\n"), "{content}");

    let out = urzua(&dir, &["check", "docs/"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON on stdout");
    let own: Vec<String> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["file"].as_str().unwrap().contains("a-fresh-record"))
        .map(|f| format!("{} {}", f["rule"], f["message"]))
        .collect();
    assert!(
        own.is_empty(),
        "the scaffold must pass check as written: {own:?}"
    );
}

fn bug(status: &str, found_in: &str, extra: &str) -> String {
    format!(
        "---\nStable-Id: 01M6JS58VTV16JK13MJWQZTPG7\nStatus: {status}\nFound-in: {found_in}\n{extra}---\n# 1 — A bug\n\n## What was wrong\n\nx\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n"
    )
}

fn finding_rules(dir: &Path) -> Vec<String> {
    let out = urzua(dir, &["check", "docs/"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON on stdout");
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["file"].as_str().unwrap().contains("BUG-1-"))
        .map(|f| f["rule"].as_str().unwrap().to_string())
        .collect()
}

/// `ADR-84`: a bug's `Status` follows its `Realized-by`. An `Open` bug whose
/// evidence names a test is a finding `urzua fix` repairs; a `Fixed` one with a
/// test is not, and neither is an `Open` one with none. A `Fixed` bug with no
/// evidence at all is reported too: a done record with no evidence is a finding
/// (a bug that legitimately has none is waived). A bug carrying the retired
/// `Regression-test` field is reported as an unknown field.
#[test]
fn a_bug_open_with_test_evidence_is_a_finding_and_the_other_shapes_are_not() {
    let files = |status: &str, extra: &str| {
        repo(
            "bug-evidence",
            &[
                ("docs/bugs/BUG-1-a-bug.md", &bug(status, "a review", extra)),
                ("rust/t.rs", "#[test]\nfn a_real_test() {}\n"),
            ],
        )
    };
    let by_scope = "Realized-by: test:rust/t.rs\n";
    let open = files("Open", by_scope);
    assert_eq!(
        finding_rules(&open),
        vec!["status.delivery-agreement".to_string()]
    );
    for (status, extra) in [("Fixed", by_scope), ("Open", "")] {
        let dir = files(status, extra);
        let rules = finding_rules(&dir);
        assert!(rules.is_empty(), "{status} {extra:?}: {rules:?}");
    }
    let unevidenced = files("Fixed", "");
    assert_eq!(
        finding_rules(&unevidenced),
        vec!["status.delivery-agreement".to_string()]
    );
    let retired = files("Fixed", "Regression-test: rust/t.rs::a_real_test\n");
    assert!(
        !finding_rules(&retired).is_empty(),
        "the retired field is no longer a known field of the bug type"
    );
}

/// BUG-65: the corpus's commonest way of saying pending is read as pending,
/// and a value that merely mentions the word is a real one.
#[test]
fn the_declared_pending_vocabulary_reads_whole_values_only() {
    let dir = repo(
        "mentions-pending",
        &[(
            "docs/bugs/BUG-1-a-bug.md",
            &bug("Open", "'a pending review of the release'", ""),
        )],
    );
    let rules = finding_rules(&dir);
    assert!(rules.is_empty(), "{rules:?}");
    let whole = repo(
        "whole-pending",
        &[("docs/bugs/BUG-1-a-bug.md", &bug("Open", "pending", ""))],
    );
    assert!(
        finding_rules(&whole).contains(&"field.pending".to_string()),
        "{:?}",
        finding_rules(&whole)
    );
}

/// ADR-74 / MILE-185: this repository's rust resolver addresses the struct
/// fields ADR-39, ADR-43 and ADR-44 pin, by name, so a new option elsewhere in
/// `RecordTypeConfig` cannot stale them.
#[test]
fn the_own_rust_resolver_addresses_each_pinned_record_type_config_field() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    for field in [
        "required_fields",
        "known_fields",
        "spec",
        "pointer_fields",
        "narrative_fields",
    ] {
        let out = urzua(
            &root,
            &[
                "explain",
                &format!("rust/crates/urzua-core/src/config.rs#RecordTypeConfig::{field}"),
            ],
        );
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|e| panic!("{field}: not JSON ({e}): {out:?}"));
        let text = parsed["scope"]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{field}: {parsed}"));
        let last = text.lines().last().unwrap();
        assert!(
            last.trim_start().starts_with(&format!("pub {field}:")),
            "{field}: the span ends at the declaration: {text}"
        );
        assert!(
            !text.contains("pub fn"),
            "{field}: the span is one field, not a neighbour: {text}"
        );
    }
}

/// MILE-206, ADR-92: this repository governs the records it has written and not
/// yet staged. A draft planted in `docs/` is examined by a plain `check` and is
/// added under `--base`.
#[test]
fn the_own_config_includes_untracked_records_and_a_draft_is_examined_and_added() {
    let dir = repo("untracked", &[]);
    std::fs::write(
        dir.join("docs/bugs/BUG-1-a-draft.md"),
        "---\nStatus: Open\n---\n# 1 - A draft\n",
    )
    .unwrap();
    let out = urzua(&dir, &["check"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["scope"]["include_untracked"], true, "{v}");
    assert_eq!(v["scope"]["untracked_included"], 1, "{v}");
    assert_eq!(v["files_examined"], 1, "{v}");

    let out = urzua(&dir, &["check", "--base", "HEAD"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let changed: Vec<&str> = v["scope"]["changed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(changed, vec!["docs/bugs/BUG-1-a-draft.md"], "{v}");
}

/// MILE-193: the waiver template names `Justified-by`, so a waiver filed with
/// `urzua new` carries the field a `cite: required` category needs. Observed
/// failing before the template line was added: the scaffold stopped at
/// `Category`.
#[test]
fn new_waiver_scaffolds_justified_by_from_the_repositorys_own_template() {
    let dir = repo("waiver-template", &[]);
    let template =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/templates/waiver.md");
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    std::fs::copy(template, dir.join(".urzua/templates/waiver.md")).unwrap();
    let out = urzua(&dir, &["new", "waiver", "A fresh record", "--by", "tester"]);
    assert!(
        out.status.success(),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read_dir(dir.join("docs/waiver"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().contains("a-fresh-record"))
        .expect("a waiver was written");
    let text = std::fs::read_to_string(written).unwrap();
    let header: Vec<&str> = text.lines().skip(1).take_while(|l| *l != "---").collect();
    for field in ["Category", "Justified-by"] {
        assert!(
            header.iter().any(|l| l.starts_with(&format!("{field}:"))),
            "the header names {field}: {header:?}"
        );
    }
}

/// `MILE-108`: this repository declares `code.dangling-bug-reference` over
/// `rust/` and `scripts/`. A comment citing a record the corpus holds passes, a
/// public standard number it lists as external passes, a fixture line carrying
/// its declared marker passes, and a dangling one is reported; a file with an
/// undeclared extension is not read.
#[test]
fn the_own_config_reports_a_dangling_citation_in_rust_and_scripts_and_nothing_else() {
    let bug1 = bug("Fixed", "a review", "Realized-by: test:rust/t.rs\n");
    let dir = repo(
        "dangling-reference",
        &[
            ("docs/bugs/BUG-1-a-bug.md", &bug1),
            (
                "rust/t.rs",
                concat!(
                    "// BUG-1 is live\n// RFC-4122 is a standard\n// BUG-9999 is gone\n",
                    // The declared marker, split so this file does not carry it on a line.
                    "// BUG-9998 citation-",
                    "fixture\n"
                ),
            ),
            ("scripts/a.sh", "# BUG-9997 is gone too\n"),
            ("scripts/notes.txt", "# BUG-9996 is not read\n"),
        ],
    );
    let out = urzua(&dir, &["check"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).expect("JSON on stdout");
    let mut found: Vec<(String, u64)> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "code.dangling-bug-reference")
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_string(),
                f["line"].as_u64().unwrap(),
            )
        })
        .collect();
    found.sort();
    assert_eq!(
        found,
        vec![
            ("rust/t.rs".to_string(), 3),
            ("scripts/a.sh".to_string(), 1)
        ],
        "{report}"
    );
    let execution = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == "code.dangling-bug-reference")
        .unwrap();
    assert_eq!(execution["population"]["eligible"], 2, "{execution}");
}

/// MILE-49: this repository does not declare `drift_baseline` (it would hold
/// out nothing here; fixtures exercise it), but its configuration's shape
/// accepts the option on `embodiment.consistency`.
#[test]
fn own_config_shape_accepts_a_drift_baseline_on_consistency() {
    let own = own_config();
    assert!(
        !own.contains("drift_baseline"),
        "not declared in the own config"
    );
    let anchor = "    drift_audit_field: \"Last-audited\"\n";
    assert_eq!(own.matches(anchor).count(), 1, "the anchor line moved");
    let declared = own.replace(
        anchor,
        &format!("{anchor}    drift_baseline: {{ref: v0.3.0}}\n"),
    );
    let config = urzua_core::config::parse(&declared).expect("config with a baseline parses");
    let baseline = config.rules["embodiment.consistency"]
        .drift_baseline
        .as_ref()
        .expect("drift_baseline is read");
    assert_eq!(baseline.git_ref, "v0.3.0");
}
