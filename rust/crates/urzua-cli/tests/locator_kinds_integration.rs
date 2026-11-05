//! BUG-176: which locator kinds a `Realized-by` may name, and which may carry a
//! pin, is configuration (`locator_kinds` and `pinnable` on
//! `embodiment.locator-exists`). Real git-repo fixtures, real binary; all
//! output is JSON.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git should run")
            .success(),
        "git {args:?} failed"
    );
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn json(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

const RECORD_PATH: &str = "docs/adr/ADR-1-first.md";

fn config(exists_options: &str) -> String {
    format!(
        "schema_version: 2\nrules:\n  embodiment.consistency: error\n  embodiment.locator-exists:\n    level: error\n{exists_options}record_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n"
    )
}

const ALL_KINDS: &str = "    locator_kinds: [file, directory, glob]\n";
const ALL_PINNABLE: &str =
    "    locator_kinds: [file, directory, glob]\n    pinnable: [file, directory, glob]\n";

fn record(realized_by: &str) -> String {
    format!(
        "---\nEmbodiment: 'Implemented'\nRealized-by: '{realized_by}'\n---\n# ADR-1: First\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
    )
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn repo(name: &str, exists_options: &str, realized_by: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-kinds-{name}"));
    for d in ["docs/adr", "src/sub", "other", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(exists_options)).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("src/b.rs"), "fn b() {}\n").unwrap();
    std::fs::write(dir.join("src/sub/c.rs"), "fn c() {}\n").unwrap();
    std::fs::write(dir.join("other/x.txt"), "x\n").unwrap();
    std::fs::write(dir.join(RECORD_PATH), record(realized_by)).unwrap();
    commit_all(&dir);
    dir
}

fn messages(dir: &Path, rule: &str) -> Vec<String> {
    let parsed = json(&run_urzua(dir, &["check", "docs/"]));
    parsed["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("no findings array: {parsed}"))
        .iter()
        .filter(|f| f["rule"] == rule)
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

fn exists_messages(dir: &Path) -> Vec<String> {
    messages(dir, "embodiment.locator-exists")
}

fn drifted(dir: &Path) -> bool {
    messages(dir, "embodiment.consistency")
        .iter()
        .any(|m| m.contains("Drift detected"))
}

#[test]
fn the_default_accepts_a_file_only_and_a_directory_or_glob_is_a_finding_naming_the_option_observed_failing(
) {
    for (locator, kind) in [
        ("code:src", "directory"),
        ("code:src/*.rs", "glob"),
        ("code:src/**/*.rs", "glob"),
    ] {
        let dir = repo("default", "", locator);
        let m = exists_messages(&dir);
        assert_eq!(m.len(), 1, "{locator}: {m:?}");
        assert!(
            m[0].contains("locator_kinds") && m[0].contains(kind),
            "{locator}: {m:?}"
        );
    }
    // A file is today's behaviour.
    let dir = repo("default-file", "", "code:src/a.rs");
    assert_eq!(exists_messages(&dir), Vec::<String>::new());
}

#[test]
fn an_opted_in_directory_or_glob_is_legal_when_it_matches_a_tracked_file_observed_failing() {
    for locator in [
        "code:src",
        "code:src/",
        "code:./src/sub",
        "code:src/*.rs",
        "code:src/**/*.rs",
        "code:src/sub/c.rs, code:src",
    ] {
        let dir = repo("optin", ALL_KINDS, locator);
        assert_eq!(exists_messages(&dir), Vec::<String>::new(), "{locator}");
    }
}

#[test]
fn a_kind_left_out_of_the_declaration_is_still_a_finding() {
    let dir = repo(
        "dir-only",
        "    locator_kinds: [file, directory]\n",
        "code:src, code:src/*.rs",
    );
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(
        m[0].contains("glob") && m[0].contains("locator_kinds"),
        "{m:?}"
    );
}

#[test]
fn a_directory_or_glob_matching_no_tracked_file_is_missing() {
    for locator in [
        "code:nodir",
        "code:src/*.py",
        "code:scratch",
        "code:other/*.rs",
    ] {
        let dir = repo("nomatch", ALL_KINDS, locator);
        // On disk and untracked is not tracked.
        std::fs::create_dir_all(dir.join("scratch")).unwrap();
        std::fs::write(dir.join("scratch/u.rs"), "x\n").unwrap();
        let m = exists_messages(&dir);
        assert_eq!(m.len(), 1, "{locator}: {m:?}");
        assert!(
            m[0].contains("not in the working tree")
                || m[0].contains("matches no tracked file")
                || m[0].contains("not tracked by git"),
            "{locator}: {m:?}"
        );
    }
    // A staged deletion of every matched file leaves nothing.
    let dir = repo("staged-delete", ALL_KINDS, "code:other");
    git(&dir, &["rm", "-q", "other/x.txt"]);
    assert_eq!(exists_messages(&dir).len(), 1);
}

#[test]
fn a_pin_on_a_directory_or_glob_needs_the_kind_to_be_pinnable() {
    let zeros = "0".repeat(64);
    let dir = repo(
        "unpinnable",
        ALL_KINDS,
        &format!("code:src@sha256:{zeros}, code:src/*.rs@sha256:{zeros}"),
    );
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 2, "{m:?}");
    assert!(m.iter().all(|m| m.contains("pinnable")), "{m:?}");
    let dir = repo(
        "pinnable",
        "    locator_kinds: [file, directory, glob]\n    pinnable: [directory]\n",
        &format!("code:src@sha256:{zeros}, code:src/*.rs@sha256:{zeros}"),
    );
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(
        m[0].contains("src/*.rs") && m[0].contains("pinnable"),
        "{m:?}"
    );
}

#[test]
fn a_scope_on_a_directory_is_reported() {
    let dir = repo("scope", ALL_KINDS, "code:src#L1-L2");
    let m = exists_messages(&dir);
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(
        m[0].contains("directory") && m[0].contains("scope"),
        "{m:?}"
    );
}

#[test]
fn the_options_are_validated_at_load() {
    for (what, options, needle) in [
        (
            "pinnable outside kinds",
            "    pinnable: [directory]\n",
            "pinnable",
        ),
        ("empty kinds", "    locator_kinds: []\n", "locator_kinds"),
        ("unknown kind", "    locator_kinds: [file, tree]\n", "tree"),
    ] {
        let dir = repo("bad-config", options, "code:src/a.rs");
        let out = run_urzua(&dir, &["check", "docs/"]);
        assert_eq!(out.status.code(), Some(2), "{what}: {out:?}");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains(needle),
            "{what}: {out:?}"
        );
    }
    // The options belong to embodiment.locator-exists alone.
    let dir = repo("elsewhere", "", "code:src/a.rs");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        config("").replace(
            "  embodiment.consistency: error\n",
            "  embodiment.consistency:\n    level: error\n    locator_kinds: [directory]\n",
        ),
    )
    .unwrap();
    let out = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("does not take the option"));
}

