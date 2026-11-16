//! `MILE-233`: the words rules compare against are read from configuration. A
//! repository that declares its own vocabulary gets findings by its own words
//! and none by the defaults; a declaration that cannot work is a load error
//! naming its key. Each case runs the real binary over a scratch repository.
//!
//! Verifies: MILE-233

mod common;

use std::path::Path;

fn repo(label: &str, config: &str, files: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-vocab-{label}"));
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::create_dir_all(dir.path().join(".urzua")).unwrap();
    std::fs::write(dir.path().join(".urzua/config.yaml"), config).unwrap();
    for (path, text) in files {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, text).unwrap();
    }
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn findings(dir: &Path, rule: &str) -> Vec<serde_json::Value> {
    let (_, out, err) = run(dir, &["check"]);
    let parsed: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("not JSON ({e}): {out} / {err}"));
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .cloned()
        .collect()
}

const ADR_TYPE: &str = "record_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\n";

fn back_pointer_config(options: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [src]\n{options}{ADR_TYPE}"
    )
}

#[test]
fn a_declared_back_pointer_marker_and_leader_are_read_and_the_defaults_are_not() {
    let options = "    implements_marker: 'Realises:'\n    comment_leaders: ['%']\n    trailing_comment_leaders: ['%']\n";
    let dir = repo(
        "marker",
        &back_pointer_config(options),
        &[
            ("docs/adr/ADR-1-x.md", "---\nStatus: Accepted\n---\n# 1 — X\n"),
            (
                "src/a.txt",
                "% Realises: ADR-9\n// Implements: ADR-8\n% Implements: ADR-7\nx = 1 % Realises: ADR-6\n",
            ),
        ],
    );
    let found = findings(dir.path(), "embodiment.back-pointer-resolution");
    let cited: Vec<&str> = found
        .iter()
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(cited.iter().any(|m| m.contains("ADR-9")), "{cited:?}");
    assert!(cited.iter().any(|m| m.contains("ADR-6")), "{cited:?}");
}

#[test]
fn without_the_new_keys_the_default_marker_is_read_as_before() {
    let dir = repo(
        "marker-default",
        &back_pointer_config(""),
        &[
            (
                "docs/adr/ADR-1-x.md",
                "---\nStatus: Accepted\n---\n# 1 — X\n",
            ),
            ("src/a.rs", "// Implements: ADR-8\n% Realises: ADR-9\n"),
        ],
    );
    let found = findings(dir.path(), "embodiment.back-pointer-resolution");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0]["message"].as_str().unwrap().contains("ADR-8"));
}

fn placeholder_config(placeholders: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  field.quality: error\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status, Owner]\n    header_shape: yaml-frontmatter\n    statuses:\n      values: [Accepted]\n{placeholders}"
    )
}

#[test]
fn declared_placeholder_words_replace_the_compiled_in_ones() {
    let records = [
        (
            "docs/adr/ADR-1-a.md",
            "---\nStatus: Accepted\nOwner: ausstehend\n---\n# 1 — A\n",
        ),
        (
            "docs/adr/ADR-2-b.md",
            "---\nStatus: Accepted\nOwner: TBD\n---\n# 2 — B\n",
        ),
    ];
    let declared = repo(
        "placeholders",
        &placeholder_config("      placeholders: [ausstehend, '(offen)']\n"),
        &records,
    );
    let found = findings(declared.path(), "field.quality");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-1-a.md");

    let default = repo("placeholders-default", &placeholder_config(""), &records);
    let found = findings(default.path(), "field.quality");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-2-b.md");
}

fn claim_config(options: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  claim.status-agreement:\n    level: error\n    claim_paths: [.changeset]\n    closed_statuses: [Fixed]\n{options}record_types:\n  bug:\n    dir: docs/bugs\n    prefix: BUG\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\n    statuses:\n      values: [Open, Fixed]\n      success: [Fixed]\n"
    )
}

#[test]
fn declared_closing_verbs_replace_the_defaults() {
    let files = [
        ("docs/bugs/BUG-1-a.md", "---\nStatus: Open\n---\n# 1 — A\n"),
        ("docs/bugs/BUG-2-b.md", "---\nStatus: Open\n---\n# 2 — B\n"),
        (".changeset/x.md", "Behebt BUG-1.\n\nAlso fixes BUG-2.\n"),
    ];
    let declared = repo(
        "verbs",
        &claim_config("    closing_verbs: [behebt, schließt]\n"),
        &files,
    );
    let found = findings(declared.path(), "claim.status-agreement");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0]["message"].as_str().unwrap().contains("BUG-1"));

    let default = repo("verbs-default", &claim_config(""), &files);
    let found = findings(default.path(), "claim.status-agreement");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0]["message"].as_str().unwrap().contains("BUG-2"));
}

