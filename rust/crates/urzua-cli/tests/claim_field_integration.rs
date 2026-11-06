//! `ADR-85`: a changeset fragment declares the records it closes in a
//! frontmatter field, and `check` reads that data instead of a sentence. Each
//! case runs the real binary over a scratch repository.
//!
//! Verifies: ADR-85

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const TYPES: &str = "record_types:\n  bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      values: [Open, Fixed]\n      success: [Fixed]\n";

fn config(claim_options: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n    closed_statuses: [\"Fixed\"]\n{claim_options}\n{TYPES}"
    )
}

/// A repository with `BUG-1` open and `BUG-2` fixed, and the given fragments.
fn repo(name: &str, claim_options: &str, fragments: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-claim-field-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join(".changeset")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(claim_options)).unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-open.md"),
        "---\nStatus: Open\n---\n# 1 — Open\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-2-fixed.md"),
        "---\nStatus: Fixed\n---\n# 2 — Fixed\n",
    )
    .unwrap();
    for (file, text) in fragments {
        std::fs::write(dir.join(".changeset").join(file), text).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(dir)
        .output()
        .expect("urzua should run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {stdout}"));
    (out.status.code().unwrap_or(-1), json)
}

fn claim_findings(report: &serde_json::Value) -> Vec<&serde_json::Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "claim.status-agreement")
        .collect()
}

const OPTIONS: &str = "    closes: closes\n    strict: true\n";

fn fragment(closes: &str) -> String {
    format!("---\ndefault: patch\ncloses: {closes}\n---\n\nA change.\n")
}

