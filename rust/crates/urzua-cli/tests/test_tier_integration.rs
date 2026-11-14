//! Test-tier back-pointers (`MILE-146`) and the judging of a record with no
//! `Realized-by` field that a back-pointer cites (`MILE-149`), against the
//! real binary: `Verifies:` in a declared test path computes `Verified`, an
//! `Implements:` elsewhere computes `Implemented`, and a record cited by
//! neither is not judged.

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

fn adr(dir: &Path, n: u32, embodiment: &str) {
    std::fs::write(
        dir.join(format!("docs/adr/ADR-{n}-x.md")),
        format!("---\nStatus: Accepted\nEmbodiment: {embodiment}\n---\n# {n} — X\n"),
    )
    .unwrap();
}

/// ADR-1: stated Verified, cited by `Verifies:` in a test file and by
/// `Implements:` in source. ADR-2: stated Verified, cited by `Implements:`
/// only. ADR-3: stated Verified, cited by nothing. ADR-4: states no tier. None has `Realized-by`.
fn repo(name: &str, test_paths: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-tier-{name}"));
    for d in ["docs/adr", "src", "src/tests", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  embodiment.consistency: error\n  \
             embodiment.back-pointer-resolution:\n    level: error\n    \
             back_pointer_paths: [\"src\"]\n{test_paths}\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\"]\n    \
             known_fields: [\"Realized-by\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    for n in 1..=3 {
        adr(&dir, n, "Verified");
    }
    adr(&dir, 4, "Specified");
    std::fs::write(
        dir.join("src/lib.rs"),
        "// Implements: ADR-1, ADR-2\nfn a() {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/tests/t.rs"),
        "#[test]\nfn t() {} // @V ADR-1\n".replace("@V", "Verifies:"),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn consistency(out: &std::process::Output) -> Vec<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .cloned()
        .collect()
}

#[test]
fn a_test_pointer_verifies_and_a_code_pointer_alone_does_not() {
    let dir = repo("on", "    test_paths: [\"src/tests\"]\n");
    let out = run_urzua(&dir, &["check"]);
    let found = consistency(&out);
    // ADR-1 is Verified; ADR-2 computes Implemented (code only); ADR-3 states
    // Verified with no evidence of either kind and is reported (`MILE-161`);
    // ADR-4 states no tier and is not judged.
    assert_eq!(found.len(), 2, "{}", String::from_utf8_lossy(&out.stdout));
    assert!(found[0]["file"].as_str().unwrap().contains("ADR-2"));
    assert!(found[1]["file"].as_str().unwrap().contains("ADR-3"));
    let none = found[1]["message"].as_str().unwrap();
    assert!(
        none.contains("'Not started'") && none.contains("no evidence"),
        "{none}"
    );
    let message = found[0]["message"].as_str().unwrap();
    assert!(message.contains("'Implemented'"), "{message}");
    assert!(message.contains("source back-pointers"), "{message}");
    assert!(message.contains("no Realized-by field"), "{message}");
}

#[test]
fn with_no_test_class_declared_a_verifies_comment_is_not_evidence() {
    let dir = repo("off", "");
    let out = run_urzua(&dir, &["check"]);
    // ADR-1 and ADR-2 both compute Implemented; ADR-3 has no evidence.
    assert_eq!(consistency(&out).len(), 3);
}

#[test]
fn fix_proposes_the_repair_for_a_cited_record_with_no_realized_by() {
    let dir = repo("fix", "    test_paths: [\"src/tests\"]\n");
    let out = run_urzua(&dir, &["fix"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let repairs = parsed["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1, "{parsed}");
    assert_eq!(repairs[0]["computed_value"], "Implemented");
}

#[test]
fn a_test_path_outside_every_back_pointer_path_is_refused_at_load() {
    let dir = repo("bad", "    test_paths: [\"elsewhere/tests\"]\n");
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2));
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("test_paths"), "{text}");
}

#[test]
fn a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported() {
    // `src/tests/t.rs` is named by the prefix; `src/extra.test.rs` by the glob.
    let dir = repo("glob", "    test_paths: [\"src/tests\", \"**/*.spec.*\"]\n");
    std::fs::write(
        dir.join("src/extra.spec.rs"),
        "// Verifies: ADR-2\nfn s() {}\n",
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "glob fixture"]);
    let out = run_urzua(&dir, &["check"]);
    // ADR-1 and ADR-2 are both Verified now; no consistency finding for them.
    let found = consistency(&out);
    assert!(
        found
            .iter()
            .all(|f| !f["file"].as_str().unwrap().contains("ADR-2")),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    // A glob matching no tracked file under a scanned path is reported.
    let dir2 = repo("glob-none", "    test_paths: [\"**/*.nomatch.*\"]\n");
    let out2 = run_urzua(&dir2, &["check"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out2.stdout).expect("json");
    let hit = parsed["findings"].as_array().unwrap().iter().any(|f| {
        f["rule"] == "embodiment.back-pointer-resolution"
            && f["message"]
                .as_str()
                .unwrap()
                .contains("test path '**/*.nomatch.*'")
    });
    assert!(hit, "{parsed}");
}

#[test]
fn a_malformed_glob_test_path_is_refused_at_load() {
    let dir = repo("badglob", "    test_paths: [\"src/[\"]\n");
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2));
}
