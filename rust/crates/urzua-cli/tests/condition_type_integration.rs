//! MILE-187: the `condition` record type (`RFC-48`), declared in configuration
//! alone. A fixture proves the engine has no special case for the name, the
//! `Depends-on` edge into a condition is judged like any other, and this
//! repository's own corpus carries the four migrated waits.
//!
//! Verifies: MILE-187

mod common;

use std::path::Path;
use std::process::Command;

/// The type's declaration under a given name, prefix and directory. The shape is
/// the one this repository's `.urzua/config.yaml` declares; only the names vary.
fn type_block(name: &str, prefix: &str, dir: &str) -> String {
    format!(
        "  {name}:
    dir: {dir}
    prefix: {prefix}
    identity: {{from: filename, pattern: '^(?P<prefix>{prefix})-(?P<number>\\d{{4}})-(?P<slug>.+)$'}}
    required_fields: [Status]
    header_shape: yaml-frontmatter
    known_fields: [Stable-Id, Met-by]
    statuses:
      values: [Open, Met, Dropped]
      success: [Met]
      failure: [Dropped]
      initial: [Open]
      transitions:
        Open: [Met, Dropped]
      require:
        - {{when: [Met], fields: [Met-by]}}
    sections:
      from: headings
      required:
        - {{name: Signal}}
    pointer_fields: [Met-by]
    narrative_fields: []
"
    )
}

fn config(condition_type: &str) -> String {
    format!(
        "schema_version: 2
roadmap:
  pairs: [sequencing]
relation_pairs:
  - {{name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}}
rules:
  header.required-fields: error
  status.undeclared-value: error
  status.required-field-missing: error
  status.transition-allowed: error
  section.required: error
  pointer.resolution: error
  sequence.cycle: error
  sequence.unresolved-blocker: error
  relation.target-status-undeclared: error
record_types:
  milestone:
    dir: docs/milestones
    prefix: MILE
    required_fields: [Status, Phase]
    known_fields: [Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    narrative_fields: []
    statuses:
      values: [Planned, Done, WontDo]
      success: [Done]
      failure: [WontDo]
  note:
    dir: docs/notes
    prefix: NOTE
    required_fields: [Date]
    known_fields: []
    pointer_fields: []
    narrative_fields: []
{condition_type}"
    )
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .expect("git should run")
        .success());
}

fn write(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, text).unwrap();
}

fn cond_file(prefix: &str, n: u32, status: &str, extra: &str) -> String {
    format!("---\nStatus: {status}\n{extra}---\n# {prefix}-{n} — c\n\n## Signal\n\nThe signal.\n")
}

fn milestone(n: u32, depends: &str) -> String {
    format!("---\nStatus: Planned\nPhase: '0'\nDepends-on: {depends}\n---\n# {n} — m\n")
}

/// A repository with the condition type under `name`/`prefix`/`dir`, one waiting
/// milestone per scenario: 1 on an Open condition, 2 on a Met one, 3 on a Dropped
/// one, 4 on one that does not exist, 5 on a note (a type declaring no statuses).
fn scenario(name: &str, prefix: &str, dir: &str) -> common::TempDir {
    let repo = common::TempDir::new(&format!("urzua-cond-{name}"));
    write(
        &repo,
        ".urzua/config.yaml",
        &config(&type_block(name, prefix, dir)),
    );
    write(
        &repo,
        &format!("{dir}/{prefix}-0001-open.md"),
        &cond_file(prefix, 1, "Open", ""),
    );
    write(
        &repo,
        &format!("{dir}/{prefix}-0002-met.md"),
        &cond_file(prefix, 2, "Met", "Met-by: MILE-1\n"),
    );
    write(
        &repo,
        &format!("{dir}/{prefix}-0003-dropped.md"),
        &cond_file(prefix, 3, "Dropped", ""),
    );
    write(
        &repo,
        "docs/notes/NOTE-1-n.md",
        "---\nDate: 2026-11-05\n---\n# NOTE-1 — n\n",
    );
    for (n, target) in [
        (1, format!("{prefix}-1")),
        (2, format!("{prefix}-2")),
        (3, format!("{prefix}-3")),
        (4, format!("{prefix}-9")),
        (5, "NOTE-1".to_string()),
    ] {
        write(
            &repo,
            &format!("docs/milestones/MILE-{n}-m.md"),
            &milestone(n, &target),
        );
    }
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["add", "-A"]);
    repo
}

