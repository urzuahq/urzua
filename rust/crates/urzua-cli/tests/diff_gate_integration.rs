//! MILE-124 and MILE-142: the two diff-gate rules, `diff.source-requires-record`
//! and `diff.source-and-records-mixed`. Both judge the set of changed paths, so
//! they run only on a `--base` run and say `not-applicable` on any other,
//! never a quiet pass (SPEC-2, Rules).

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn config(rules: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n{rules}\nrecord_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
    )
}

const REQUIRE: &str =
    "  diff.source-requires-record:\n    level: error\n    source_paths: [\"src\"]\n";
const MIXED: &str = "  diff.source-and-records-mixed:\n    level: warn\n    application_paths: [\"src\"]\n    exclude_paths: [\"src/tests\"]\n";

/// A repository whose base commit (tagged `base`) holds one record and two
/// source files; nothing has changed since.
fn fixture(name: &str, rules: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-diff-gate-{name}"));
    for sub in ["docs/adr/archive", ".urzua", "src/tests"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(rules)).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("src/tests/t.rs"), "fn t() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn edit(dir: &Path, rel: &str) {
    let path = dir.join(rel);
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\nchanged\n");
    std::fs::write(path, text).unwrap();
}

fn check(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);
    (out.status.code().unwrap_or(-1), parsed)
}

fn findings(report: &serde_json::Value, rule: &str) -> Vec<serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|f| f.iter().filter(|f| f["rule"] == rule).cloned().collect())
        .unwrap_or_default()
}

fn execution<'a>(report: &'a serde_json::Value, rule: &str) -> &'a serde_json::Value {
    report["rules_executed"]
        .as_array()
        .and_then(|e| e.iter().find(|e| e["rule"] == rule))
        .unwrap_or_else(|| panic!("{rule} missing from rules_executed: {report}"))
}

const R124: &str = "diff.source-requires-record";
const R142: &str = "diff.source-and-records-mixed";

#[test]
fn source_change_with_no_record_change_is_an_error_naming_the_path() {
    let dir = fixture("req-source-only", REQUIRE);
    edit(&dir, "src/lib.rs");
    let (code, report) = check(&dir, &["--base", "base"]);
    let found = findings(&report, R124);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "src/lib.rs");
    assert_eq!(found[0]["severity"], "error");
    assert_eq!(code, 1, "{report}");
    assert_eq!(execution(&report, R124)["status"], "ran");
}

#[test]
fn source_change_alongside_a_record_change_is_clean() {
    let dir = fixture("req-both", REQUIRE);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/adr/ADR-1-x.md");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert!(findings(&report, R124).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
    assert_eq!(execution(&report, R124)["status"], "ran");
}

