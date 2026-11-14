//! The could-not-run output of every command that opens a corpus (MILE-173):
//! no repository, no config, and an unreadable tracked record each produce the
//! same `not-run` JSON on stdout and exit code 2, whichever command was asked.

mod common;

use std::path::Path;
use std::process::{Command, Output};

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n";

/// Every command that loads the corpus, as its argv.
const CORPUS_COMMANDS: &[&[&str]] = &[
    &["check"],
    &["audit"],
    &["explain", "docs/adr/ADR-1-x.md"],
    &["graph"],
    &["export", "--format", "json"],
    &["fix"],
    &["migrate", "ids"],
    &["migrate", "schema", "--report", "--field", "Status"],
];

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn scratch(name: &str) -> common::TempDir {
    common::TempDir::new(&format!("urzua-cnr-{name}"))
}

fn repo(name: &str) -> common::TempDir {
    let dir = scratch(name);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn assert_not_run(out: &Output, error: &str, what: &str) {
    assert_eq!(out.status.code(), Some(2), "{what}: {out:?}");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{what}: stdout is not JSON ({e}): {out:?}"));
    assert_eq!(
        json,
        serde_json::json!({ "status": "not-run", "error": error, "next_action": null, "next_action_reason": null }),
        "{what}"
    );
}

const NEEDS_A_GIT_REPOSITORY: &str =
    "urzua needs a git repository: no .git directory found above the given path";

#[test]
fn every_command_that_reads_a_repository_exits_2_outside_one_observed_failing() {
    // BUG-170: outside a git repository there is no fallback that walks the
    // directory (SPEC-2). Every command that reads a repository says so and
    // exits 2, even when a config and a corpus sit right there on disk.
    let dir = scratch("no-repo");
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# ADR-1 — x\n",
    )
    .unwrap();
    for args in CORPUS_COMMANDS.iter().copied().chain([
        &["doctor"][..],
        &["init"][..],
        &["new", "adr", "T"][..],
        &["index"][..],
        &["refs", "refresh"][..],
        &["check", "docs/adr/ADR-1-x.md"][..],
        &["check", "docs"][..],
    ]) {
        assert_not_run(&run(&dir, args), NEEDS_A_GIT_REPOSITORY, &args.join(" "));
    }
}

#[test]
fn rules_reads_no_repository_and_runs_outside_one() {
    let out = run(&scratch("rules-no-repo"), &["rules"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["rules"].as_array().is_some_and(|r| !r.is_empty()));
}

#[test]
fn every_corpus_command_reports_a_missing_config() {
    let dir = repo("no-config");
    let error = format!(
        "could not read config at {}/.urzua/config.yaml: No such file or directory (os error 2)",
        dir.display()
    );
    for args in CORPUS_COMMANDS
        .iter()
        .copied()
        .chain([&["new", "adr", "T"][..]])
    {
        assert_not_run(&run(&dir, args), &error, &args.join(" "));
    }
}

#[test]
fn doctor_reports_a_missing_config_as_its_own_report() {
    let dir = repo("doctor-no-config");
    let out = run(&dir, &["doctor"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["status"], "error");
    assert_eq!(json["checks"][0]["check"], "config-exists");
}

#[cfg(unix)]
#[test]
fn every_corpus_command_reports_an_unreadable_tracked_record() {
    let dir = repo("unreadable");
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    // A tracked record that is present but cannot be read as a file (a link to
    // a directory): unlike an absent one, which is judged as removed, it is
    // not a record that passes.
    std::os::unix::fs::symlink(".", dir.join("docs/adr/ADR-1-x.md")).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    let error =
        "could not read 1 tracked record(s): docs/adr/ADR-1-x.md: Is a directory (os error 21)";
    for args in CORPUS_COMMANDS {
        assert_not_run(&run(&dir, args), error, &args.join(" "));
    }
}

#[test]
fn every_corpus_command_refuses_a_non_canonical_dir_and_names_the_canonical_form() {
    // BUG-168: `dir: ./docs/adr` parsed, matched nothing and ran as `not-run`
    // with no reason. It is a load error that names the spelling to write.
    for bad in ["./docs/adr", "docs/adr/", "docs//adr"] {
        let dir = repo("non-canonical-dir");
        std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            CONFIG.replace("\"docs/adr\"", &format!("\"{bad}\"")),
        )
        .unwrap();
        std::fs::write(
            dir.join("docs/adr/ADR-1-x.md"),
            "---\nStatus: Accepted\n---\n# ADR-1 — x\n",
        )
        .unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "fixture"]);
        for args in CORPUS_COMMANDS {
            let out = run(&dir, args);
            assert_eq!(out.status.code(), Some(2), "{bad} {args:?}: {out:?}");
            let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(json["status"], "not-run", "{bad} {args:?}");
            let error = json["error"].as_str().unwrap_or_default();
            assert!(
                error.contains(&format!("declares dir '{bad}'"))
                    && error.contains("write 'docs/adr'"),
                "{bad} {args:?}: {error}"
            );
        }
    }
}