fn urzua(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(&out.stdout)))
}

/// `(rule, file name)` of every finding, sorted, with the type's own names
/// replaced so two spellings of one shape can be compared.
fn findings(report: &serde_json::Value, prefix: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["rule"].as_str().unwrap().to_string(),
                f["file"]
                    .as_str()
                    .unwrap()
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .replace(prefix, "X"),
            )
        })
        .collect();
    out.sort();
    out
}

fn on<'a>(found: &'a [(String, String)], rule: &str) -> Vec<&'a str> {
    found
        .iter()
        .filter(|(r, _)| r == rule)
        .map(|(_, f)| f.as_str())
        .collect()
}

#[test]
fn a_dependency_on_a_condition_is_judged_like_any_other_edge() {
    let repo = scenario("condition", "COND", "docs/conditions");
    let report = urzua(&repo, &["check", "docs/"]);
    let found = findings(&report, "COND");
    // The missing condition is reported once, by the pointer rule that owns a
    // reference naming nothing; the sequence rules examine every edge besides.
    assert_eq!(
        on(&found, "pointer.resolution"),
        ["MILE-4-m.md"],
        "only the missing condition is unresolved: {report}"
    );
    for rule in ["sequence.cycle", "sequence.unresolved-blocker"] {
        let ran = report["rules_executed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule"] == rule)
            .unwrap();
        assert_eq!(ran["status"], "ran", "{rule}");
        assert_eq!(
            ran["population"]["examined"], 5,
            "{rule} examined the five Depends-on edges, the conditions' included"
        );
    }
    assert_eq!(
        on(&found, "relation.target-status-undeclared"),
        ["MILE-5-m.md"],
        "a target type declaring no status is reported, not read as satisfied: {report}"
    );
    assert!(
        on(&found, "sequence.cycle").is_empty()
            && on(&found, "status.required-field-missing").is_empty()
            && on(&found, "section.required").is_empty(),
        "{found:?}"
    );
}

#[test]
fn the_roadmap_blocks_on_open_keeps_dropped_out_and_frees_met() {
    let repo = scenario("condition", "COND", "docs/conditions");
    let r = urzua(&repo, &["roadmap"])["roadmap"].clone();
    let rec = |id: &str| r["records"][id].clone();
    assert_eq!(rec("MILE-1")["blocked"], true);
    assert_eq!(rec("MILE-1")["ready"], false);
    assert_eq!(rec("MILE-1")["blocked_by"], serde_json::json!(["COND-1"]));
    assert_eq!(
        rec("MILE-2")["blocked"],
        false,
        "a Met condition does not block"
    );
    assert_eq!(rec("MILE-2")["ready"], true);
    assert_eq!(
        rec("MILE-3")["ready"],
        false,
        "a Dropped condition keeps it out"
    );
    assert_eq!(rec("MILE-3")["blocked"], true);
    let ready: Vec<&str> = r["ready"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        ready.contains(&"COND-1"),
        "an Open condition is itself open work: {ready:?}"
    );
    assert!(ready.contains(&"MILE-2") && !ready.contains(&"MILE-1") && !ready.contains(&"MILE-3"));
    assert!(
        r["needs_replan"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["record"] == "MILE-3" && n["reason"] == "failed-dependency"),
        "{r}"
    );
}

