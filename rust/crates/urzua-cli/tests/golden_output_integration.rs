//! Golden output of the commands (MILE-171, MILE-172, MILE-173).
//!
//! Three milestones restructured the command layer without changing what a
//! command prints: the rule table and runner (`MILE-171`), the split of
//! `check::run` into stages (`MILE-172`), and the shared session open of the
//! nine commands (`MILE-173`). Each says the output is unchanged. This test
//! holds that claim: it runs every one of the nine commands (and the scoped
//! forms of `check`) over a fixed corpus and compares the whole JSON output
//! with the checked-in file under `tests/fixtures/golden-output/`. A restructure
//! that changes a byte of any of them fails here; an intended change to an
//! output is made by regenerating the files and reading the diff:
//!
//!     URZUA_UPDATE_GOLDEN=1 cargo test -p urzua-cli --test golden_output_integration
//!
//! The corpus is `tests/fixtures/golden-corpus`. It holds two record types (one
//! with `header_shape: none`, so the out-of-scope census is covered), findings
//! from four rules, a dangling pointer and a stale `Embodiment` repair, and the
//! run happens in a fresh git repository with two commits so `--base` has a diff.
//! A rule that is `not-enabled` is left out of `rules_executed`, so adding a rule
//! to the build does not change a golden file; a change to any rule the corpus
//! enables does.

mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The corpus as a git repository with two commits; the second edits `ADR-1`.
fn repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-golden-{name}"));
    copy_tree(&manifest().join("tests/fixtures/golden-corpus"), &dir);
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "one"]);
    let adr = dir.join("docs/adr/ADR-1-first.md");
    let mut text = std::fs::read_to_string(&adr).unwrap();
    text.push_str("\nA later edit.\n");
    std::fs::write(&adr, text).unwrap();
    git(&dir, &["commit", "-q", "-a", "-m", "two"]);
    dir
}

/// Drops what varies run to run or build to build: the root path, the clock's
/// evaluation date, and the rules a build lists as `not-enabled`.
fn normalise(value: &mut Value, root: &str) {
    match value {
        Value::String(s) => *s = s.replace(root, "<ROOT>"),
        Value::Array(items) => items.iter_mut().for_each(|v| normalise(v, root)),
        Value::Object(map) => {
            if let Some(Value::Array(rules)) = map.get_mut("rules_executed") {
                rules.retain(|r| r["status"] != "not-enabled");
            }
            // The evaluation date read from the clock is the day the test runs.
            if let Some(Value::Object(as_of)) = map.get_mut("as_of") {
                if as_of.get("source") == Some(&Value::String("clock".into())) {
                    as_of.insert("date".into(), Value::String("<TODAY>".into()));
                }
            }
            map.values_mut().for_each(|v| normalise(v, root));
        }
        _ => {}
    }
}

fn run_case(dir: &Path, args: &[&str]) -> String {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("urzua binary should run");
    let mut stdout: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: stdout is not JSON ({e}): {out:?}"));
    let root = dir.to_str().unwrap();
    normalise(&mut stdout, root);
    let case = serde_json::json!({
        "args": args,
        "exit": out.status.code(),
        "stdout": stdout,
    });
    serde_json::to_string_pretty(&case).unwrap() + "\n"
}

fn golden_path(name: &str) -> PathBuf {
    manifest().join(format!("tests/fixtures/golden-output/{name}.json"))
}

fn assert_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    if std::env::var_os("URZUA_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; regenerate with URZUA_UPDATE_GOLDEN=1",
            path.display()
        )
    });
    assert!(
        expected == actual,
        "{name}: output changed from {}\n--- expected\n{expected}\n--- actual\n{actual}",
        path.display()
    );
}

/// One case per entry: a golden file name and the arguments. `check` covers the
/// whole corpus, a record path, a directory of a `header_shape: none` type, a
/// `--base` diff (alone and narrowed by a path) and a `--rules` selection.
const CASES: &[(&str, &[&str])] = &[
    ("check-whole", &["check"]),
    ("check-path", &["check", "docs/adr/ADR-2-second.md"]),
    ("check-header-none-dir", &["check", "docs/note"]),
    ("check-base", &["check", "--base", "HEAD~1"]),
    (
        "check-base-path",
        &["check", "--base", "HEAD~1", "docs/adr"],
    ),
    (
        "check-rules",
        &["check", "--rules", "header.required-fields"],
    ),
    ("audit", &["audit"]),
    ("doctor", &["doctor"]),
    ("explain-path", &["explain", "src/a.rs"]),
    ("export-json", &["export", "--format", "json"]),
    ("fix", &["fix"]),
    ("fix-path", &["fix", "docs/adr/ADR-2-second.md"]),
    ("graph", &["graph"]),
    ("migrate-ids", &["migrate", "ids"]),
];

#[test]
fn every_command_prints_the_output_it_always_printed() {
    let dir = repo("commands");
    for (name, args) in CASES {
        assert_golden(name, &run_case(&dir, args));
    }
}

/// `new` writes a record, so it runs in a repository of its own. The identifier
/// and the date are the two things that differ between runs; both are replaced
/// by a placeholder, and the written file is part of the golden output.
#[test]
fn new_prints_and_writes_what_it_always_did() {
    let dir = repo("new");
    let out = common::urzua()
        .args(["new", "adr", "Golden title", "--by", "tester"])
        .current_dir(&dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    let mut stdout: Value = serde_json::from_slice(&out.stdout).unwrap();
    let path = stdout["path"].as_str().unwrap().to_string();
    let stable_id = stdout["stable_id"].as_str().unwrap().to_string();
    let written = std::fs::read_to_string(dir.join(&path)).unwrap();
    let today = String::from_utf8(Command::new("date").arg("+%F").output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string();
    normalise(&mut stdout, dir.to_str().unwrap());
    let case = serde_json::json!({
        "args": ["new", "adr", "Golden title", "--by", "tester"],
        "exit": out.status.code(),
        "stdout": stdout,
        "written": written,
    });
    let text = serde_json::to_string_pretty(&case).unwrap() + "\n";
    let text = text
        .replace(&stable_id, "<STABLE-ID>")
        .replace(&today, "<TODAY>");
    assert_golden("new", &text);
}