fn y_config(phrases: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  section.shape: error\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\n    sections:\n      from: headings\n      required:\n        - {{name: Decision, shape: y-statement{phrases}}}\n"
    )
}

const GERMAN: &str = "---\nStatus: Accepted\n---\n# 1 — A\n\n## Decision\n\nIm Kontext von X haben wir Y entschieden.\n";
const ENGLISH: &str = "---\nStatus: Accepted\n---\n# 2 — B\n\n## Decision\n\nIn the context of X, facing Y, we decided Z to achieve W, accepting V.\n";

#[test]
fn declared_y_statement_phrases_replace_the_five_defaults() {
    let files = [
        ("docs/adr/ADR-1-a.md", GERMAN),
        ("docs/adr/ADR-2-b.md", ENGLISH),
    ];
    let declared = repo(
        "phrases",
        &y_config(", phrases: ['im kontext von', 'haben wir']"),
        &files,
    );
    let found = findings(declared.path(), "section.shape");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-2-b.md");

    let default = repo("phrases-default", &y_config(""), &files);
    let found = findings(default.path(), "section.shape");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["file"], "docs/adr/ADR-1-a.md");
}

fn waiver_config(rejected: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  field.quality: error\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status, Owner]\n    header_shape: yaml-frontmatter\n  waiver:\n    dir: docs/waivers\n    prefix: WAIVER\n    role: suppression\n    required_fields: [Rule, Scope, Reason]\n    known_fields: [Expires]\n    header_shape: yaml-frontmatter\n    effective_requires: [Reason]\n{rejected}"
    )
}

fn waived(expires: &str, rejected: &str) -> bool {
    let dir = repo(
        "rejected",
        &waiver_config(rejected),
        &[
            (
                "docs/adr/ADR-1-a.md",
                "---\nStatus: Accepted\nOwner: TBD\n---\n# 1 — A\n",
            ),
            (
                "docs/waivers/WAIVER-1-a.md",
                &format!("---\nRule: field.quality\nScope: docs/adr/ADR-1-a.md\nReason: The owner is assigned at the next review.\nExpires: {expires}\n---\n# 1 — A\n"),
            ),
        ],
    );
    let found = findings(dir.path(), "field.quality");
    assert_eq!(found.len(), 1, "{found:?}");
    !found[0]["waived"].is_null()
}

#[test]
fn an_added_rejected_expires_value_keeps_a_waiver_from_taking_effect() {
    let rejected = "    rejected_values: ['2099-12-31']\n";
    assert!(waived("2099-12-31", ""));
    assert!(!waived("2099-12-31", rejected));
    assert!(waived("2099-12-30", rejected));
}

#[test]
fn an_added_rejected_reviewer_cannot_lift_the_built_in_floor() {
    // A type's own placeholder words replace `TBD`, yet a waiver Reason of
    // `TBD` still never takes effect: the built-in words are a floor.
    let config = waiver_config("").replace(
        "    effective_requires: [Reason]\n",
        "    effective_requires: [Reason]\n    statuses:\n      pending: [offen]\n      placeholders: [ausstehend]\n",
    );
    let dir = repo(
        "floor",
        &config,
        &[
            (
                "docs/adr/ADR-1-a.md",
                "---\nStatus: Accepted\nOwner: \n---\n# 1 — A\n",
            ),
            (
                "docs/waivers/WAIVER-1-a.md",
                "---\nRule: field.quality\nScope: docs/adr/ADR-1-a.md\nReason: TBD\nExpires: 2099-01-01\n---\n# 1 — A\n",
            ),
        ],
    );
    let found = findings(dir.path(), "field.quality");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0]["waived"].is_null(), "{found:?}");
}

