//! MILE-190: `check --rules`, `rules_selected`, and `audit` as a deprecated
//! alias of `check` (ADR-79). Real git-repo fixtures, real binary.

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

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-audit-fold-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    dir
}

fn commit(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error, pointer.resolution: error, relation.supersession-reciprocity: error, field.pending: off}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n    known_fields: [\"Supersedes / Superseded-by\"]\n";

/// ADR-1 lacks `Date` (header.required-fields) and claims to supersede ADR-2,
/// which never points back (relation.supersession-reciprocity).
fn violating(name: &str) -> common::TempDir {
    let dir = repo(name);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nSupersedes / Superseded-by: ADR-2\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Superseded\nDate: 2026-01-01\n---\n# 2 — Y\n",
    )
    .unwrap();
    commit(&dir);
    dir
}

fn executed(v: &serde_json::Value) -> Vec<String> {
    v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["rule"].as_str().unwrap().to_string())
        // The refusal that cannot be turned off runs under every selection
        // (`identity.record-symlink-escapes`, SPEC-2); these tests count the rules
        // a selection chose.
        .filter(|rule| rule != "identity.record-symlink-escapes")
        .collect()
}

fn finding_rules(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn an_unfiltered_run_reports_rules_selected_as_null() {
    let dir = violating("unfiltered");
    let (_, v) = urzua(&dir, &["check"]);
    assert!(v.as_object().unwrap().contains_key("rules_selected"), "{v}");
    assert!(v["rules_selected"].is_null(), "{v}");
}

#[test]
fn rules_restricts_the_run_names_the_filter_and_omits_the_rest() {
    let dir = violating("selected");
    let (code, v) = urzua(
        &dir,
        &["check", "--rules", "relation.supersession-reciprocity"],
    );
    assert_eq!(
        v["rules_selected"],
        serde_json::json!(["relation.supersession-reciprocity"])
    );
    // The excluded rules are omitted: not reported as `not-enabled`, which
    // would say the repository declined a policy it declared.
    assert_eq!(
        executed(&v),
        vec!["relation.supersession-reciprocity"],
        "{v}"
    );
    assert_eq!(
        finding_rules(&v),
        vec!["relation.supersession-reciprocity"],
        "{v}"
    );
    assert_eq!(code, 1, "an error in the selected set exits 1: {v}");
}

#[test]
fn status_and_the_exit_code_are_computed_over_the_selected_set() {
    let dir = violating("exit");
    // header.required-fields is violated, but pointer.resolution is not and is
    // the only rule selected.
    let (code, v) = urzua(&dir, &["check", "--rules", "pointer.resolution"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["status"], "ok", "{v}");
    assert!(finding_rules(&v).is_empty(), "{v}");
    // The full run still fails.
    let (full, _) = urzua(&dir, &["check"]);
    assert_eq!(full, 1);
}

#[test]
fn several_ids_keep_the_order_given_and_a_repeat_is_one() {
    let dir = violating("several");
    let (_, v) = urzua(
        &dir,
        &[
            "check",
            "--rules",
            "pointer.resolution,header.required-fields,pointer.resolution",
        ],
    );
    assert_eq!(
        v["rules_selected"],
        serde_json::json!(["pointer.resolution", "header.required-fields"]),
        "{v}"
    );
    assert_eq!(executed(&v).len(), 2, "{v}");
}

#[test]
fn a_rule_the_config_does_not_declare_is_an_invocation_error() {
    let dir = violating("undeclared");
    // `section.required` is a rule this build ships; the config does not declare it.
    let (code, v) = urzua(&dir, &["check", "--rules", "section.required"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["status"], "not-run", "{v}");
    let err = v["error"].as_str().unwrap();
    assert!(
        err.contains("section.required") && err.contains("not declared"),
        "{err}"
    );
}

#[test]
fn a_rule_this_build_does_not_ship_and_an_empty_entry_are_invocation_errors() {
    let dir = violating("unknown");
    let (code, v) = urzua(&dir, &["check", "--rules", "no.such-rule"]);
    assert_eq!(code, 2, "{v}");
    assert!(v["error"].as_str().unwrap().contains("no.such-rule"), "{v}");
    let (code, v) = urzua(
        &dir,
        &[
            "check",
            "--rules",
            "pointer.resolution,,header.required-fields",
        ],
    );
    assert_eq!(code, 2, "{v}");
}

#[test]
fn selection_never_enables_a_declared_off_rule() {
    let dir = violating("off");
    let (code, v) = urzua(&dir, &["check", "--rules", "field.pending"]);
    // The rule is declared `off`: it is listed as not-enabled, nothing ran.
    assert_eq!(executed(&v), vec!["field.pending"], "{v}");
    assert_eq!(v["rules_executed"][0]["status"], "not-enabled", "{v}");
    assert_eq!(v["status"], "not-run", "{v}");
    assert_eq!(code, 2);
}

fn without_the_audit_notice(mut v: serde_json::Value) -> serde_json::Value {
    if let Some(notices) = v.get_mut("notices").and_then(|n| n.as_array_mut()) {
        notices.retain(|n| n["subject"] != "audit");
    }
    if v["notices"].as_array().is_some_and(|n| n.is_empty()) {
        v.as_object_mut().unwrap().remove("notices");
    }
    v
}

#[test]
fn audit_is_check_over_the_configured_rules_plus_one_deprecation_notice() {
    let dir = violating("alias");
    let (check_code, check) = urzua(&dir, &["check"]);
    let (audit_code, audit) = urzua(&dir, &["audit"]);
    assert_eq!(
        audit_code, check_code,
        "the notice never moves the exit code"
    );
    assert_eq!(without_the_audit_notice(audit.clone()), check);
    let notice = audit["notices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["subject"] == "audit")
        .unwrap_or_else(|| panic!("a deprecation notice: {audit}"));
    let message = notice["message"].as_str().unwrap();
    assert!(
        message.contains("urzua check") && message.contains("deprecated"),
        "{message}"
    );
}

#[test]
fn audit_on_a_clean_corpus_still_exits_0_with_the_notice() {
    let dir = repo("alias-clean");
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nDate: 2026-01-01\n---\n# 1 — X\n",
    )
    .unwrap();
    commit(&dir);
    let (code, v) = urzua(&dir, &["audit"]);
    assert_eq!(code, 0, "{v}");
    assert!(v["notices"]
        .as_array()
        .is_some_and(|n| n.iter().any(|x| x["subject"] == "audit")));
}

/// BUG-99: on a corpus with no type prefix, `init` declines the
/// identity-dependent rules, and `audit` used to own exactly those, so it
/// exited 2 on the config `init` had just written. `audit` owns no rule set
/// now: it is `check` over whatever the config declares.
#[test]
fn audit_on_the_config_init_writes_for_a_prefixless_corpus_matches_check() {
    let dir = repo("bug-99");
    std::fs::write(
        dir.join("docs/adr/0001-alpha.md"),
        "---\nStatus: Accepted\n---\n# 1 — Alpha\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0002-beta.md"),
        "---\nStatus: Accepted\n---\n# 2 — Beta\n",
    )
    .unwrap();
    commit(&dir);
    let init = common::urzua()
        .arg("init")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(init.status.success(), "{init:?}");
    commit(&dir);

    let (check_code, check) = urzua(&dir, &["check"]);
    let (audit_code, audit) = urzua(&dir, &["audit"]);
    assert_ne!(
        audit_code, 2,
        "audit must not exit 2 on a config check accepts: {audit}"
    );
    assert_eq!(audit_code, check_code);
    assert_eq!(without_the_audit_notice(audit), check);
}

/// MILE-190: the fold is exercised against this repository's own corpus and
/// config, not only against fixtures. `audit` is `check` over everything this
/// repository declares plus the one notice, and `check --rules` restricts the
/// run to the named declared rules and computes the exit code over them.
#[test]
fn this_repositorys_own_corpus_is_audited_and_checked_with_rules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");

    // `audit` owns no rule set: it runs every rule this repository declares.
    let (audit_code, audit) = urzua(&root, &["--as-of", common::OWN_CORPUS_AS_OF, "audit"]);
    assert_eq!(audit_code, 0, "this repository audits clean: {audit}");
    assert!(audit["rules_selected"].is_null(), "{audit}");
    let notices = audit["notices"].as_array().expect("notices");
    assert!(
        notices.iter().any(|n| n["subject"] == "audit"),
        "the deprecation notice: {audit}"
    );
    let all = executed(&audit);
    assert!(all.len() > 20, "only {} rules ran", all.len());

    // `check --rules` over two rules this repository declares.
    let selected = ["status.delivery-agreement", "pointer.resolution"];
    for rule in &selected {
        assert!(all.iter().any(|r| r == rule), "{rule} is declared here");
    }
    let (code, v) = urzua(
        &root,
        &[
            "--as-of",
            common::OWN_CORPUS_AS_OF,
            "check",
            "--rules",
            &selected.join(","),
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["rules_selected"], serde_json::json!(selected), "{v}");
    assert_eq!(v["status"], "ok", "{v}");
    let mut ran = executed(&v);
    ran.sort();
    let mut expected: Vec<String> = selected.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(ran, expected, "the other rules are omitted");

    // A rule this repository does not declare is an invocation error here too.
    let (bad, report) = urzua(&root, &["check", "--rules", "no.such-rule"]);
    assert_eq!(bad, 2, "{report}");
    assert_eq!(report["status"], "not-run", "{report}");
}

/// Every acceptance case that runs `check` over this repository's own corpus pins the
/// evaluation date, so none of them goes red when a claim expires (ADR-97 leaves expiry to
/// the records gate, on the real clock).
#[test]
fn every_own_corpus_acceptance_case_pins_the_evaluation_date() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/acceptance/corpora/corpus-zero/expectations");
    let mut own = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        if !text.contains("corpus: \"@repo\"") {
            continue;
        }
        own += 1;
        let line = text.lines().find(|l| l.starts_with("invocation:")).unwrap();
        assert!(
            line.contains(&format!("--as-of, \"{}\"", common::OWN_CORPUS_AS_OF)),
            "{} does not pin --as-of: {line}",
            path.display()
        );
    }
    assert!(own >= 20, "only {own} own-corpus cases found");
}
