//! `check <path>` reads only the source files the back-pointer rules need
//! (`MILE-207`): those under the requested paths, and those whose comments cite
//! a record the run reports on. The findings for what is in scope are the ones
//! a whole-corpus run gives, restricted to it.
//!
//! Verifies: MILE-207

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

const RESOLUTION: &str = "embodiment.back-pointer-resolution";
const DECISION: &str = "embodiment.back-pointer-decision-accepted";
const DANGLING: &str = "code.dangling-bug-reference";

/// The rules that read source comments, or the evidence they give.
const JUDGED: [&str; 5] = [
    RESOLUTION,
    DECISION,
    DANGLING,
    "embodiment.consistency",
    "status.delivery-agreement",
];

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn check(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

/// ADR-1 and ADR-2 are `Proposed`, ADR-3 is `Accepted`; every source file is
/// under `src`. A back-pointer to a `Proposed` record is a decision finding.
fn repo(name: &str, files: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-scoped-read-{name}"));
    for d in ["docs/adr", "src/deep", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 embodiment.back-pointer-resolution:\n    level: error\n    back_pointer_paths: [\"src\"]\n\
         \x20 embodiment.back-pointer-decision-accepted:\n    level: error\n    back_pointer_paths: [\"src\"]\n    accepted_statuses: [\"Accepted\"]\n\
         \x20 code.dangling-bug-reference:\n    level: error\n    back_pointer_paths: [\"src\"]\n    extensions: [\"rs\"]\n\
         record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
         required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    for (n, status) in [(1, "Proposed"), (2, "Proposed"), (3, "Accepted")] {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!("---\nStatus: {status}\n---\n# {n} — X\n"),
        )
        .unwrap();
    }
    for (path, text) in files {
        std::fs::write(dir.join(path), text).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn population(report: &serde_json::Value, rule: &str) -> serde_json::Value {
    report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == rule)
        .unwrap_or_else(|| panic!("{rule} must be listed: {report}"))["population"]
        .clone()
}

/// `(rule, file, line)` of every finding of the three source rules.
fn source_findings(report: &serde_json::Value) -> BTreeSet<(String, String, u64)> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| [RESOLUTION, DECISION, DANGLING].contains(&f["rule"].as_str().unwrap()))
        .map(|f| {
            (
                f["rule"].as_str().unwrap().to_string(),
                f["file"].as_str().unwrap().to_string(),
                f["line"].as_u64().unwrap_or(0),
            )
        })
        .collect()
}

fn sources() -> Vec<(&'static str, &'static str)> {
    vec![
        // Cites ADR-1, a record in scope when the run names it.
        ("src/a.rs", "// Implements: ADR-1\nfn a() {}\n"),
        // Cites ADR-2, which a run naming only ADR-1 does not report on.
        ("src/b.rs", "// Implements: ADR-2\nfn b() {}\n"),
        // A citation of no record, and one a path scope names.
        ("src/deep/c.rs", "// Implements: ADR-77\nfn c() {}\n"),
        ("src/d.rs", "// see ADR-88\nfn d() {}\n"),
    ]
}

#[test]
fn a_source_file_citing_a_record_in_scope_is_read_and_one_citing_another_is_not() {
    let dir = repo("record", &sources());
    let report = check(&dir, &["docs/adr/ADR-1-x.md"]);

    // a.rs cites ADR-1: read, and its finding about ADR-1 is reported.
    let found = source_findings(&report);
    assert!(
        found.contains(&(DECISION.to_string(), "src/a.rs".to_string(), 1)),
        "{found:?}"
    );
    // b.rs cites ADR-2, c.rs a record that does not exist, d.rs mentions one:
    // none is in scope, so none is read and none is reported.
    assert!(
        !found.iter().any(|(_, f, _)| f != "src/a.rs"),
        "only the file citing a record in scope is reported: {found:?}"
    );

    // The population says what was examined: one file, and the three it did
    // not read are counted, never dropped.
    let p = population(&report, DECISION);
    assert_eq!(p["examined"], 1, "{p}");
    assert_eq!(p["deferred_by_scope"], 3, "{p}");
    assert_eq!(p["eligible"], 4, "{p}");
    let p = population(&report, RESOLUTION);
    assert_eq!(p["examined"], 0, "{p}");
    assert_eq!(p["deferred_by_scope"], 4, "{p}");
    assert_eq!(p["eligible"], 4, "{p}");
    assert_eq!(p["out_of_scope"], 0, "{p}");
}

#[test]
fn a_path_inside_the_sources_reads_those_files_whole() {
    let dir = repo("dir", &sources());
    let report = check(&dir, &["src/deep"]);
    let found = source_findings(&report);
    let want: BTreeSet<_> = [(RESOLUTION.to_string(), "src/deep/c.rs".to_string(), 1)].into();
    assert_eq!(found, want, "only the files under the path are judged");
    let p = population(&report, RESOLUTION);
    assert_eq!(p["examined"], 1, "{p}");
    assert_eq!(p["deferred_by_scope"], 3, "{p}");
}

#[test]
fn a_single_source_file_is_judged_like_a_whole_run_restricted_to_it() {
    let dir = repo("file", &sources());
    let full = check(&dir, &[]);
    let scoped = check(&dir, &["src/d.rs"]);
    let want: BTreeSet<_> = source_findings(&full)
        .into_iter()
        .filter(|(_, f, _)| f == "src/d.rs")
        .collect();
    assert!(!want.is_empty(), "the fixture has a finding in d.rs");
    assert_eq!(source_findings(&scoped), want);
}

#[test]
fn a_whole_corpus_run_reads_every_source_file_and_defers_none() {
    let dir = repo("whole", &sources());
    let report = check(&dir, &[]);
    for rule in [RESOLUTION, DECISION, DANGLING] {
        let p = population(&report, rule);
        assert!(p.get("deferred_by_scope").is_none(), "{rule}: {p}");
        assert_eq!(p["examined"], p["eligible"], "{rule}: {p}");
    }
    // Every file's finding is there.
    let found = source_findings(&report);
    for file in ["src/a.rs", "src/b.rs", "src/deep/c.rs", "src/d.rs"] {
        assert!(found.iter().any(|(_, f, _)| f == file), "{file}: {found:?}");
    }
}

#[test]
fn a_scoped_run_says_how_many_source_files_it_did_not_read() {
    let dir = repo("notice", &sources());
    let report = check(&dir, &["docs/adr/ADR-1-x.md"]);
    let notices = report["notices"].as_array().unwrap();
    let said: Vec<&serde_json::Value> = notices
        .iter()
        .filter(|n| n["subject"] == DECISION)
        .collect();
    assert_eq!(said.len(), 1, "{notices:?}");
    let message = said[0]["message"].as_str().unwrap();
    assert!(message.contains("3 of 4"), "{message}");
    assert!(message.contains("path scope"), "{message}");
}

#[test]
fn a_declared_path_with_no_file_is_still_reported_under_a_scope() {
    // `lib` holds no tracked file: the finding is about the configuration and
    // is the same on every run, even though no file was read for it.
    let dir = repo("uncovered", &sources());
    let config = dir.join(".urzua/config.yaml");
    let text = std::fs::read_to_string(&config).unwrap().replace(
        "back_pointer_paths: [\"src\"]",
        "back_pointer_paths: [\"src\", \"lib\"]",
    );
    std::fs::write(&config, text).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "declare lib"]);
    let full = check(&dir, &[]);
    let scoped = check(&dir, &["docs/adr/ADR-3-x.md"]);
    let uncovered = |r: &serde_json::Value| {
        r["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| {
                f["message"]
                    .as_str()
                    .unwrap()
                    .contains("covers no tracked file")
            })
            .count()
    };
    assert_eq!(uncovered(&full), 3, "one per rule: {full}");
    assert_eq!(uncovered(&scoped), 3, "{scoped}");
}

