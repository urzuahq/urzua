//! `boundary.published-path-reference` (`MILE-125`, `RFC-11`) against the real
//! binary: a record identifier cited from inside a declared published path is
//! a finding, at the severity the declaration gives it, and a run that examined
//! nothing says so.

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

fn fixture_repo(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("urzua-boundary-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    dir
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

/// A repository with one ADR record (so the run has a corpus to examine) and
/// the boundary rule declared at `level` over `published`.
fn repo_with_boundary(name: &str, level: &str, published: &str) -> std::path::PathBuf {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n\
             \x20 boundary.published-path-reference:\n    level: {level}\n    published_paths: {published}\n\n\
             record_types:\n\
             \x20 adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    dir
}

fn report(out: &std::process::Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not valid JSON: {e}\nstdout: {stdout}"))
}

fn boundary_execution(parsed: &serde_json::Value) -> serde_json::Value {
    parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "boundary.published-path-reference")
        .unwrap_or_else(|| panic!("the rule must appear in rules_executed: {parsed}"))
        .clone()
}

fn boundary_findings(parsed: &serde_json::Value) -> Vec<serde_json::Value> {
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "boundary.published-path-reference")
        .cloned()
        .collect()
}

#[test]
fn a_citation_inside_a_published_path_blocks_and_names_file_and_line() {
    let dir = repo_with_boundary("cited", "error", "[\"pkg/public\"]");
    std::fs::create_dir_all(dir.join("pkg/public")).unwrap();
    std::fs::write(
        dir.join("pkg/public/lib.rs"),
        "fn a() {}\n// narrow on purpose, see ADR-1 for why\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    let findings = boundary_findings(&parsed);
    assert_eq!(findings.len(), 1, "{parsed}");
    assert_eq!(findings[0]["file"], "pkg/public/lib.rs");
    assert_eq!(findings[0]["line"], 2);
    assert_eq!(findings[0]["severity"], "error");
    assert_eq!(parsed["blocking"], true, "{parsed}");
    assert_eq!(out.status.code(), Some(1), "{parsed}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_same_citation_outside_every_published_path_is_not_reported() {
    let dir = repo_with_boundary("internal", "error", "[\"pkg/public\"]");
    std::fs::create_dir_all(dir.join("pkg/public")).unwrap();
    std::fs::create_dir_all(dir.join("pkg/internal")).unwrap();
    std::fs::create_dir_all(dir.join("pkg/public-notes")).unwrap();
    std::fs::write(dir.join("pkg/public/lib.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("pkg/internal/lib.rs"), "// see ADR-1\n").unwrap();
    std::fs::write(dir.join("pkg/public-notes/n.md"), "ADR-1\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    assert!(boundary_findings(&parsed).is_empty(), "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_clean_run_reports_how_many_files_it_looked_at() {
    let dir = repo_with_boundary("clean", "error", "[\"pkg/public\"]");
    std::fs::create_dir_all(dir.join("pkg/public/sub")).unwrap();
    std::fs::write(dir.join("pkg/public/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("pkg/public/sub/b.rs"), "fn b() {}\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    let exec = boundary_execution(&parsed);
    assert_eq!(exec["status"], "ran", "{parsed}");
    assert_eq!(exec["population"]["unit"], "file", "{parsed}");
    assert_eq!(exec["population"]["eligible"], 2, "{parsed}");
    assert_eq!(exec["population"]["examined"], 2, "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_declared_level_is_the_boundary_severity() {
    let dir = repo_with_boundary("warn", "warn", "[\"pkg/public\"]");
    std::fs::create_dir_all(dir.join("pkg/public")).unwrap();
    std::fs::write(dir.join("pkg/public/lib.rs"), "// ADR-1\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    let findings = boundary_findings(&parsed);
    assert_eq!(findings.len(), 1, "{parsed}");
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(parsed["blocking"], false, "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_published_path_covering_no_tracked_file_fails_closed() {
    let dir = repo_with_boundary("empty", "error", "[\"pkg/renamed-away\"]");
    std::fs::create_dir_all(dir.join("pkg/public")).unwrap();
    std::fs::write(dir.join("pkg/public/lib.rs"), "// ADR-1\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    let findings = boundary_findings(&parsed);
    assert_eq!(findings.len(), 1, "{parsed}");
    assert_eq!(findings[0]["file"], "pkg/renamed-away");
    assert!(findings[0]["message"]
        .as_str()
        .unwrap()
        .contains("no tracked file"));
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    assert_eq!(boundary_execution(&parsed)["population"]["examined"], 0);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_binary_file_is_counted_unreadable_not_clean() {
    let dir = repo_with_boundary("binary", "error", "[\"pkg/public\"]");
    std::fs::create_dir_all(dir.join("pkg/public")).unwrap();
    std::fs::write(dir.join("pkg/public/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(
        dir.join("pkg/public/logo.png"),
        [0x89u8, 0x50, 0xff, 0xfe, 0x00],
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    let pop = &boundary_execution(&parsed)["population"];
    assert_eq!(pop["eligible"], 2, "{parsed}");
    assert_eq!(pop["examined"], 1, "{parsed}");
    assert_eq!(pop["unreadable"], 1, "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_tracked_file_deleted_from_disk_is_not_a_crash_and_not_examined() {
    let dir = repo_with_boundary("deleted", "error", "[\"pkg/public\"]");
    std::fs::create_dir_all(dir.join("pkg/public")).unwrap();
    std::fs::write(dir.join("pkg/public/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("pkg/public/gone.rs"), "// ADR-1\n").unwrap();
    commit_all(&dir);
    std::fs::remove_file(dir.join("pkg/public/gone.rs")).unwrap();

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    assert!(boundary_findings(&parsed).is_empty(), "{parsed}");
    assert_eq!(boundary_execution(&parsed)["population"]["examined"], 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_rule_does_not_load_without_published_paths() {
    let dir = fixture_repo("no-option");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n  boundary.published-path-reference: error\n\n\
         record_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_ne!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("published_paths"), "{stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_undeclared_rule_is_reported_not_enabled() {
    let dir = fixture_repo("undeclared");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n  header.required-fields: error\n\n\
         record_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    assert_eq!(boundary_execution(&parsed)["status"], "not-enabled");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_dot_slash_entry_covers_its_files_and_names_the_normalised_path() {
    let dir = repo_with_boundary("dotslash", "error", "[\"./pkg/sdk/\"]");
    std::fs::create_dir_all(dir.join("pkg/sdk")).unwrap();
    std::fs::write(dir.join("pkg/sdk/a.txt"), "see ADR-1\n").unwrap();
    commit_all(&dir);

    let parsed = report(&run_urzua(&dir, &["check"]));
    let findings = boundary_findings(&parsed);
    assert_eq!(findings.len(), 1, "{parsed}");
    assert_eq!(findings[0]["file"], "pkg/sdk/a.txt");
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("'pkg/sdk'"),
        "{parsed}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A repository declaring `rule_body` (the boundary rule's options, indented
/// four spaces) at rule `level`.
fn repo_with_rule(name: &str, level: &str, rule_body: &str) -> std::path::PathBuf {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  boundary.published-path-reference:\n    level: {level}\n{rule_body}\n\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    dir
}

fn boundary_yaml(kind: &str, severity: &str, paths: &str, extra: &str) -> String {
    format!(
        "    boundaries:\n      - paths: {paths}\n        kind: {kind}\n        reason: \"a test boundary\"\n        severity: {severity}\n{extra}"
    )
}

#[test]
fn a_boundarys_severity_sets_its_findings_level_and_the_rules_level_only_enables_it() {
    // The rule is at `warn`; the boundary says `error`: the finding is an error.
    let dir = repo_with_rule(
        "sev-error",
        "warn",
        &boundary_yaml("published", "error", "[\"pkg\"]", ""),
    );
    std::fs::create_dir_all(dir.join("pkg")).unwrap();
    std::fs::write(dir.join("pkg/a.rs"), "// see ADR-1\n").unwrap();
    commit_all(&dir);
    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    assert_eq!(
        boundary_findings(&parsed)[0]["severity"],
        "error",
        "{parsed}"
    );
    assert_eq!(parsed["blocking"], true, "{parsed}");
    std::fs::remove_dir_all(&dir).ok();

    // The rule is at `error`; the boundary says `warn`: the finding is a warning.
    let dir = repo_with_rule(
        "sev-warn",
        "error",
        &boundary_yaml("published", "warn", "[\"pkg\"]", ""),
    );
    std::fs::create_dir_all(dir.join("pkg")).unwrap();
    std::fs::write(dir.join("pkg/a.rs"), "// see ADR-1\n").unwrap();
    commit_all(&dir);
    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    assert_eq!(
        boundary_findings(&parsed)[0]["severity"],
        "warning",
        "{parsed}"
    );
    assert_eq!(parsed["blocking"], false, "{parsed}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_generated_boundary_reports_a_back_pointer_and_not_a_plain_record_id() {
    let dir = repo_with_rule(
        "generated",
        "error",
        &boundary_yaml("generated", "error", "[\"gen\"]", ""),
    );
    std::fs::create_dir_all(dir.join("gen")).unwrap();
    std::fs::write(
        dir.join("gen/table.md"),
        "ADR-1 is cited in prose here\n// Implements: ADR-1\n",
    )
    .unwrap();
    commit_all(&dir);
    let parsed = report(&run_urzua(&dir, &["check"]));
    let findings = boundary_findings(&parsed);
    assert_eq!(findings.len(), 1, "{parsed}");
    assert_eq!(findings[0]["line"], 2);
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("generated"),
        "{parsed}"
    );
    assert!(findings[0].get("suggested_action").is_none(), "{parsed}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_vendored_boundary_is_declared_and_prohibits_nothing() {
    let dir = repo_with_rule(
        "vendored",
        "error",
        &boundary_yaml("vendored", "error", "[\"vendor\"]", ""),
    );
    std::fs::create_dir_all(dir.join("vendor")).unwrap();
    std::fs::write(dir.join("vendor/a.rs"), "// Implements: ADR-1, see ADR-1\n").unwrap();
    commit_all(&dir);
    let out = run_urzua(&dir, &["check"]);
    let parsed = report(&out);
    assert!(boundary_findings(&parsed).is_empty(), "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_exclude_excuses_a_fixture_directory_and_an_unused_one_is_reported() {
    let dir = repo_with_rule(
        "exclude",
        "error",
        &boundary_yaml(
            "published",
            "error",
            "[\"pkg\"]",
            "        exclude: [\"pkg/fixtures\", \"pkg/gone\"]\n",
        ),
    );
    std::fs::create_dir_all(dir.join("pkg/fixtures")).unwrap();
    std::fs::write(dir.join("pkg/a.rs"), "clean\n").unwrap();
    std::fs::write(dir.join("pkg/fixtures/f.rs"), "// ADR-1 on purpose\n").unwrap();
    commit_all(&dir);
    let parsed = report(&run_urzua(&dir, &["check"]));
    let findings = boundary_findings(&parsed);
    assert_eq!(findings.len(), 1, "{parsed}");
    assert_eq!(findings[0]["file"], "pkg/gone");
    assert_eq!(
        findings[0]["suggested_action"]["kind"], "edit-config",
        "{parsed}"
    );
    assert_eq!(findings[0]["suggested_action"]["op"], "remove", "{parsed}");
    assert_eq!(
        findings[0]["suggested_action"]["target"]["key_path"],
        serde_json::json!([
            "rules",
            "boundary.published-path-reference",
            "boundaries",
            "0",
            "exclude"
        ]),
        "{parsed}"
    );
    assert_eq!(
        findings[0]["suggested_action"]["target"]["file"], ".urzua/config.yaml",
        "{parsed}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_declared_marker_excuses_its_line_and_a_stale_marker_is_reported() {
    let dir = repo_with_rule(
        "marker",
        "error",
        &format!(
            "{}    ignore_marker: \"record-id-ok\"\n",
            boundary_yaml("published", "error", "[\"pkg\"]", "")
        ),
    );
    std::fs::create_dir_all(dir.join("pkg")).unwrap();
    std::fs::write(
        dir.join("pkg/a.rs"),
        "// ADR-1 record-id-ok\n// ADR-1 here is not excused\nlet a = 1; // record-id-ok\n",
    )
    .unwrap();
    commit_all(&dir);
    let parsed = report(&run_urzua(&dir, &["check"]));
    let mut lines: Vec<i64> = boundary_findings(&parsed)
        .iter()
        .map(|f| f["line"].as_i64().unwrap())
        .collect();
    lines.sort();
    assert_eq!(lines, [2, 3], "{parsed}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_glob_boundary_and_the_compatible_key_apply_together() {
    let dir = repo_with_rule(
        "both",
        "error",
        &format!(
            "{}    published_paths: [\"old\"]\n",
            boundary_yaml("published", "warn", "[\"packages/*/src\"]", "")
        ),
    );
    for d in ["packages/a/src", "packages/a/tests", "old"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join("packages/a/src/x.rs"), "// ADR-1\n").unwrap();
    std::fs::write(dir.join("packages/a/tests/y.rs"), "// ADR-1\n").unwrap();
    std::fs::write(dir.join("old/z.rs"), "// ADR-1\n").unwrap();
    commit_all(&dir);
    let parsed = report(&run_urzua(&dir, &["check"]));
    let mut seen: Vec<(String, String)> = boundary_findings(&parsed)
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_string(),
                f["severity"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        [
            ("old/z.rs".to_string(), "error".to_string()),
            ("packages/a/src/x.rs".to_string(), "warning".to_string())
        ],
        "{parsed}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_boundary_without_a_reason_does_not_load() {
    let dir = repo_with_rule(
        "no-reason",
        "error",
        "    boundaries:\n      - paths: [\"pkg\"]\n        kind: published\n        severity: error\n",
    );
    commit_all(&dir);
    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_ne!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("reason"), "{stdout}");
    std::fs::remove_dir_all(&dir).ok();
}
