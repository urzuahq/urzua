//! `urzua init` (SPEC-5): the files-touched criterion and the withheld-rule
//! mechanism. The dry-run equivalence criterion is held by
//! `init_adopt_integration.rs`.
//!
//! Verifies: SPEC-5

mod common;

use std::path::Path;

fn adopt_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-init-{name}"));
    std::fs::create_dir_all(dir.path().join("docs/adr")).unwrap();
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.path().join("docs/adr/ADR-0001-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn init(dir: &Path) -> String {
    let out = common::urzua()
        .arg("init")
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    stdout
}

fn porcelain(dir: &Path) -> String {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(dir)
        .output()
        .expect("git should run");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// SPEC-5 criterion 2: only `.urzua/` appears, `.gitignore` is untouched.
#[test]
fn init_changes_only_the_urzua_directory_and_leaves_gitignore_alone() {
    let dir = adopt_repo("files-touched");
    init(dir.path());
    assert_eq!(porcelain(dir.path()), "?? .urzua/config.yaml");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        "target/\n"
    );
}

/// SPEC-5 "Proposed rules": every rule whose table entry has a
/// `reads_declaration` is withheld from the config and named in the notice,
/// with no list kept in `init`.
#[test]
fn init_withholds_every_rule_that_reads_a_declaration() {
    let dir = adopt_repo("withholds");
    let stdout = init(dir.path());
    let config = std::fs::read_to_string(dir.path().join(".urzua/config.yaml")).unwrap();
    let mut seen = 0;
    for def in urzua_core::rules::RULE_TABLE {
        let Some(declaration) = def.reads_declaration else {
            continue;
        };
        seen += 1;
        assert!(
            !config.contains(def.id),
            "{} was proposed:\n{config}",
            def.id
        );
        assert!(stdout.contains(def.id), "notice omits {}: {stdout}", def.id);
        assert!(
            stdout.contains(declaration.key()),
            "notice omits {}: {stdout}",
            declaration.key()
        );
    }
    assert!(
        seen >= 5,
        "expected at least five declaration-reading rules, saw {seen}"
    );
}

/// Review finding 6.2: every rule adopt proposes examines a record of the corpus
/// it was proposed for. Fifteen proposed rules reported `ran` over zero records
/// (`header.required-fields`, `pointer.resolution`, `status.delivery-agreement`,
/// `waiver.matches-nothing` among them) because each reads a declaration adopt
/// never writes. Two types so a per-type reading cannot hide a gap.
#[test]
fn every_rule_adopt_proposes_examines_a_record() {
    let dir = adopt_repo("zero-examined");
    std::fs::create_dir_all(dir.path().join("docs/rfc")).unwrap();
    for n in 1..=2 {
        std::fs::write(
            dir.path().join(format!("docs/rfc/RFC-000{n}-y.md")),
            "---\nStatus: Draft\n---\n# RFC\n",
        )
        .unwrap();
    }
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "more"]);
    init(dir.path());
    common::git(&dir, &["add", "-A"]);
    let out = common::urzua()
        .arg("check")
        .current_dir(dir.path())
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let idle: Vec<String> = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["status"] == "ran" && r["population"]["examined"] == 0)
        .map(|r| r["rule"].as_str().unwrap().to_string())
        // A rule on for every configuration is not one init proposes; here it
        // finds no evidence pin to examine.
        .filter(|rule| urzua_core::rules::default_on(rule).is_none())
        .collect();
    assert!(idle.is_empty(), "ran over zero records: {idle:?}");
}

fn empty_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-init-{name}"));
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    std::fs::write(dir.path().join("README.md"), "# project\n").unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn run_json(dir: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let value = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    (out.status.code(), value)
}

/// MILE-265: in a repository with no record directory, `init` said "nothing to
/// adopt" and never named `--types`.
#[test]
fn init_in_an_empty_repository_names_the_greenfield_command_observed_failing() {
    let dir = empty_repo("empty-names-types");
    let (code, report) = run_json(dir.path(), &["init"]);
    assert_eq!(code, Some(2), "{report}");
    let error = report["error"].as_str().unwrap();
    assert!(error.contains("nothing to adopt"), "{error}");
    assert!(error.contains("`urzua init --types adr`"), "{error}");
    assert!(error.contains("adr, rfc, spec, prd"), "{error}");
    assert_eq!(report["next_action"], "urzua init --types adr", "{report}");
    assert!(report["next_action_reason"].is_string(), "{report}");
}

/// MILE-265: `doctor` after an `init` that could not adopt sent the user back
/// to `urzua init`, a loop. It now names the step that works.
#[test]
fn doctor_after_a_refused_init_names_a_step_that_works_observed_failing() {
    let dir = empty_repo("doctor-after-refusal");
    let (code, _) = run_json(dir.path(), &["init"]);
    assert_eq!(code, Some(2));
    let (code, report) = run_json(dir.path(), &["doctor"]);
    assert_eq!(code, Some(2), "{report}");
    let message = report["checks"][0]["message"].as_str().unwrap();
    assert!(!message.contains("run `urzua init` to adopt"), "{message}");
    assert!(message.contains("urzua init --types adr"), "{message}");
    assert_eq!(report["next_action"], "urzua init --types adr", "{report}");
}

/// When adopt would propose a config, `doctor` says so and points at it.
#[test]
fn doctor_with_an_adoptable_corpus_names_init() {
    let dir = adopt_repo("doctor-adoptable");
    let (_, report) = run_json(dir.path(), &["doctor"]);
    let message = report["checks"][0]["message"].as_str().unwrap();
    assert!(message.contains("proposes 1 record type(s)"), "{message}");
    assert_eq!(report["next_action"], "urzua init", "{report}");
}