/// A copy of this repository's own tracked files (its corpus, its configuration
/// and its source) in a fresh git repository, with four planted source files:
/// an unresolved back-pointer, a back-pointer to a rejected decision (twice, in
/// two trees) and a dangling citation, so the comparison below is over findings
/// and not over an empty set. Returns the copy and the rejected decision's id.
fn own_corpus_copy() -> (common::TempDir, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = common::TempDir::new("urzua-scoped-read-own");
    let listed = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(&root)
        .output()
        .expect("git ls-files should run");
    assert!(listed.status.success());
    for name in listed.stdout.split(|b| *b == 0).filter(|n| !n.is_empty()) {
        let rel = Path::new(std::str::from_utf8(name).unwrap());
        let from = root.join(rel);
        // A path tracked but absent or a link is not part of the corpus here.
        if !from.is_file() || from.is_symlink() {
            continue;
        }
        let to = dir.join(rel);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&from, &to).unwrap();
    }
    let rejected = walk(&root.join("docs/adr"))
        .into_iter()
        .find(|f| {
            std::fs::read_to_string(f).is_ok_and(|t| t.lines().any(|l| l == "Status: Rejected"))
        })
        .expect("the corpus holds a rejected ADR");
    let name = rejected.file_name().unwrap().to_str().unwrap();
    let mut parts = name.splitn(3, '-');
    let id = format!(
        "{}-{}",
        parts.next().unwrap(),
        parts.next().unwrap().trim_start_matches('0')
    );
    for (path, text) in [
        (
            "rust/crates/urzua-core/src/planted_unresolved.rs",
            "// Implements: ADR-99999\n".to_string(),
        ),
        (
            "rust/crates/urzua-core/src/planted_decision.rs",
            format!("// Implements: {id}\n"),
        ),
        (
            "rust/crates/urzua-cli/src/planted_decision.rs",
            format!("// Implements: {id}\n"),
        ),
        (
            "scripts/planted_dangling.sh",
            "# see BUG-99999\n".to_string(),
        ),
    ] {
        std::fs::write(dir.join(path), text).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "copy"]);
    (dir, id)
}

