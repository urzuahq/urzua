//! `code.dangling-bug-reference` (`MILE-108`, `RFC-41`) against the real binary:
//! a source comment citing a record id that resolves to no record is reported,
//! and a comment citing a live one is not.
//!
//! Verifies: MILE-108

mod common;

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

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn report(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("json")
}

const RULE: &str = "code.dangling-bug-reference";

fn findings(out: &std::process::Output, rule: &str) -> Vec<serde_json::Value> {
    report(out)["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == rule)
        .cloned()
        .collect()
}

/// A repository with BUG-1 (Fixed), ADR-2 (Superseded) and a `src/` tree of
/// the given `(name, text)` files. `rules` is the YAML under `rules:`.
fn repo(name: &str, rules: &str, files: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-dangling-{name}"));
    for d in ["docs/adr", "docs/bugs", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n{rules}\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n  \
             bug:\n    dir: \"docs/bugs\"\n    prefix: \"BUG\"\n    \
             required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Fixed\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Superseded\n---\n# 2 — Y\n",
    )
    .unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn rule(extra: &str) -> String {
    format!(
        "  {RULE}:\n    level: error\n    back_pointer_paths: [\"src\"]\n    extensions: [\"rs\"]\n{extra}"
    )
}

fn population(out: &std::process::Output) -> serde_json::Value {
    report(out)["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == RULE)
        .unwrap_or_else(|| panic!("{RULE} did not run: {}", report(out)))["population"]
        .clone()
}

#[test]
fn a_dangling_citation_is_reported_with_its_line_and_a_live_one_is_not() {
    let dir = repo(
        "basic",
        &rule(""),
        &[(
            "src/lib.rs",
            "// fixed by BUG-1\nfn a() {}\n/// see BUG-9, which was deleted\nfn b() {} // and ADR-2\n",
        )],
    );
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, RULE);
    assert_eq!(found.len(), 1, "{}", report(&out));
    assert_eq!(found[0]["file"], "src/lib.rs");
    assert_eq!(found[0]["line"], 3);
    assert!(found[0]["message"].as_str().unwrap().contains("BUG-9"));
    assert_eq!(out.status.code(), Some(1), "{}", report(&out));
    // The population says what was read.
    let p = population(&out);
    assert_eq!(p["eligible"], 1, "{p}");
    assert_eq!(p["examined"], 1, "{p}");
}

#[test]
fn only_existence_is_judged_so_a_superseded_record_is_a_live_citation() {
    let dir = repo("superseded", &rule(""), &[("src/lib.rs", "// per ADR-2\n")]);
    let out = run_urzua(&dir, &["check"]);
    assert!(findings(&out, RULE).is_empty(), "{}", report(&out));
}

#[test]
fn ids_compare_numerically_so_zero_padding_does_not_dangle() {
    let dir = repo("padded", &rule(""), &[("src/lib.rs", "// see BUG-0001\n")]);
    let out = run_urzua(&dir, &["check"]);
    assert!(findings(&out, RULE).is_empty(), "{}", report(&out));
}

#[test]
fn only_files_with_a_declared_extension_are_read() {
    let dir = repo(
        "extensions",
        &rule(""),
        &[
            ("src/lib.rs", "// fine, BUG-1\n"),
            ("src/notes.txt", "// BUG-77 dangles but is not read\n"),
        ],
    );
    let out = run_urzua(&dir, &["check"]);
    assert!(findings(&out, RULE).is_empty(), "{}", report(&out));
    assert_eq!(population(&out)["eligible"], 1);
}

#[test]
fn only_comments_are_read_and_an_id_run_into_a_name_is_not_a_citation() {
    let dir = repo(
        "not-comments",
        &rule(""),
        &[(
            "src/lib.rs",
            "const A: &str = \"BUG-77\";\nfn b() {}\n// docs/bugs/BUG-78-title.md and BUG-79x\n",
        )],
    );
    let out = run_urzua(&dir, &["check"]);
    assert!(findings(&out, RULE).is_empty(), "{}", report(&out));
}

#[test]
fn a_trailing_comment_and_a_block_comment_line_are_read() {
    let dir = repo(
        "trailing",
        &rule(""),
        &[("src/lib.rs", "fn a() {} // BUG-10\n/*\n * BUG-11\n */\n")],
    );
    let out = run_urzua(&dir, &["check"]);
    let lines: Vec<u64> = findings(&out, RULE)
        .iter()
        .map(|f| f["line"].as_u64().unwrap())
        .collect();
    assert_eq!(lines, vec![1, 3], "{}", report(&out));
}

#[test]
fn an_undeclared_rule_reads_nothing() {
    let dir = repo("undeclared", "", &[("src/lib.rs", "// BUG-9\n")]);
    let out = run_urzua(&dir, &["check"]);
    assert!(findings(&out, RULE).is_empty());
    let status = report(&out)["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == RULE)
        .map(|e| e["status"].clone());
    assert_eq!(status, Some(serde_json::json!("not-enabled")));
}

#[test]
fn a_back_pointer_the_resolution_rule_judges_is_not_reported_twice() {
    let resolution =
        "  embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n";
    let source = "// Implements: BUG-9\n// plain mention of BUG-8\n";
    let both = repo(
        "both",
        &format!("{resolution}{}", rule("")),
        &[("src/lib.rs", source)],
    );
    let out = run_urzua(&both, &["check"]);
    let mine = findings(&out, RULE);
    assert_eq!(mine.len(), 1, "{}", report(&out));
    assert_eq!(mine[0]["line"], 2);
    assert_eq!(
        findings(&out, "embodiment.back-pointer-resolution").len(),
        1
    );

    // With the resolution rule off, the back-pointer line is this rule's.
    let alone = repo("alone", &rule(""), &[("src/lib.rs", source)]);
    let out = run_urzua(&alone, &["check"]);
    assert_eq!(findings(&out, RULE).len(), 2, "{}", report(&out));
}

#[test]
fn external_ids_are_not_records() {
    let dir = repo(
        "external",
        &rule("    external_ids: [\"ADR-4122\"]\n"),
        &[("src/lib.rs", "// ADR-4122 is a standard, ADR-4123 is not\n")],
    );
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, RULE);
    assert_eq!(found.len(), 1, "{}", report(&out));
    assert!(found[0]["message"].as_str().unwrap().contains("ADR-4123"));
}

#[test]
fn a_declared_marker_excuses_its_line_and_an_unused_marker_is_reported() {
    let dir = repo(
        "marker",
        &rule("    ignore_marker: \"record-id-ok\"\n"),
        &[(
            "src/lib.rs",
            "// BUG-9 record-id-ok\n// BUG-8\nfn a() {} // record-id-ok\n",
        )],
    );
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, RULE);
    let lines: Vec<u64> = found.iter().map(|f| f["line"].as_u64().unwrap()).collect();
    // Line 2 dangles; line 3 carries a marker and cites nothing.
    assert_eq!(lines, vec![2, 3], "{}", report(&out));
    assert!(found[1]["message"]
        .as_str()
        .unwrap()
        .contains("suppresses nothing"));
    assert_eq!(population(&out)["excused_lines"], 1);
    assert!(
        report(&out)["notices"].as_array().unwrap().iter().any(|n| {
            let m = n["message"].as_str().unwrap_or("");
            m.contains("ignore_marker") && m.contains("1 line")
        }),
        "{}",
        report(&out)
    );
}

#[test]
fn a_declared_path_with_no_file_of_a_declared_extension_is_a_finding() {
    let dir = repo("uncovered", &rule(""), &[("src/notes.txt", "// BUG-9\n")]);
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, RULE);
    assert_eq!(found.len(), 1, "{}", report(&out));
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("covers no tracked file"));
}