#[test]
fn any_type_declaring_depends_on_resolves_to_a_condition_not_only_milestones() {
    // This repository's adr and rfc types declare no Depends-on, so its own
    // corpus cannot prove the generic claim. A decision type that does declare it
    // can: the edge resolves to a condition and the roadmap reads it the same.
    let repo = scenario("condition", "COND", "docs/conditions");
    let cfg = repo.join(".urzua/config.yaml");
    let mut text = std::fs::read_to_string(&cfg).unwrap();
    text.push_str(
        "  decision:
    dir: docs/decisions
    prefix: DEC
    required_fields: [Status]
    known_fields: [Depends-on, Blocks]
    pointer_fields: [Depends-on, Blocks]
    narrative_fields: []
    statuses:
      values: [Proposed, Accepted, Rejected]
      success: [Accepted]
      failure: [Rejected]
",
    );
    std::fs::write(&cfg, text).unwrap();
    for (n, target) in [(1, "COND-1"), (2, "COND-2"), (3, "COND-3"), (4, "COND-9")] {
        write(
            &repo,
            &format!("docs/decisions/DEC-{n}-d.md"),
            &format!("---\nStatus: Proposed\nDepends-on: {target}\n---\n# DEC-{n} — d\n"),
        );
    }
    git(&repo, &["add", "-A"]);
    let report = urzua(&repo, &["check", "docs/"]);
    let found = findings(&report, "COND");
    assert_eq!(
        on(&found, "pointer.resolution"),
        ["DEC-4-d.md", "MILE-4-m.md"],
        "only the missing condition is unresolved, from either type: {report}"
    );
    let r = urzua(&repo, &["roadmap"])["roadmap"].clone();
    let rec = |id: &str| r["records"][id].clone();
    assert_eq!(rec("DEC-1")["blocked_by"], serde_json::json!(["COND-1"]));
    assert_eq!(rec("DEC-1")["ready"], false);
    assert_eq!(rec("DEC-2")["blocked"], false);
    assert_eq!(rec("DEC-2")["ready"], true);
    assert_eq!(rec("DEC-3")["ready"], false);
    assert!(
        r["needs_replan"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["record"] == "DEC-3" && n["reason"] == "failed-dependency"),
        "{r}"
    );
}

#[test]
fn the_engine_has_no_special_case_for_the_name_condition() {
    let named = scenario("condition", "COND", "docs/conditions");
    let other = scenario("fact", "FACT", "docs/facts");
    let a = findings(&urzua(&named, &["check", "docs/"]), "COND");
    let b = findings(&urzua(&other, &["check", "docs/"]), "FACT");
    assert_eq!(
        a, b,
        "the same shape under another name reports the same findings"
    );
    let flags = |dir: &Path, prefix: &str| -> Vec<(String, bool, bool)> {
        let r = urzua(dir, &["roadmap"])["roadmap"].clone();
        let mut v: Vec<_> = r["records"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                (
                    k.replace(prefix, "X"),
                    v["blocked"].as_bool().unwrap(),
                    v["ready"].as_bool().unwrap(),
                )
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(flags(&named, "COND"), flags(&other, "FACT"));
}

#[test]
fn met_needs_its_evidence_and_the_lifecycle_is_enforced() {
    let repo = scenario("condition", "COND", "docs/conditions");
    git(&repo, &["commit", "-q", "-m", "base"]);
    // Met without Met-by; a Met condition moved back to Open; a condition filed already Met.
    write(
        &repo,
        "docs/conditions/COND-0004-bare.md",
        &cond_file("COND", 4, "Met", ""),
    );
    write(
        &repo,
        "docs/conditions/COND-0002-met.md",
        &cond_file("COND", 2, "Open", ""),
    );
    git(&repo, &["add", "-A"]);
    let report = urzua(&repo, &["check", "--base", "HEAD"]);
    let found = findings(&report, "COND");
    assert_eq!(
        on(&found, "status.required-field-missing"),
        ["X-0004-bare.md"],
        "{report}"
    );
    let transitions = on(&found, "status.transition-allowed");
    assert!(
        transitions.contains(&"X-0002-met.md") && transitions.contains(&"X-0004-bare.md"),
        "Met is terminal and Open is the only initial state: {report}"
    );
}

// ---- this repository's own configuration and corpus

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn header(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .skip(1)
        .take_while(|l| *l != "---")
        .find_map(|l| l.strip_prefix(&format!("{key}: ")).map(str::to_string))
}

fn milestone_path(n: u32) -> std::path::PathBuf {
    let dir = root().join("docs/milestones");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("MILE-{n}-"))
        })
        .unwrap_or_else(|| panic!("MILE-{n} not found"))
}

