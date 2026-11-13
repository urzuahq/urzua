//! MILE-155: a relation pair bound to the record types that carry it. The
//! scoped case runs this repository's own `.urzua/config.yaml`; the global
//! case runs the same pair without `types` for contrast.

mod common;

use std::process::Command;

fn own_config() -> String {
    common::own_config_for_fixture()
}

fn run(name: &str, config: &str) -> serde_json::Value {
    let dir = common::TempDir::new(&format!("urzua-scoped-pair-{name}"));
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(
        dir.join("docs/rfc/RFC-1-amender.md"),
        "---\nStatus: Accepted\nDate: 2026-11-04\nAuthor: Ana\nAmends: ADR-2\n---\n# 1 - Amender\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-base.md"),
        "---\nStatus: Accepted\nDate: 2026-11-04\nAuthor: Ana\nDeciders: Ana\n---\n# 2 - Base\n",
    )
    .unwrap();
    for args in [&["init", "-q"][..], &["add", "-A"][..]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
    }
    let out = common::urzua()
        .args(["check", "docs/"])
        .current_dir(&dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn pair_findings(report: &serde_json::Value) -> usize {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "relation.pair-reciprocity")
        .count()
}

#[test]
fn an_rfc_amending_an_adr_is_eligible_and_unexamined_under_the_scoped_pair_observed_failing() {
    let report = run("scoped", &own_config());
    assert_eq!(pair_findings(&report), 0, "{report}");
    let rule = report["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "relation.pair-reciprocity")
        .expect("rule executed");
    assert_eq!(rule["population"]["out_of_scope"], 0, "{rule}");
    // A legitimate cross-type claim is not a configuration the tool cannot
    // reach, so the config-level rule has nothing to warn about.
    let scope_warnings: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| {
            f["rule"] == "config.scope-matches-nothing"
                && f["message"]
                    .as_str()
                    .unwrap_or("")
                    .contains("relation.pair-reciprocity")
        })
        .collect();
    assert!(scope_warnings.is_empty(), "{report}");
}

#[test]
fn the_same_claim_errors_when_the_pair_is_global() {
    let config = own_config().replace("    types: [\"rfc\"]\n", "");
    assert!(config.contains("name: amendment") && !config.contains("types: [\"rfc\"]"));
    let report = run("global", &config);
    // The ADR declares no Amended-by, so a global pair cannot be echoed.
    assert_eq!(pair_findings(&report), 1, "{report}");
}