// ---- explain, drift and re-baselining ---------------------------------------------

fn explained(dir: &Path, arg: &str) -> serde_json::Value {
    let out = run_urzua(dir, &["explain", arg]);
    assert!(out.status.success(), "{out:?}");
    json(&out)
}

fn pinned_repo(name: &str, locator: &str) -> common::TempDir {
    let dir = repo(name, ALL_PINNABLE, "code:src/a.rs");
    let pin = explained(&dir, locator)["coverage"]["pin"]
        .as_str()
        .unwrap_or_else(|| panic!("no pin for {locator}"))
        .to_string();
    std::fs::write(dir.join(RECORD_PATH), record(&format!("code:{pin}"))).unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn explain_shows_the_files_a_directory_or_glob_pin_covers() {
    let dir = repo("explain", ALL_PINNABLE, "code:src/a.rs");
    let parsed = explained(&dir, "src");
    let c = &parsed["coverage"];
    assert_eq!(c["kind"], "directory", "{parsed}");
    assert_eq!(
        c["files"],
        serde_json::json!(["src/a.rs", "src/b.rs", "src/sub/c.rs"])
    );
    let hash = c["sha256"].as_str().unwrap();
    assert_eq!(c["pin"], format!("src@sha256:{hash}"));

    let parsed = explained(&dir, "src/*.rs");
    assert_eq!(parsed["coverage"]["kind"], "glob");
    assert_eq!(
        parsed["coverage"]["files"],
        serde_json::json!(["src/a.rs", "src/b.rs"])
    );
    // The same files hash to the same pin whatever the spelling.
    let recursive = explained(&dir, "src/**");
    assert_eq!(
        recursive["coverage"]["files"],
        explained(&dir, "src")["coverage"]["files"]
    );
    assert_eq!(recursive["coverage"]["sha256"], hash);

    // A file is the old query, with no coverage.
    assert!(explained(&dir, "src/a.rs").get("coverage").is_none());
}

#[test]
fn explain_of_a_kind_the_config_does_not_declare_says_so() {
    let dir = repo("explain-undeclared", "", "code:src/a.rs");
    let parsed = explained(&dir, "src");
    assert!(parsed.get("coverage").is_none(), "{parsed}");
    let notice = parsed["notices"][0]["message"].as_str().unwrap();
    assert!(notice.contains("locator_kinds"), "{parsed}");
}

#[test]
fn explain_of_a_directory_that_matches_nothing_is_a_could_not_run() {
    let dir = repo("explain-none", ALL_PINNABLE, "code:src/a.rs");
    let out = run_urzua(&dir, &["explain", "src/*.py"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("matches no tracked file"));
}

#[test]
fn an_unpinned_directory_drifts_on_the_newest_commit_among_its_files() {
    for locator in ["code:src", "code:src/**/*.rs"] {
        let dir = repo("drift-unpinned", ALL_KINDS, locator);
        // The record is as new as the files: no drift.
        assert!(!drifted(&dir), "{locator}");
        // A commit that touches nothing the locator matches.
        std::fs::write(dir.join("other/x.txt"), "y\n").unwrap();
        commit_all(&dir);
        assert!(!drifted(&dir), "{locator}: unrelated change");
        // A commit to a file deep under it.
        std::fs::write(dir.join("src/sub/c.rs"), "fn c() { 1; }\n").unwrap();
        commit_all(&dir);
        assert!(drifted(&dir), "{locator}: matched file changed");
    }
}

#[test]
fn a_directory_pin_hashes_the_sorted_paths_and_content_hashes_of_its_files_observed_failing() {
    let dir = pinned_repo("pin-drift", "src");
    assert!(!drifted(&dir));
    // Outside the directory: not drift.
    std::fs::write(dir.join("other/x.txt"), "y\n").unwrap();
    commit_all(&dir);
    assert!(!drifted(&dir));
    // A content change to a matched file, deep under it.
    std::fs::write(dir.join("src/sub/c.rs"), "fn c() { 1; }\n").unwrap();
    commit_all(&dir);
    assert!(drifted(&dir), "content change");
    // Restoring the content restores the pin.
    std::fs::write(dir.join("src/sub/c.rs"), "fn c() {}\n").unwrap();
    commit_all(&dir);
    assert!(!drifted(&dir), "restored");
    // A new file under it changes the set.
    std::fs::write(dir.join("src/d.rs"), "fn d() {}\n").unwrap();
    commit_all(&dir);
    assert!(drifted(&dir), "added file");
}

#[test]
fn a_glob_pin_follows_the_files_the_glob_matches() {
    let dir = pinned_repo("glob-pin", "src/*.rs");
    assert!(!drifted(&dir));
    // Below the glob's one level: outside the match.
    std::fs::write(dir.join("src/sub/c.rs"), "fn c() { 1; }\n").unwrap();
    commit_all(&dir);
    assert!(!drifted(&dir));
    std::fs::write(dir.join("src/b.rs"), "fn b() { 1; }\n").unwrap();
    commit_all(&dir);
    assert!(drifted(&dir));
}

#[test]
fn rebaseline_lists_the_files_and_re_pins_a_directory() {
    let dir = pinned_repo("rebase", "src");
    std::fs::write(dir.join("src/b.rs"), "fn b() { 1; }\n").unwrap();
    commit_all(&dir);
    let listed = json(&run_urzua(&dir, &["fix", "--rebaseline"]));
    let pin = &listed["rebaselines"][0]["pins"][0];
    assert_eq!(pin["locator"], "src", "{listed}");
    assert_eq!(
        pin["files"],
        serde_json::json!(["src/a.rs", "src/b.rs", "src/sub/c.rs"]),
        "{listed}"
    );
    let applied = run_urzua(
        &dir,
        &[
            "fix",
            "--rebaseline",
            "--apply",
            "--ids",
            RECORD_PATH,
            "--by",
            "ana",
            "--reason",
            "Read the files; the claim holds.",
        ],
    );
    assert!(applied.status.success(), "{applied:?}");
    let new_pin = explained(&dir, "src")["coverage"]["pin"]
        .as_str()
        .unwrap()
        .to_string();
    let rec = std::fs::read_to_string(dir.join(RECORD_PATH)).unwrap();
    assert!(rec.contains(&format!("code:{new_pin}")), "{rec}");
    commit_all(&dir);
    assert!(!drifted(&dir));
}

#[test]
fn a_directory_locator_under_the_default_config_keeps_todays_drift() {
    // An undeclared kind is reported by locator-exists and drift is judged as
    // it always was, on the path's git history.
    let dir = repo("legacy", "", "code:src");
    std::fs::write(dir.join("src/b.rs"), "fn b() { 1; }\n").unwrap();
    commit_all(&dir);
    assert!(drifted(&dir));
}