fn condition_path(id: &str) -> std::path::PathBuf {
    let n: u32 = id.trim_start_matches("COND-").parse().unwrap();
    let prefix = format!("COND-{n:04}-");
    std::fs::read_dir(root().join("docs/conditions"))
        .unwrap_or_else(|e| panic!("docs/conditions: {e}"))
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&prefix)
        })
        .unwrap_or_else(|| panic!("{id} not found"))
}

#[test]
fn the_four_waits_are_conditions_the_roadmap_reads() {
    let roadmap = urzua(&root(), &["roadmap"])["roadmap"].clone();
    for n in [13, 21, 23, 30] {
        let mile = milestone_path(n);
        let deps = header(&mile, "Depends-on").unwrap_or_default();
        let conds: Vec<&str> = deps
            .split(',')
            .map(str::trim)
            .filter(|d| d.starts_with("COND-"))
            .collect();
        assert_eq!(
            conds.len(),
            1,
            "MILE-{n} depends on exactly one condition: {deps:?}"
        );
        let cond = condition_path(conds[0]);
        let text = std::fs::read_to_string(&cond).unwrap();
        assert!(text.contains("\n## Signal\n"), "{cond:?} states its Signal");
        let status = header(&cond, "Status").unwrap();
        let rec = &roadmap["records"][format!("MILE-{n}")];
        if status == "Open" {
            assert_eq!(
                rec["blocked"], true,
                "MILE-{n} waits on an Open condition: {rec}"
            );
            assert_eq!(rec["blocked_by"], serde_json::json!([conds[0]]));
            assert_eq!(rec["ready"], false);
        } else {
            assert_eq!(
                status, "Met",
                "a person moves a condition to Met or Dropped"
            );
        }
    }
}

#[test]
fn the_own_corpus_checks_clean_and_no_prose_wait_is_left_to_report() {
    let report = urzua(&root(), &["check"]);
    let unresolved: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "sequence.unresolved-blocker")
        .collect();
    assert!(unresolved.is_empty(), "{unresolved:?}");
}

#[test]
fn the_own_config_declares_the_type_and_holds_met_to_its_evidence() {
    let repo = common::TempDir::new("urzua-cond-own");
    std::fs::create_dir_all(repo.join(".urzua")).unwrap();
    std::fs::copy(
        root().join(".urzua/config.yaml"),
        repo.join(".urzua/config.yaml"),
    )
    .unwrap();
    write(
        &repo,
        "docs/conditions/COND-0001-open.md",
        &cond_file("COND", 1, "Open", ""),
    );
    write(
        &repo,
        "docs/conditions/COND-0002-bare.md",
        &cond_file("COND", 2, "Met", ""),
    );
    write(
        &repo,
        "docs/conditions/COND-0003-met.md",
        &cond_file("COND", 3, "Met", "Met-by: COND-1\n"),
    );
    write(
        &repo,
        "docs/conditions/COND-0004-odd.md",
        &cond_file("COND", 4, "Waiting", ""),
    );
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "-A"]);
    let report = urzua(&repo, &["check", "docs/conditions"]);
    let found = findings(&report, "COND");
    assert_eq!(
        on(&found, "status.required-field-missing"),
        ["X-0002-bare.md"],
        "{report}"
    );
    assert_eq!(
        on(&found, "status.undeclared-value"),
        ["X-0004-odd.md"],
        "{report}"
    );
    assert!(
        on(&found, "type.no-declared-spec").is_empty(),
        "the type names its spec: {report}"
    );
}