/// `BUG-66`: five bugs named `(BUG-60)` in a fragment while `Open` were
/// invisible. Listed in `closes`, the open one is an error with a suggested
/// action on the record's `Status`.
#[test]
fn a_fragment_listing_a_still_open_record_fails_the_check_observed_failing() {
    let dir = repo("open", OPTIONS, &[("a.md", &fragment("[BUG-1, BUG-2]"))]);
    let (code, report) = check(&dir);
    let found = claim_findings(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(code, 1, "{report}");
    assert_eq!(found[0]["file"], ".changeset/a.md");
    assert_eq!(found[0]["line"], 3);
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("BUG-1 has Status Open"));
    let action = &found[0]["suggested_action"];
    assert_eq!(action["kind"], "edit-field", "{report}");
    assert_eq!(action["target"]["file"], "docs/bugs/BUG-1-open.md");
    assert_eq!(action["target"]["field"], "Status");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn closed_records_an_empty_list_and_a_non_fragment_are_clean_and_disclosed_observed_failing() {
    let dir = repo(
        "clean",
        OPTIONS,
        &[
            ("a.md", &fragment("[BUG-2]")),
            ("b.md", &fragment("[]")),
            ("README.md", "# What this directory is\n"),
        ],
    );
    let (code, report) = check(&dir);
    assert!(claim_findings(&report).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "claim.status-agreement")
        .unwrap();
    assert_eq!(rule["population"]["out_of_scope"], 1, "{report}");
    assert_eq!(rule["population"]["examined"], 1, "{report}");
    let notices = report["notices"].to_string();
    assert!(notices.contains("README.md"), "{notices}");
    assert!(notices.contains("no frontmatter"), "{notices}");
    assert!(notices.contains("0 closing-verb claim"), "{notices}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn strict_catches_the_incident_shape_a_fragment_naming_a_record_and_declaring_nothing_observed_failing(
) {
    let bare = "---\ndefault: patch\n---\n\nTightens the thing (BUG-1).\n";
    let dir = repo("strict", OPTIONS, &[("a.md", bare)]);
    let (code, report) = check(&dir);
    let found = claim_findings(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("claim.field-missing"));
    assert_eq!(code, 1);
    std::fs::remove_dir_all(&dir).ok();

    // Not strict: the field is checked when present, and this fragment has none.
    let dir = repo("not-strict", "    closes: closes\n", &[("a.md", bare)]);
    let (code, report) = check(&dir);
    assert!(claim_findings(&report).is_empty(), "{report}");
    assert_eq!(code, 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_block_list_an_unresolved_id_and_a_non_id_are_findings_observed_failing() {
    let dir = repo(
        "bad",
        OPTIONS,
        &[
            ("block.md", "---\ndefault: patch\ncloses:\n  - BUG-2\n---\n"),
            ("gone.md", &fragment("[BUG-99]")),
            ("words.md", &fragment("fixed")),
        ],
    );
    let (code, report) = check(&dir);
    let found = claim_findings(&report);
    let message = |file: &str| -> String {
        found
            .iter()
            .find(|f| f["file"] == file)
            .unwrap_or_else(|| panic!("no finding on {file}: {report}"))["message"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(message(".changeset/block.md").contains("claim.field-malformed"));
    assert!(message(".changeset/block.md").contains("block list"));
    assert!(message(".changeset/gone.md").contains("claim.id-unresolved"));
    assert!(message(".changeset/words.md").contains("claim.field-malformed"));
    assert_eq!(found.len(), 3, "{report}");
    assert_eq!(code, 1);
    std::fs::remove_dir_all(&dir).ok();
}

/// Declaring neither option changes nothing: the same fragments, read by the
/// closing verb alone, are reported as they always were.
#[test]
fn without_the_options_the_rule_reads_closing_verbs_alone_observed_failing() {
    let dir = repo(
        "undeclared",
        "",
        &[
            (
                "block.md",
                "---\ndefault: patch\ncloses:\n  - BUG-1\n---\n\nNo verb.\n",
            ),
            (
                "gone.md",
                "---\ndefault: patch\n---\n\nSee BUG-99 and BUG-1.\n",
            ),
        ],
    );
    let (code, report) = check(&dir);
    assert!(claim_findings(&report).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
    assert!(report
        .get("notices")
        .is_none_or(|n| !n.to_string().contains("closing-verb")));
    std::fs::remove_dir_all(&dir).ok();
}

/// A closing verb in the body still works with the field declared, and the
/// frontmatter line `closes: [BUG-1]` is not read a second time as one.
#[test]
fn a_verb_claim_in_the_body_is_still_found_and_counted_observed_failing() {
    let text = "---\ndefault: patch\ncloses: [BUG-2]\n---\n\nFixes BUG-1.\n";
    let dir = repo("verb", OPTIONS, &[("a.md", text)]);
    let (code, report) = check(&dir);
    let found = claim_findings(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["line"], 6);
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("claims to close BUG-1"));
    assert_eq!(code, 1);
    assert!(report["notices"]
        .to_string()
        .contains("1 closing-verb claim"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn strict_without_closes_does_not_load_observed_failing() {
    let dir = repo("strict-alone", "    strict: true\n", &[]);
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("strict") && text.contains("closes"), "{text}");
    std::fs::remove_dir_all(&dir).ok();
}

/// The release tool refuses an unterminated fence and a blank line before the
/// opening fence (`could_not_read_changeset`), so under `strict` both are
/// `claim.field-malformed` rather than a file that is out of scope.
#[test]
fn strict_reports_a_fence_the_release_tool_cannot_read_observed_failing() {
    let unterminated = "---\ndefault: patch\ncloses: [BUG-2]\n\nA change.\n";
    let blank_first = "\n---\ndefault: patch\ncloses: [BUG-2]\n---\n\nA change.\n";
    for (name, text) in [("unterminated", unterminated), ("blank-first", blank_first)] {
        let dir = repo(name, OPTIONS, &[("a.md", text)]);
        let (code, report) = check(&dir);
        let found = claim_findings(&report);
        assert_eq!(found.len(), 1, "{name}: {report}");
        assert!(
            found[0]["message"]
                .as_str()
                .unwrap()
                .contains("claim.field-malformed"),
            "{name}: {report}"
        );
        assert_eq!(code, 1, "{name}");
        std::fs::remove_dir_all(&dir).ok();

        // Not strict: read as no frontmatter, out of scope, no finding.
        let dir = repo(name, "    closes: closes\n", &[("a.md", text)]);
        let (code, report) = check(&dir);
        assert!(claim_findings(&report).is_empty(), "{name}: {report}");
        assert_eq!(code, 0, "{name}");
        std::fs::remove_dir_all(&dir).ok();
    }
}

// ---- `verb_claims` (ADR-88, MILE-197)

const VERB_OFF: &str = "    closes: closes\n    strict: true\n    verb_claims: false\n";

fn load_error(dir: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// With the reading off, prose is never a claim: `Fixes BUG-1` in the body
/// raises nothing and counts nothing, while the same fragment's `closes`
/// still decides.
#[test]
fn verb_claims_false_turns_the_prose_reading_off_and_keeps_the_field_observed_failing() {
    let prose = "---\ndefault: patch\ncloses: [BUG-2]\n---\n\nFixes BUG-1.\n";
    let dir = repo("verb-off", VERB_OFF, &[("a.md", prose)]);
    let (code, report) = check(&dir);
    assert!(claim_findings(&report).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
    let notices = report["notices"].to_string();
    assert!(notices.contains("closing-verb reading is off"), "{notices}");
    assert!(!notices.contains("0 closing-verb"), "no count: {notices}");
    std::fs::remove_dir_all(&dir).ok();

    let listed = "---\ndefault: patch\ncloses: [BUG-1]\n---\n\nFixes BUG-1.\n";
    let dir = repo("verb-off-listed", VERB_OFF, &[("a.md", listed)]);
    let (code, report) = check(&dir);
    let found = claim_findings(&report);
    assert_eq!(found.len(), 1, "only the field claims: {report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .starts_with("claim.closes-open"));
    assert_eq!(code, 1);
    std::fs::remove_dir_all(&dir).ok();
}

/// Absent and `true` read prose exactly as before, byte for byte.
#[test]
fn verb_claims_absent_or_true_is_the_reading_as_it_was_observed_failing() {
    let prose = "---\ndefault: patch\ncloses: [BUG-2]\n---\n\nFixes BUG-1.\n";
    let dir_absent = repo("verb-absent", OPTIONS, &[("a.md", prose)]);
    let dir_on = repo(
        "verb-on",
        "    closes: closes\n    strict: true\n    verb_claims: true\n",
        &[("a.md", prose)],
    );
    let absent = check(&dir_absent);
    let on = check(&dir_on);
    assert_eq!(absent.0, 1);
    assert_eq!(claim_findings(&absent.1).len(), 1, "{}", absent.1);
    assert_eq!(absent, on);
    std::fs::remove_dir_all(&dir_absent).ok();
    std::fs::remove_dir_all(&dir_on).ok();
}

#[test]
fn verb_claims_needs_closes_whether_true_or_false_observed_failing() {
    for value in ["true", "false"] {
        let dir = repo("verb-alone", &format!("    verb_claims: {value}\n"), &[]);
        let text = load_error(&dir);
        assert!(
            text.contains("verb_claims") && text.contains("needs `closes`"),
            "{value}: {text}"
        );
        assert!(!text.contains("does not take"), "{text}");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn verb_claims_on_another_rule_is_not_applicable_observed_failing() {
    let dir = repo("verb-other", "", &[]);
    let path = dir.join(".urzua/config.yaml");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text = text.replace(
        "rules:\n",
        "rules:\n  field.quality:\n    level: error\n    verb_claims: false\n",
    );
    std::fs::write(&path, text).unwrap();
    let text = load_error(&dir);
    assert!(
        text.contains("does not take the option 'verb_claims'"),
        "{text}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// Off without `strict`: a fragment lacking the key then has no claim at all,
/// and the run says so instead of leaving the gap silent.
#[test]
fn verb_claims_false_without_strict_carries_a_notice_observed_failing() {
    let bare = "---\ndefault: patch\n---\n\nFixes BUG-1.\n";
    let dir = repo(
        "verb-off-lax",
        "    closes: closes\n    verb_claims: false\n",
        &[("a.md", bare)],
    );
    let (code, report) = check(&dir);
    assert_eq!(code, 0, "{report}");
    assert!(claim_findings(&report).is_empty(), "{report}");
    let notices = report["notices"].to_string();
    assert!(
        notices.contains("without `strict`") && notices.contains("no claim"),
        "{notices}"
    );
    std::fs::remove_dir_all(&dir).ok();

    // With `strict` the gap is a finding, so no such notice.
    let dir = repo("verb-off-strict", VERB_OFF, &[("a.md", bare)]);
    let (_, report) = check(&dir);
    assert!(!report["notices"].to_string().contains("without `strict`"));
    std::fs::remove_dir_all(&dir).ok();
}