#[test]
fn a_scoped_run_over_this_repositorys_corpus_equals_a_whole_run_restricted_to_the_scope() {
    // For each scope, the findings of the rules that read source comments, or
    // the evidence they give, are the whole-corpus ones restricted to the paths
    // asked about, plus, for the decision rule, the back-pointers that cite a
    // record in scope (a finding about that record).
    let (root, rejected) = own_corpus_copy();
    let run = |args: &[&str]| -> serde_json::Value {
        let out = common::urzua()
            .args(["--as-of", common::OWN_CORPUS_AS_OF, "check"])
            .args(["--rules", &JUDGED.join(",")])
            .args(args)
            .current_dir(&root)
            .output()
            .expect("urzua binary should run");
        serde_json::from_slice(&out.stdout).expect("json")
    };
    let full = run(&[]);
    let judged = JUDGED;
    let key = |f: &serde_json::Value| {
        (
            f["rule"].as_str().unwrap().to_string(),
            f["file"].as_str().unwrap().to_string(),
            f["line"].as_u64().unwrap_or(0),
            f["message"].as_str().unwrap().to_string(),
        )
    };
    let rejected_file = walk(&root.join("docs/adr"))
        .into_iter()
        .find(|f| {
            std::fs::read_to_string(f).is_ok_and(|t| t.lines().any(|l| l == "Status: Rejected"))
        })
        .unwrap();
    let rejected_rel = rejected_file
        .strip_prefix(&root)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let scopes: Vec<Vec<&str>> = vec![
        vec!["docs/milestones"],
        vec!["docs/milestones/MILE-207-scope-the-back-pointer-rules-for-check-path-on-large-repos-where-they-scan-every-source-file.md"],
        vec![rejected_rel.as_str()],
        vec!["docs/adr"],
        vec!["rust/crates/urzua-core/src"],
        vec!["rust/crates/urzua-core/src/rules/back_pointer.rs"],
        vec!["rust/crates/urzua-cli/tests", "docs/specs"],
        vec!["scripts"],
        vec!["rust/crates/urzua-core/src/planted_unresolved.rs", "docs/rfc"],
    ];
    let mut compared_findings = 0;
    for scope in &scopes {
        let scoped = run(scope);
        let under = |file: &str| scope.iter().any(|p| Path::new(file).starts_with(p));
        let in_scope_ids: BTreeSet<String> = scope_record_ids(&root, scope);
        let mut expected = BTreeSet::new();
        for f in full["findings"].as_array().unwrap() {
            let rule = f["rule"].as_str().unwrap();
            if !judged.contains(&rule) {
                continue;
            }
            let file = f["file"].as_str().unwrap();
            let about_a_record_in_scope = rule == DECISION
                && in_scope_ids.iter().any(|id| {
                    f["message"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("cites {id},"))
                });
            if under(file) || about_a_record_in_scope {
                expected.insert(key(f));
            }
        }
        let got: BTreeSet<_> = scoped["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| judged.contains(&f["rule"].as_str().unwrap()))
            .map(key)
            .collect();
        assert_eq!(got, expected, "scope {scope:?}");
        compared_findings += got.len();
    }
    // The planted findings are what makes the comparison mean something.
    assert!(
        compared_findings >= 6,
        "the scopes compared only {compared_findings} findings"
    );
    let all = source_findings(&full);
    assert!(all
        .iter()
        .any(|(r, f, _)| r == RESOLUTION && f.ends_with("planted_unresolved.rs")));
    assert!(all
        .iter()
        .any(|(r, f, _)| r == DECISION && f.ends_with("planted_decision.rs")));
    assert!(all
        .iter()
        .any(|(r, f, _)| r == DANGLING && f.ends_with("planted_dangling.sh")));
    assert!(!rejected.is_empty());
}

/// The ids (`MILE-207`) of the records under any of `scope`'s paths.
fn scope_record_ids(root: &Path, scope: &[&str]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for p in scope {
        let full = root.join(p);
        let mut files = Vec::new();
        if full.is_dir() {
            for entry in walk(&full) {
                files.push(entry);
            }
        } else {
            files.push(full);
        }
        for f in files {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let mut parts = name.splitn(3, '-');
            if let (Some(prefix), Some(n)) = (parts.next(), parts.next()) {
                let record_dir = f
                    .strip_prefix(root)
                    .is_ok_and(|rel| rel.starts_with("docs"));
                if record_dir
                    && name.ends_with(".md")
                    && prefix.chars().all(|c| c.is_ascii_uppercase())
                    && n.chars().all(|c| c.is_ascii_digit())
                {
                    ids.insert(format!("{prefix}-{}", n.trim_start_matches('0')));
                }
            }
        }
    }
    ids
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}
