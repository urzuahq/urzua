//! MILE-124 and MILE-142: the two diff-gate rules, `diff.source-requires-record`
//! and `diff.source-and-records-mixed`. Both judge the set of changed paths, so
//! they run only on a `--base` run and say `not-applicable` on any other,
//! never a quiet pass (SPEC-2, Rules).

use std::path::{Path, PathBuf};
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
fn fixture(name: &str, rules: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-diff-gate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
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
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
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
    assert_eq!(run["population"]["out_of_scope"], 2, "{run}");
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
    let dir = std::env::temp_dir().join(format!("urzua-diff-gate-own-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
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
        execution(&report, R124)["population"]["out_of_scope"],
        1,
        "{report}"
    );

    edit(&dir, "rust/crates/x/src/lib.rs");
    let (_, report) = check(&dir, &["--base", "base"]);
    let found = findings(&report, R124);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "rust/crates/x/src/lib.rs");
}
