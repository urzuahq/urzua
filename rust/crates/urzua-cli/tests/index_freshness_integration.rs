//! `index.freshness` (`MILE-139`) against the real binary: the CLI reads the
//! declared index artifacts, the rule compares them with the records, and a
//! run says how many it compared.

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

fn repo(name: &str, indexes: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-index-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  index.freshness:\n    level: error\n    indexes: {indexes}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    for (n, title) in [(1, "One"), (2, "Two")] {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!("---\nStatus: Accepted\n---\n# {n} \u{2014} {title}\n"),
        )
        .unwrap();
    }
    dir
}

const INDEXES: &str = "[{path: \"docs/adr/README.md\", type: adr, format: \"- [{id}]({file}): {title}\", begin: \"<!-- begin -->\", end: \"<!-- end -->\"}]";

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn check(dir: &Path) -> (std::process::Output, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}: {stdout}"));
    (out, parsed)
}

fn findings(parsed: &serde_json::Value) -> Vec<serde_json::Value> {
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "index.freshness")
        .cloned()
        .collect()
}

fn readme(entries: &[&str]) -> String {
    format!(
        "# ADRs\n\n<!-- begin -->\n{}\n<!-- end -->\n",
        entries.join("\n")
    )
}

const E1: &str = "- [ADR-1](ADR-1-x.md): One";
const E2: &str = "- [ADR-2](ADR-2-x.md): Two";

#[test]
fn a_current_index_passes_and_the_census_counts_it() {
    let dir = repo("current", INDEXES);
    std::fs::write(dir.join("docs/adr/README.md"), readme(&[E1, E2])).unwrap();
    commit_all(&dir);
    let (out, parsed) = check(&dir);
    assert!(findings(&parsed).is_empty(), "{parsed}");
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    let exec = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "index.freshness")
        .unwrap();
    assert_eq!(exec["population"]["examined"], 1, "{exec}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_record_an_extra_entry_and_a_stale_title_each_block() {
    let dir = repo("stale", INDEXES);
    std::fs::write(
        dir.join("docs/adr/README.md"),
        readme(&["- [ADR-1](ADR-1-x.md): Old", "- [ADR-7](ADR-7-x.md): Ghost"]),
    )
    .unwrap();
    commit_all(&dir);
    let (out, parsed) = check(&dir);
    let all = findings(&parsed);
    let text: Vec<String> = all
        .iter()
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(all.len(), 3, "{text:?}");
    assert!(text.iter().any(|m| m.contains("ADR-2 has no entry")));
    assert!(text.iter().any(|m| m.contains("ADR-7 names no adr record")));
    assert!(text.iter().any(|m| m.contains("title 'Old'")));
    assert!(all.iter().all(|f| f["file"] == "docs/adr/README.md"));
    assert_eq!(out.status.code(), Some(1), "{parsed}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_index_file_and_a_removed_marker_pair_block() {
    let dir = repo("missing", INDEXES);
    commit_all(&dir);
    let (out, parsed) = check(&dir);
    assert_eq!(findings(&parsed).len(), 1, "{parsed}");
    assert_eq!(out.status.code(), Some(1));

    std::fs::write(dir.join("docs/adr/README.md"), "# ADRs\n\n").unwrap();
    let (out, parsed) = check(&dir);
    let all = findings(&parsed);
    assert_eq!(all.len(), 1, "{parsed}");
    assert!(all[0]["message"].as_str().unwrap().contains("marker"));
    assert_eq!(out.status.code(), Some(1));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_rule_does_not_load_without_indexes_or_with_a_bad_format() {
    for (name, declaration) in [
        ("none", "[]"),
        (
            "badformat",
            "[{path: \"docs/adr/README.md\", type: adr, format: \"no placeholder\"}]",
        ),
        (
            "badtype",
            "[{path: \"docs/adr/README.md\", type: nope, format: \"{id}\"}]",
        ),
        (
            "onemarker",
            "[{path: \"docs/adr/README.md\", type: adr, format: \"{id}\", begin: \"<!-- b -->\"}]",
        ),
    ] {
        let dir = repo(name, declaration);
        commit_all(&dir);
        let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
            .arg("check")
            .current_dir(&dir)
            .output()
            .unwrap();
        assert_ne!(out.status.code(), Some(0), "{name}");
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("indexes"),
            "{name}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