#[test]
fn each_invalid_declaration_is_a_load_error_naming_its_key() {
    let cases: Vec<(String, &str)> = vec![
        (
            back_pointer_config("    implements_marker: 'Realises'\n"),
            "implements_marker",
        ),
        (
            back_pointer_config("    implements_marker: ''\n"),
            "implements_marker",
        ),
        (
            back_pointer_config("    verifies_marker: 'Implements:'\n"),
            "verifies_marker",
        ),
        (
            back_pointer_config("    comment_leaders: []\n"),
            "comment_leaders",
        ),
        (
            back_pointer_config("    comment_leaders: ['#', '#']\n"),
            "comment_leaders",
        ),
        (
            back_pointer_config("    trailing_comment_leaders: ['']\n"),
            "trailing_comment_leaders",
        ),
        // A wrong type is refused by the deserializer, which names the line
        // rather than the key, as for every rule option.
        (
            back_pointer_config("    comment_leaders: '#'\n"),
            "expected a sequence",
        ),
        (
            back_pointer_config("    implement_marker: 'X:'\n"),
            "implement_marker",
        ),
        (claim_config("    closing_verbs: []\n"), "closing_verbs"),
        (
            claim_config("    closing_verbs: [fixes, Fixes]\n"),
            "closing_verbs",
        ),
        (
            claim_config("    closing_verbs: ['closes out']\n"),
            "closing_verbs",
        ),
        (
            placeholder_config("      placeholders: []\n"),
            "placeholders",
        ),
        (
            placeholder_config("      placeholders: [tbd, TBD]\n"),
            "placeholders",
        ),
        (y_config(", phrases: []"), "phrases"),
        (y_config(", phrases: [a, ' A ']"), "phrases"),
        (
            waiver_config("    rejected_values: []\n"),
            "rejected_values",
        ),
        (
            waiver_config("    rejected_values: [x, X]\n"),
            "rejected_values",
        ),
        (
            waiver_config("    waiver_fields: {reason: Rule}\n"),
            "waiver_fields",
        ),
        (
            waiver_config("    waiver_fields: {reasons: Grund}\n"),
            "reasons",
        ),
        (
            override_config("    rejected_values: []\n"),
            "rejected_values",
        ),
    ];
    for (config, key) in cases {
        let dir = repo("invalid", &config, &[]);
        let (code, out, err) = run(dir.path(), &["check"]);
        let text = out + &err;
        assert_eq!(code, Some(2), "{key}: {text}");
        assert!(text.contains(key), "{key}: {text}");
    }
}

fn override_config(options: &str) -> String {
    format!(
        "schema_version: 2\noverride_log: .urzua/override-log.md\nrules:\n  override.unreviewed:\n    level: error\n{options}{ADR_TYPE}"
    )
}

#[test]
fn an_added_rejected_reviewer_is_not_a_review_and_the_built_in_floor_stays() {
    let log = |reviewer: &str| {
        format!("| Date | Name | Record | Field | Rule | Reason | Reviewed-by |\n|---|---|---|---|---|---|---|\n| 2026-11-05 | alice | docs/adr/ADR-1-a.md | Status | field.quality | draft | {reviewer} |\n")
    };
    let unreviewed = |reviewer: &str, options: &str| {
        let dir = repo(
            "reviewer",
            &override_config(options),
            &[
                (
                    "docs/adr/ADR-1-a.md",
                    "---\nStatus: Accepted\n---\n# 1 — A\n",
                ),
                (".urzua/override-log.md", &log(reviewer)),
            ],
        );
        findings(dir.path(), "override.unreviewed").len()
    };
    let added = "    rejected_values: [niemand]\n";
    assert_eq!(unreviewed("niemand", ""), 0);
    assert_eq!(unreviewed("niemand", added), 1);
    assert_eq!(unreviewed("TBD", added), 1);
    assert_eq!(unreviewed("carol", added), 0);
}

/// A waiver's required field holding a word its own type declares as a
/// placeholder does not take effect, and the built-in words still block
/// beside the declared ones: for a suppression they only add.
#[test]
fn a_waiver_type_s_declared_placeholders_add_to_the_floor() {
    let config = waiver_config("").replace(
        "    effective_requires: [Reason]\n",
        "    effective_requires: [Reason]\n    statuses:\n      pending: [offen]\n      placeholders: [ausstehend]\n",
    );
    let takes_effect = |reason: &str| {
        let dir = repo(
            "waiver-placeholders",
            &config,
            &[
                ("docs/adr/ADR-1-a.md", "---\nStatus: Accepted\nOwner: TBD\n---\n# 1 — A\n"),
                (
                    "docs/waivers/WAIVER-1-a.md",
                    &format!("---\nRule: field.quality\nScope: docs/adr/ADR-1-a.md\nReason: {reason}\nExpires: 2099-01-01\n---\n# 1 — A\n"),
                ),
            ],
        );
        let found: Vec<_> = findings(dir.path(), "field.quality")
            .into_iter()
            .filter(|f| f["file"] == "docs/adr/ADR-1-a.md")
            .collect();
        assert_eq!(found.len(), 1, "{found:?}");
        !found[0]["waived"].is_null()
    };
    assert!(!takes_effect("ausstehend"));
    assert!(!takes_effect("Ausstehend"));
    assert!(!takes_effect("TBD"));
    assert!(takes_effect("The owner is assigned at the next review."));
}

/// `rejected_values: null` is refused alike on a rule and on a type.
#[test]
fn a_null_rejected_values_is_refused_on_a_rule_and_on_a_type() {
    for config in [
        override_config("    rejected_values: null\n"),
        waiver_config("    rejected_values: null\n"),
    ] {
        let dir = repo("null", &config, &[]);
        let (code, out, err) = run(dir.path(), &["check"]);
        assert_eq!(code, Some(2), "{out}{err}");
        assert!((out + &err).contains("'rejected_values' is present with no value"));
    }
}