#[test]
fn a_record_change_alone_and_an_undeclared_path_change_are_clean() {
    let dir = fixture("req-other", REQUIRE);
    edit(&dir, "docs/adr/ADR-1-x.md");
    std::fs::write(dir.join("README.md"), "x\n").unwrap();
    git(&dir, &["add", "README.md"]);
    let (code, report) = check(&dir, &["--base", "base"]);
    assert!(findings(&report, R124).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_file_below_a_record_directory_is_not_a_governed_record_change() {
    let dir = fixture("req-nested", REQUIRE);
    edit(&dir, "src/lib.rs");
    std::fs::write(
        dir.join("docs/adr/archive/ADR-9-y.md"),
        "---\nStatus: x\n---\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(findings(&report, R124).len(), 1, "{report}");
}

#[test]
fn without_base_the_rule_reports_not_applicable_and_the_run_is_not_ok() {
    let dir = fixture("req-nobase", REQUIRE);
    edit(&dir, "src/lib.rs");
    let (code, report) = check(&dir, &[]);
    let exec = execution(&report, R124);
    assert_eq!(exec["status"], "not-applicable", "{report}");
    assert!(exec.get("population").is_none(), "{report}");
    assert!(findings(&report, R124).is_empty());
    assert_eq!(
        report["status"], "not-run",
        "a skipped gate is not a pass: {report}"
    );
    assert_ne!(code, 0, "{report}");
}

#[test]
fn mixed_change_warns_once_naming_a_path_from_each_side() {
    let dir = fixture("mix-both", MIXED);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/adr/ADR-1-x.md");
    let (code, report) = check(&dir, &["--base", "base"]);
    let found = findings(&report, R142);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["severity"], "warning");
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("docs/adr/ADR-1-x.md"), "{message}");
    assert!(message.contains("src/lib.rs"), "{message}");
    assert_eq!(code, 0, "a warning does not block: {report}");
}

#[test]
fn a_change_touching_only_one_side_does_not_warn() {
    for (name, path) in [
        ("mix-src", "src/lib.rs"),
        ("mix-rec", "docs/adr/ADR-1-x.md"),
    ] {
        let dir = fixture(name, MIXED);
        edit(&dir, path);
        let (_, report) = check(&dir, &["--base", "base"]);
        assert!(findings(&report, R142).is_empty(), "{path}: {report}");
        assert_eq!(execution(&report, R142)["status"], "ran");
    }
}

#[test]
fn excluded_paths_are_not_application_source() {
    let dir = fixture("mix-excluded", MIXED);
    edit(&dir, "src/tests/t.rs");
    edit(&dir, "docs/adr/ADR-1-x.md");
    let (_, report) = check(&dir, &["--base", "base"]);
    assert!(findings(&report, R142).is_empty(), "{report}");
}

#[test]
fn mixed_rule_without_base_is_not_applicable() {
    let dir = fixture("mix-nobase", MIXED);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/adr/ADR-1-x.md");
    let (_, report) = check(&dir, &[]);
    assert_eq!(
        execution(&report, R142)["status"],
        "not-applicable",
        "{report}"
    );
    assert!(findings(&report, R142).is_empty());
    assert_ne!(report["status"], "ok", "{report}");
}

#[test]
fn both_rules_need_their_path_options_to_load() {
    for rule in [
        "  diff.source-requires-record: error\n",
        "  diff.source-and-records-mixed: warn\n",
    ] {
        let dir = fixture("noopts", rule);
        let (code, _) = check(&dir, &[]);
        assert_eq!(code, 2, "{rule} must not load without its paths");
    }
}

#[test]
fn a_path_option_that_escapes_the_repository_is_rejected() {
    let dir = fixture(
        "badpath",
        "  diff.source-requires-record:\n    level: error\n    source_paths: [\"../x\"]\n",
    );
    let (code, _) = check(&dir, &["--base", "base"]);
    assert_eq!(code, 2);
}

/// MILE-158: tests beside the code, stated as a glob exclusion.
const REQUIRE_EXCLUDING_TESTS: &str = "  diff.source-requires-record:\n    level: error\n    source_paths: [\"src\"]\n    exclude_paths: [\"**/*.test.*\", \"src/tests\"]\n";

fn add_file(dir: &Path, rel: &str) {
    std::fs::write(dir.join(rel), "fn t() {}\n").unwrap();
    git(dir, &["add", rel]);
}

#[test]
fn a_diff_touching_only_test_files_beside_the_code_is_not_a_trigger() {
    let dir = fixture("req-glob-tests-only", REQUIRE_EXCLUDING_TESTS);
    add_file(&dir, "src/lib.test.rs");
    edit(&dir, "src/tests/t.rs");
    let (code, report) = check(&dir, &["--base", "base"]);
    assert!(findings(&report, R124).is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
    let run = execution(&report, R124);
    assert_eq!(run["status"], "ran");
    assert_eq!(run["population"]["eligible"], 2, "{run}");
    assert_eq!(run["population"]["examined"], 0, "{run}");
    assert!(
        findings(&report, "config.scope-matches-nothing").is_empty(),
        "{report}"
    );
}

#[test]
fn code_changed_beside_its_tests_still_triggers_and_names_the_code() {
    let dir = fixture("req-glob-code-and-tests", REQUIRE_EXCLUDING_TESTS);
    add_file(&dir, "src/lib.test.rs");
    edit(&dir, "src/lib.rs");
    let (code, report) = check(&dir, &["--base", "base"]);
    let found = findings(&report, R124);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "src/lib.rs");
    assert_eq!(code, 1, "{report}");
}

#[test]
fn a_prefix_only_configuration_still_treats_a_test_file_as_source() {
    let dir = fixture("req-glob-unchanged", REQUIRE);
    add_file(&dir, "src/lib.test.rs");
    let (_, report) = check(&dir, &["--base", "base"]);
    assert_eq!(findings(&report, R124).len(), 1, "{report}");
}

/// MILE-158, dogfood: this repository's own `diff.source-requires-record`
/// declaration, run on a throwaway repository with the same layout.
#[test]
fn this_repositorys_own_config_lets_a_crate_test_change_through_but_not_its_source() {
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    let dir = common::TempDir::new("urzua-diff-gate-own");
    for sub in [".urzua", "rust/crates/x/src", "rust/crates/x/tests"] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::copy(own, dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(dir.join("rust/crates/x/src/lib.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("rust/crates/x/tests/t.rs"), "fn t() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);

    edit(&dir, "rust/crates/x/tests/t.rs");
    let (_, report) = check(&dir, &["--base", "base"]);
    assert!(findings(&report, R124).is_empty(), "{report}");
    assert_eq!(
        execution(&report, R124)["population"]["eligible"],
        1,
        "{report}"
    );
    assert!(
        findings(&report, "config.scope-matches-nothing").is_empty(),
        "{report}"
    );

    edit(&dir, "rust/crates/x/src/lib.rs");
    let (_, report) = check(&dir, &["--base", "base"]);
    let found = findings(&report, R124);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "rust/crates/x/src/lib.rs");
}

// ---------------------------------------------------------------- MILE-133
// The decision gate: `decision_types`, `fix_types` and `link_fields` turn
// `diff.source-requires-record` from "some governed record changed" into "a
// decision at a decided status, a record linking one, or a fix record changed".

const GATE_RULE: &str = "  diff.source-requires-record:\n    level: error\n    source_paths: [\"src\"]\n    decision_types:\n      adr: [Accepted]\n      spec: [Accepted, Shipped]\n    fix_types: [bug]\n    link_fields: [Implements]\n";

const GATE_TYPES: &str = "\
\x20 adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      values: [Proposed, Accepted, Rejected]\n      success: [Accepted]\n      failure: [Rejected]\n\
\x20 spec:\n    dir: \"docs/specs\"\n    prefix: \"SPEC\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n\
\x20 milestone:\n    dir: \"docs/milestones\"\n    prefix: \"MILE\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Implements\"]\n    header_shape: \"yaml-frontmatter\"\n    pointer_fields: [\"Implements\"]\n    narrative_fields: []\n\
\x20 bug:\n    dir: \"docs/bugs\"\n    prefix: \"BUG\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n";

fn gate_config(rule: &str) -> String {
    format!("schema_version: 2\nrules:\n{rule}\nrecord_types:\n{GATE_TYPES}")
}

/// A base commit (tagged `base`) holding: ADR-1 Accepted, ADR-2 Proposed,
/// SPEC-1 Shipped, SPEC-2 Draft, MILE-1 implementing ADR-1, MILE-2
/// implementing ADR-2, MILE-3 implementing nothing, BUG-1 Fixed, and a source
/// file.
fn gate_fixture(name: &str, rule: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-diff-gate-decision-{name}"));
    for sub in [
        "docs/adr",
        "docs/specs",
        "docs/milestones",
        "docs/bugs",
        ".urzua",
        "src",
    ] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.join(".urzua/config.yaml"), gate_config(rule)).unwrap();
    let records = [
        ("docs/adr/ADR-1-a.md", "Status: Accepted"),
        ("docs/adr/ADR-2-b.md", "Status: Proposed"),
        ("docs/specs/SPEC-1-s.md", "Status: Shipped"),
        ("docs/specs/SPEC-2-d.md", "Status: Draft"),
        (
            "docs/milestones/MILE-1-m.md",
            "Status: Planned\nImplements: ADR-1",
        ),
        (
            "docs/milestones/MILE-2-n.md",
            "Status: Planned\nImplements: ADR-2",
        ),
        (
            "docs/milestones/MILE-3-o.md",
            "Status: Planned\nImplements: —",
        ),
        ("docs/bugs/BUG-1-p.md", "Status: Fixed"),
    ];
    for (path, header) in records {
        std::fs::write(dir.join(path), format!("---\n{header}\n---\n# t\n")).unwrap();
    }
    std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn gate_findings(dir: &Path) -> (i32, Vec<serde_json::Value>, serde_json::Value) {
    let (code, report) = check(dir, &["--base", "base"]);
    (code, findings(&report, R124), report)
}

#[test]
fn a_source_change_beside_only_an_undecided_decision_record_is_reported_observed_failing() {
    let dir = gate_fixture("proposed", GATE_RULE);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/adr/ADR-2-b.md");
    let (code, found, report) = gate_findings(&dir);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "src/lib.rs");
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("ADR-2 (Proposed)"), "{message}");
    assert!(message.contains("Accepted"), "{message}");
    assert_eq!(code, 1, "{report}");
}