#[test]
fn the_rule_needs_extensions_and_refuses_one_written_with_a_dot() {
    for (extra, needle) in [
        ("", "extensions"),
        ("    extensions: [\".rs\"]\n", "without the dot"),
    ] {
        let rules =
            format!("  {RULE}:\n    level: error\n    back_pointer_paths: [\"src\"]\n{extra}");
        let dir = repo("config", &rules, &[("src/lib.rs", "// BUG-1\n")]);
        let out = run_urzua(&dir, &["check"]);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_ne!(out.status.code(), Some(0), "{text}");
        assert!(text.contains(needle), "{needle} missing: {text}");
    }
}

#[test]
fn a_rules_selection_that_leaves_it_out_does_not_read_the_source() {
    let dir = repo("selection", &rule(""), &[("src/lib.rs", "// BUG-9\n")]);
    let out = run_urzua(&dir, &["check", "--rules", RULE]);
    assert_eq!(findings(&out, RULE).len(), 1, "{}", report(&out));
}

#[test]
fn the_prefixes_come_from_the_declared_record_types() {
    let dir = repo(
        "prefixes",
        &rule(""),
        &[("src/lib.rs", "// BUG-1 and DEF-5 and QQ-6\n")],
    );
    // `DEF` is not declared: it is not an id, so it is not read.
    assert!(findings(&run_urzua(&dir, &["check"]), RULE).is_empty());

    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "{config}  defect:\n    dir: \"docs/defects\"\n    prefix: \"DEF\"\n    \
             required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("docs/defects")).unwrap();
    std::fs::write(dir.join("docs/defects/.gitkeep"), "").unwrap();
    git(&dir, &["add", "-A"]);
    let out = run_urzua(&dir, &["check"]);
    let found = findings(&out, RULE);
    assert_eq!(found.len(), 1, "{}", report(&out));
    assert!(found[0]["message"].as_str().unwrap().contains("DEF-5"));
}
