//! The declared `Embodiment` vocabulary (`MILE-144`) against the real binary:
//! a repository that names its own states is held to those names by `check`
//! and repaired to them by `fix`, and a bad declaration is refused at load.

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
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

const VOCABULARY: &str = "embodiment:\n  drift: Stale\n  tiers:\n    - {state: Proven, evidence: test, claims_built: true}\n    - {state: Built, evidence: code, claims_built: true}\n    - {state: Described, evidence: spec}\n  none: Untouched\n";

/// ADR-1 states `Described` on spec evidence (agrees), ADR-2 states `Proven`
/// on spec evidence (disagrees), ADR-3 states `Built` with no evidence at all
/// (a claim with nothing behind it), ADR-4 states the built-in `Verified`,
/// which this vocabulary does not declare.
fn repo(name: &str, vocabulary: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("urzua-vocab-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for d in ["docs/adr", "docs/spec", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n{vocabulary}rules:\n  embodiment.consistency:\n    level: error\n  \
             embodiment.requires-accepted-status: error\n\
             record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    \
             required_fields: [\"Status\", \"Embodiment\"]\n    known_fields: [\"Realized-by\"]\n    \
             header_shape: \"yaml-frontmatter\"\n    \
             statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n"
        ),
    )
    .unwrap();
    for (n, state, locator) in [
        (1, "Described", "Realized-by: spec:docs/spec/s.md\n"),
        (2, "Proven", "Realized-by: spec:docs/spec/s.md\n"),
        (3, "Built", ""),
        (4, "Verified", "Realized-by: spec:docs/spec/s.md\n"),
    ] {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!("---\nStatus: Accepted\nEmbodiment: {state}\n{locator}---\n# {n} — X\n"),
        )
        .unwrap();
    }
    std::fs::write(dir.join("docs/spec/s.md"), "spec\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn parsed(out: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).expect("json")
}

fn consistency(report: &serde_json::Value) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_string(),
                f["message"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_declared_vocabulary_is_what_check_holds_records_to() {
    let dir = repo("check", VOCABULARY);
    let found = consistency(&parsed(&run_urzua(&dir, &["check"])));
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found[0].0.contains("ADR-2") && found[0].1.contains("'Described'"),
        "{found:?}"
    );
    assert!(
        found[1].0.contains("ADR-3") && found[1].1.contains("'Untouched'"),
        "{found:?}"
    );
    assert!(
        found[2].0.contains("ADR-4") && found[2].1.contains("'Described'"),
        "{found:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn without_a_declaration_the_built_in_vocabulary_applies() {
    let dir = repo("default", "");
    let found = consistency(&parsed(&run_urzua(&dir, &["check"])));
    // ADR-3 states `Built`, which the built-in vocabulary does not know as a
    // claim, so it is not judged; the other three disagree with `Specified`.
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found.iter().all(|(_, m)| m.contains("'Specified'")),
        "{found:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn stating_the_default_vocabulary_explicitly_changes_nothing() {
    let default_decl = "embodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\n";
    let (a, b) = (repo("implicit", ""), repo("explicit", default_decl));
    let ra = run_urzua(&a, &["check"]);
    let rb = run_urzua(&b, &["check"]);
    let strip = |out: &std::process::Output, dir: &Path| {
        String::from_utf8_lossy(&out.stdout).replace(dir.to_str().unwrap(), "<dir>")
    };
    let (ta, tb) = (strip(&ra, &a), strip(&rb, &b));
    let norm = |s: String| {
        s.replace("urzua-vocab-implicit", "x")
            .replace("urzua-vocab-explicit", "x")
    };
    assert_eq!(norm(ta), norm(tb));
    std::fs::remove_dir_all(&a).ok();
    std::fs::remove_dir_all(&b).ok();
}

#[test]
fn fix_repairs_to_the_declared_names() {
    let dir = repo("fix", VOCABULARY);
    let report = parsed(&run_urzua(&dir, &["fix"]));
    let repairs = report["repairs"].as_array().unwrap();
    let computed: Vec<(String, String)> = repairs
        .iter()
        .map(|r| {
            (
                r["record"].as_str().unwrap().to_string(),
                r["computed_value"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(computed.len(), 2, "{report}");
    assert!(
        computed
            .iter()
            .any(|(r, v)| r.contains("ADR-2") && v == "Described"),
        "{computed:?}"
    );
    assert!(
        computed
            .iter()
            .any(|(r, v)| r.contains("ADR-4") && v == "Described"),
        "{computed:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_built_claiming_declared_state_needs_an_accepted_status() {
    let dir = repo("accepted", VOCABULARY);
    std::fs::write(
        dir.join("docs/adr/ADR-3-x.md"),
        "---\nStatus: Proposed\nEmbodiment: Proven\n---\n# 3 — X\n",
    )
    .unwrap();
    let report = parsed(&run_urzua(&dir, &["check"]));
    let hit = report["findings"].as_array().unwrap().iter().any(|f| {
        f["rule"] == "embodiment.requires-accepted-status"
            && f["file"].as_str().unwrap().contains("ADR-3")
    });
    assert!(hit, "{report}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_bad_declaration_is_refused_at_load() {
    for (name, decl, needle) in [
        ("dup", "embodiment:\n  drift: A\n  tiers: [{state: A, evidence: test, claims_built: true}]\n  none: B\n", "twice"),
        ("ev", "embodiment:\n  drift: A\n  tiers: [{state: B, evidence: test, claims_built: true}, {state: C, evidence: test}]\n  none: D\n", "evidence"),
        ("empty", "embodiment:\n  drift: A\n  tiers: []\n  none: D\n", "tiers"),
        (
            "claims",
            "embodiment:\n  drift: A\n  tiers: [{state: B, evidence: test}]\n  none: D\n",
            "claims_built",
        ),
    ] {
        let dir = repo(name, decl);
        let out = run_urzua(&dir, &["check"]);
        assert!(!out.status.success());
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(text.contains("embodiment") && text.contains(needle), "{name}: {text}");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn a_sticky_state_naming_a_declared_state_is_refused() {
    let dir = repo("sticky", VOCABULARY);
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        cfg.replace(
            "level: error\n  embodiment.requires",
            "level: error\n    sticky_states: [Proven]\n  embodiment.requires",
        ),
    )
    .unwrap();
    let out = run_urzua(&dir, &["check"]);
    assert!(!out.status.success());
    std::fs::remove_dir_all(&dir).ok();
}