#[test]
fn a_source_change_beside_a_decision_at_a_decided_status_is_clean() {
    for (case, record) in [
        ("adr", "docs/adr/ADR-1-a.md"),
        ("spec-shipped", "docs/specs/SPEC-1-s.md"),
    ] {
        let dir = gate_fixture(case, GATE_RULE);
        edit(&dir, "src/lib.rs");
        edit(&dir, record);
        let (code, found, report) = gate_findings(&dir);
        assert!(found.is_empty(), "{case}: {report}");
        assert_eq!(code, 0, "{case}: {report}");
        assert_eq!(execution(&report, R124)["status"], "ran");
    }
}

#[test]
fn a_decided_status_is_judged_per_type() {
    // SPEC-2 is a Draft: spec lists Accepted and Shipped, not Draft.
    let dir = gate_fixture("spec-draft", GATE_RULE);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/specs/SPEC-2-d.md");
    let (_, found, report) = gate_findings(&dir);
    assert_eq!(found.len(), 1, "{report}");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("SPEC-2 (Draft)"));
}

#[test]
fn a_record_linking_a_decided_decision_through_a_link_field_satisfies_the_gate() {
    let dir = gate_fixture("linked", GATE_RULE);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/milestones/MILE-1-m.md");
    let (code, found, report) = gate_findings(&dir);
    assert!(found.is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_record_linking_an_undecided_decision_or_nothing_does_not() {
    for (case, record) in [
        ("link-proposed", "docs/milestones/MILE-2-n.md"),
        ("link-none", "docs/milestones/MILE-3-o.md"),
    ] {
        let dir = gate_fixture(case, GATE_RULE);
        edit(&dir, "src/lib.rs");
        edit(&dir, record);
        let (code, found, report) = gate_findings(&dir);
        assert_eq!(found.len(), 1, "{case}: {report}");
        assert_eq!(code, 1, "{case}: {report}");
    }
}

#[test]
fn a_fix_record_exempts_the_change() {
    let dir = gate_fixture("fix", GATE_RULE);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/bugs/BUG-1-p.md");
    let (code, found, report) = gate_findings(&dir);
    assert!(found.is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_deleted_decision_record_does_not_satisfy_the_gate() {
    let dir = gate_fixture("deleted", GATE_RULE);
    edit(&dir, "src/lib.rs");
    git(&dir, &["rm", "-q", "docs/adr/ADR-1-a.md"]);
    let (_, found, report) = gate_findings(&dir);
    assert_eq!(found.len(), 1, "{report}");
}

#[test]
fn the_gate_is_off_unless_declared_so_any_governed_record_still_satisfies_the_rule() {
    let dir = gate_fixture("undeclared", REQUIRE);
    edit(&dir, "src/lib.rs");
    edit(&dir, "docs/adr/ADR-2-b.md");
    let (code, found, report) = gate_findings(&dir);
    assert!(found.is_empty(), "{report}");
    assert_eq!(code, 0, "{report}");
}

#[test]
fn a_gate_finding_offers_an_add_record_action() {
    let dir = gate_fixture("action", GATE_RULE);
    edit(&dir, "src/lib.rs");
    let (_, found, report) = gate_findings(&dir);
    assert_eq!(found.len(), 1, "{report}");
    let action = &found[0]["suggested_action"];
    assert_eq!(action["kind"], "add-record", "{report}");
    // Two decision types are declared, so the type to add is not determinable.
    assert!(action["target"].get("type").is_none(), "{report}");
}

#[test]
fn a_single_decision_type_names_the_record_to_add() {
    let rule = "  diff.source-requires-record:\n    level: error\n    source_paths: [\"src\"]\n    decision_types:\n      adr: [Accepted]\n";
    let dir = gate_fixture("one-type", rule);
    edit(&dir, "src/lib.rs");
    let (_, found, report) = gate_findings(&dir);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(
        found[0]["suggested_action"]["target"]["type"], "adr",
        "{report}"
    );
}

#[test]
fn a_gate_declaration_that_can_never_match_is_a_config_error() {
    let source = "    source_paths: [\"src\"]\n";
    let cases = [
        (
            "fix_types without decision_types",
            format!("{source}    fix_types: [bug]\n"),
            "decision_types",
        ),
        (
            "link_fields without decision_types",
            format!("{source}    link_fields: [Implements]\n"),
            "decision_types",
        ),
        (
            "an undeclared type",
            format!("{source}    decision_types:\n      nosuch: [Accepted]\n"),
            "nosuch",
        ),
        (
            "a status outside the vocabulary",
            format!("{source}    decision_types:\n      adr: [Approved]\n"),
            "Approved",
        ),
        (
            "an empty status list",
            format!("{source}    decision_types:\n      adr: []\n"),
            "adr",
        ),
        (
            "an undeclared fix type",
            format!(
                "{source}    decision_types:\n      adr: [Accepted]\n    fix_types: [nosuch]\n"
            ),
            "nosuch",
        ),
        (
            "a type both decision and fix",
            format!("{source}    decision_types:\n      adr: [Accepted]\n    fix_types: [adr]\n"),
            "adr",
        ),
        (
            "an undeclared link field",
            format!(
                "{source}    decision_types:\n      adr: [Accepted]\n    link_fields: [Nosuch]\n"
            ),
            "Nosuch",
        ),
    ];
    for (case, options, needle) in cases {
        let rule = format!("  diff.source-requires-record:\n    level: error\n{options}");
        let dir = gate_fixture("bad-config", &rule);
        let out = common::urzua()
            .args(["check", "--base", "base"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{case}: {text}");
        assert!(text.contains(needle), "{case}: {text}");
    }
}

#[test]
fn the_gate_options_are_refused_on_a_rule_that_does_not_take_them() {
    let rule = format!("{MIXED}    decision_types:\n      adr: [Accepted]\n");
    let dir = gate_fixture("wrong-rule", &rule);
    let out = common::urzua()
        .args(["check", "--base", "base"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stdout).contains("decision_types"));
}

/// MILE-133, dogfood: this repository's own declaration, run on a throwaway
/// repository carrying its record types.
#[test]
fn this_repositorys_own_gate_holds_a_source_change_until_a_decision_fix_or_link_changes() {
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.urzua/config.yaml");
    let dir = common::TempDir::new("urzua-diff-gate-own-decision");
    for sub in [
        ".urzua",
        "rust/crates/x/src",
        "docs/adr",
        "docs/specs",
        "docs/milestones",
        "docs/bugs",
    ] {
        std::fs::create_dir_all(dir.join(sub)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::copy(own, dir.join(".urzua/config.yaml")).unwrap();
    let write = |path: &str, header: &str| {
        std::fs::write(
            dir.join(path),
            format!("---\n{header}\n---\n# t\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | x | **structural** |\n"),
        )
        .unwrap();
    };
    write("docs/adr/ADR-1-a.md", "Status: Accepted");
    write("docs/adr/ADR-2-b.md", "Status: Proposed");
    write("docs/specs/SPEC-1-s.md", "Status: Shipped");
    write(
        "docs/milestones/MILE-1-m.md",
        "Status: Planned\nImplements: ADR-1",
    );
    write("docs/bugs/BUG-1-p.md", "Status: Fixed");
    std::fs::write(dir.join("rust/crates/x/src/lib.rs"), "fn a() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    edit(&dir, "rust/crates/x/src/lib.rs");

    let count = |dir: &Path| findings(&check(dir, &["--base", "base"]).1, R124).len();
    assert_eq!(count(&dir), 1, "a source change alone is held");
    edit(&dir, "docs/adr/ADR-2-b.md");
    assert_eq!(count(&dir), 1, "a Proposed ADR does not release it");
    for released_by in [
        "docs/adr/ADR-1-a.md",
        "docs/specs/SPEC-1-s.md",
        "docs/milestones/MILE-1-m.md",
        "docs/bugs/BUG-1-p.md",
    ] {
        git(&dir, &["checkout", "-q", "--", "docs"]);
        edit(&dir, released_by);
        assert_eq!(count(&dir), 0, "{released_by} releases the gate");
    }
}
