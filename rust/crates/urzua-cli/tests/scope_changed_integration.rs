//! MILE-203, ADR-90: a rule that declares `scope: changed` judges only the
//! records a `--base` diff adds or changes. Real git-repo fixtures, real
//! binary; all output is JSON.

mod common;

use std::path::Path;
use std::process::Command;

const RULE: &str = "embodiment.locator-on-back-pointer-path";
const PATH_1: &str = "docs/adr/ADR-1-a.md";
const PATH_2: &str = "docs/adr/ADR-2-b.md";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

/// The locator nudge, optionally scoped, plus `config.scope-matches-nothing`
/// and a `note` type with no header (its slots are out of scope for the rule).
fn config(scope: &str) -> String {
    format!(
        "schema_version: 2
embodiment:
  drift: Drift detected
  tiers:
    - {{state: Verified, evidence: test, claims_built: true}}
    - {{state: Implemented, evidence: code, claims_built: true}}
    - {{state: Specified, evidence: spec}}
  none: Not started
rules:
  {RULE}:
    level: warn
    back_pointer_paths: [src]
{scope}  config.scope-matches-nothing: warn
record_types:
  adr:
    dir: docs/adr
    required_fields: []
    known_fields: [Embodiment, Realized-by]
    companion_fields:
      - [embodiment_state, embodiment_locator]
"
    )
}

fn record(n: u32, realized_by: Option<&str>) -> String {
    let locator = realized_by
        .map(|r| format!("Embodiment: Implemented\nRealized-by: {r}\n"))
        .unwrap_or_default();
    format!("---\n{locator}---\n# ADR-{n}: T\n")
}

/// ADR-1, ADR-2 and ADR-3 cite `src/a.rs` (each is nudged without the option);
/// ADR-4 declares no locator at all. Committed.
fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-scope-changed-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(PATH_1), record(1, Some("code:src/a.rs"))).unwrap();
    std::fs::write(dir.join(PATH_2), record(2, Some("code:src/a.rs"))).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-3-c.md"),
        record(3, Some("code:src/a.rs")),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-4-d.md"), record(4, None)).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn nudged(v: &serde_json::Value) -> Vec<String> {
    let mut files: Vec<String> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .map(|f| f["file"].as_str().unwrap().to_string())
        .collect();
    files.sort();
    files
}

fn execution<'a>(v: &'a serde_json::Value, rule: &str) -> &'a serde_json::Value {
    v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == rule)
        .unwrap_or_else(|| panic!("no execution of {rule}: {v}"))
}

fn scope_notices(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    // `notices` is omitted from the report when there are none.
    v.get("notices")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
        .filter(|n| n["subject"] == RULE)
        .collect()
}

fn stdin(dir: &Path, args: &[&str], text: &str) -> (i32, serde_json::Value) {
    use std::io::Write;
    let mut child = common::urzua()
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

#[test]
fn a_plain_check_judges_none_says_why_and_raises_no_scope_warning() {
    let dir = fixture("plain", &config("    scope: changed\n"));
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 0, "{v}");
    assert!(nudged(&v).is_empty(), "{v}");
    let exec = execution(&v, RULE);
    assert_eq!(exec["status"], "ran", "{exec}");
    let p = &exec["population"];
    assert_eq!(p["unit"], "field");
    // ADR-4 has no slot, so it is Absent, not deferred.
    assert_eq!(p["eligible"], 4, "{p}");
    assert_eq!(p["examined"], 0, "{p}");
    assert_eq!(p["deferred_by_scope"], 3, "{p}");
    assert_eq!(p["out_of_scope"], 0, "{p}");
    // The deferral is not an unreachable configuration.
    let matches_nothing: Vec<_> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "config.scope-matches-nothing")
        .collect();
    assert!(matches_nothing.is_empty(), "{v}");
    assert_eq!(
        execution(&v, "config.scope-matches-nothing")["status"],
        "ran"
    );
    // One info notice for the rule: the option, the number, the reason.
    let notices = scope_notices(&v);
    assert_eq!(notices.len(), 1, "{v}");
    assert_eq!(notices[0]["severity"], "info");
    let message = notices[0]["message"].as_str().unwrap();
    assert!(message.contains("scope: changed"), "{message}");
    assert!(message.contains('3'), "{message}");
    assert!(message.contains("no --base"), "{message}");
}

#[test]
fn a_base_run_nudges_the_changed_record_and_defers_the_rest() {
    let dir = fixture("changed", &config("    scope: changed\n"));
    // A working-tree edit to ADR-2 only.
    std::fs::write(
        dir.join(PATH_2),
        format!("{}\nMore.\n", record(2, Some("code:src/a.rs"))),
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(nudged(&v), vec![PATH_2.to_string()], "{v}");
    let p = &execution(&v, RULE)["population"];
    assert_eq!(p["eligible"], 4, "{p}");
    assert_eq!(p["examined"], 1, "{p}");
    assert_eq!(p["deferred_by_scope"], 2, "{p}");
    let notices = scope_notices(&v);
    assert_eq!(notices.len(), 1, "{v}");
    let message = notices[0]["message"].as_str().unwrap();
    assert!(message.contains("scope: changed"), "{message}");
    assert!(message.contains("unchanged since HEAD"), "{message}");
}

#[test]
fn a_dependent_of_a_changed_file_is_not_changed() {
    // Editing the file every record cites makes all three records dependents of
    // the diff, which other rules examine. None is itself changed, so the
    // nudge stays silent: a dependent is not nudged.
    let dir = fixture("dependents", &config("    scope: changed\n"));
    std::fs::write(dir.join("src/a.rs"), "fn a() { 1; }\n").unwrap();
    let (code, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "{v}");
    assert!(nudged(&v).is_empty(), "{v}");
    let p = &execution(&v, RULE)["population"];
    assert_eq!(p["examined"], 0, "{p}");
    assert_eq!(p["deferred_by_scope"], 3, "{p}");
}

#[test]
fn a_planted_unchanged_record_is_not_nudged_and_a_changed_one_is() {
    let dir = fixture("committed", &config("    scope: changed\n"));
    std::fs::write(
        dir.join("docs/adr/ADR-3-c.md"),
        format!("{}\nChanged.\n", record(3, Some("code:src/a.rs"))),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "change ADR-3"]);
    let (_, v) = urzua(&dir, &["check", "--base", "HEAD~1"]);
    assert_eq!(nudged(&v), vec!["docs/adr/ADR-3-c.md".to_string()], "{v}");
}

#[test]
fn an_unchanged_record_with_no_slot_is_absent_not_deferred() {
    let dir = fixture("absent", &config("    scope: changed\n"));
    let (_, v) = urzua(&dir, &["check"]);
    let p = &execution(&v, RULE)["population"];
    assert_eq!(p["eligible"], 4);
    assert_eq!(p["deferred_by_scope"], 3);
}

#[test]
fn the_editor_hook_treats_the_proposed_record_as_changed() {
    let dir = fixture("stdin", &config("    scope: changed\n"));
    let (_, v) = stdin(
        &dir,
        &["check", "--stdin-path", PATH_1],
        &record(1, Some("code:src/a.rs")),
    );
    assert_eq!(nudged(&v), vec![PATH_1.to_string()], "{v}");
    let p = &execution(&v, RULE)["population"];
    assert_eq!(p["examined"], 1, "{p}");
    assert_eq!(p["deferred_by_scope"], 2, "{p}");
}

#[test]
fn a_rule_without_the_option_is_unchanged() {
    let dir = fixture("plain-rule", &config(""));
    let (_, v) = urzua(&dir, &["check"]);
    assert_eq!(nudged(&v).len(), 3, "{v}");
    let p = &execution(&v, RULE)["population"];
    assert_eq!(p["examined"], 3, "{p}");
    assert!(
        p.as_object().unwrap().get("deferred_by_scope").is_none(),
        "the counter is absent for a rule that does not declare the option: {p}"
    );
    assert!(scope_notices(&v).is_empty(), "{v}");
}

#[test]
fn a_real_out_of_scope_still_raises_the_scope_warning() {
    // A type with no header: its slots are out of scope for the rule whatever
    // `scope` says, and `config.scope-matches-nothing` still reads that.
    let mut cfg = config("    scope: changed\n");
    cfg.push_str(
        "  note:\n    dir: docs/note\n    prefix: NOTE\n    header_shape: none\n    required_fields: []\n    known_fields: [Embodiment, Realized-by]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n",
    );
    let dir = fixture("out-of-scope", &cfg);
    std::fs::create_dir_all(dir.join("docs/note")).unwrap();
    std::fs::write(dir.join("docs/note/NOTE-1-x.md"), "# 1 - X\n\nbody\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "note"]);
    let (_, v) = urzua(&dir, &["check"]);
    let p = &execution(&v, RULE)["population"];
    assert!(p["out_of_scope"].as_u64().unwrap() > 0, "{p}");
    assert!(p["deferred_by_scope"].as_u64().unwrap() > 0, "{p}");
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "config.scope-matches-nothing"),
        "{v}"
    );
}

#[test]
fn a_waiver_on_a_scope_changed_rule_is_not_reported_as_matching_nothing() {
    let mut cfg = config("    scope: changed\n");
    cfg = cfg.replace(
        "  config.scope-matches-nothing: warn\n",
        "  waiver.matches-nothing: warn\n",
    );
    cfg.push_str("  waiver:\n    dir: docs/waiver\n    required_fields: [Rule, Scope]\n    known_fields: [Expires]\n    header_shape: yaml-frontmatter\n");
    let dir = fixture("waiver", &cfg);
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(
        dir.join("docs/waiver/W-1.md"),
        format!("---\nRule: {RULE}\nScope: docs/adr/ADR-1-a.md\n---\n# Waiver\n"),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "waiver"]);
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        !v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "waiver.matches-nothing"),
        "{v}"
    );
    // Without the option the same waiver, matching no finding, is reported.
    let unscoped = fixture("waiver-unscoped", &cfg.replace("    scope: changed\n", ""));
    std::fs::create_dir_all(unscoped.join("docs/waiver")).unwrap();
    std::fs::write(
        unscoped.join("docs/waiver/W-1.md"),
        format!("---\nRule: {RULE}\nScope: docs/adr/ADR-9-none.md\n---\n# Waiver\n"),
    )
    .unwrap();
    git(&unscoped, &["add", "-A"]);
    git(&unscoped, &["commit", "-q", "-m", "waiver"]);
    let (_, v) = urzua(&unscoped, &["check"]);
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "waiver.matches-nothing"),
        "{v}"
    );
}

#[test]
fn a_bad_value_or_a_rule_that_does_not_take_it_fails_to_load() {
    let dir = fixture("bad-value", &config("    scope: all\n"));
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 2, "{v}");
    assert!(v["error"].as_str().unwrap().contains("'changed'"), "{v}");

    let dir = fixture(
        "wrong-rule",
        &config("").replace(
            "  config.scope-matches-nothing: warn\n",
            "  header.required-fields:\n    level: warn\n    scope: changed\n",
        ),
    );
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 2, "{v}");
    assert!(
        v["error"]
            .as_str()
            .unwrap()
            .contains("rule 'header.required-fields' does not take the option 'scope'"),
        "{v}"
    );
}

// ---------------------------------------------------------------------------
// embodiment.locator-promotion-candidate: the second rule that takes the option.
// A promotion candidate is a locator cited by two or more records, so the unchanged
// citers still count toward the candidate; the finding sits on a changed citer.
// ---------------------------------------------------------------------------

const PROMO: &str = "embodiment.locator-promotion-candidate";

fn promo_config(scope: &str) -> String {
    config("").replace(
        &format!("  {RULE}:\n    level: warn\n    back_pointer_paths: [src]\n"),
        &format!("  {PROMO}:\n    level: warn\n{scope}"),
    )
}

fn promo_findings(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == PROMO)
        .collect()
}

#[test]
fn the_promotion_nudge_without_the_option_reports_the_shared_locator_on_every_run() {
    let dir = fixture("promo-plain", &promo_config(""));
    let (_, v) = urzua(&dir, &["check"]);
    let found = promo_findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(found[0]["file"], PATH_1, "{v}");
    let p = &execution(&v, PROMO)["population"];
    assert_eq!(p["examined"], 3, "{p}");
    assert!(
        p.as_object().unwrap().get("deferred_by_scope").is_none(),
        "{p}"
    );
}

#[test]
fn the_promotion_nudge_with_scope_changed_says_nothing_on_a_plain_check_and_why() {
    let dir = fixture("promo-scope-plain", &promo_config("    scope: changed\n"));
    let (code, v) = urzua(&dir, &["check"]);
    assert_eq!(code, 0, "{v}");
    assert!(promo_findings(&v).is_empty(), "{v}");
    let p = &execution(&v, PROMO)["population"];
    assert_eq!(p["examined"], 0, "{p}");
    assert_eq!(p["deferred_by_scope"], 3, "{p}");
    let notices: Vec<_> = v
        .get("notices")
        .and_then(|n| n.as_array())
        .into_iter()
        .flatten()
        .filter(|n| n["subject"] == PROMO)
        .collect();
    assert_eq!(notices.len(), 1, "{v}");
    assert!(notices[0]["message"]
        .as_str()
        .unwrap()
        .contains("scope: changed"));
}

#[test]
fn the_promotion_nudge_with_scope_changed_fires_on_the_changed_citer_and_names_all_citers() {
    let dir = fixture("promo-scope-changed", &promo_config("    scope: changed\n"));
    std::fs::write(
        dir.join(PATH_2),
        format!("{}\nMore.\n", record(2, Some("code:src/a.rs"))),
    )
    .unwrap();
    let (code, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "{v}");
    let found = promo_findings(&v);
    assert_eq!(found.len(), 1, "{v}");
    assert_eq!(
        found[0]["file"], PATH_2,
        "the finding sits on the changed record: {v}"
    );
    let message = found[0]["message"].as_str().unwrap();
    for path in [PATH_1, PATH_2, "docs/adr/ADR-3-c.md"] {
        assert!(message.contains(path), "{message}");
    }
    let p = &execution(&v, PROMO)["population"];
    assert_eq!(p["examined"], 1, "{p}");
    assert_eq!(p["deferred_by_scope"], 2, "{p}");
}

#[test]
fn the_promotion_nudge_with_scope_changed_is_silent_when_the_changed_record_cites_no_shared_locator(
) {
    let dir = fixture(
        "promo-scope-unrelated",
        &promo_config("    scope: changed\n"),
    );
    std::fs::write(
        dir.join("docs/adr/ADR-4-d.md"),
        format!("{}\nMore.\n", record(4, None)),
    )
    .unwrap();
    let (_, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    assert!(promo_findings(&v).is_empty(), "{v}");
}

#[test]
fn the_promotion_nudge_with_scope_changed_does_not_fire_for_a_locator_with_one_citer() {
    let dir = fixture("promo-scope-single", &promo_config("    scope: changed\n"));
    std::fs::write(dir.join(PATH_2), record(2, Some("code:src/only-here.rs"))).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "ADR-2 cites its own file"]);
    std::fs::write(
        dir.join(PATH_2),
        format!("{}\nMore.\n", record(2, Some("code:src/only-here.rs"))),
    )
    .unwrap();
    let (_, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    assert!(promo_findings(&v).is_empty(), "{v}");
}

#[test]
fn the_promotion_nudge_with_scope_changed_fires_on_every_changed_citer() {
    let mut cfg = promo_config("    scope: changed\n");
    cfg.push_str("  waiver:\n    dir: docs/waiver\n    role: suppression\n    effective_requires: [Reason]\n    required_fields: [Rule, Scope, Reason]\n    known_fields: [Expires]\n    header_shape: yaml-frontmatter\n");
    let dir = fixture("promo-two-changed", &cfg);
    // ADR-1 (already citing src/a.rs) and ADR-2 (newly citing it) both change.
    for (path, n, loc) in [(PATH_1, 1, "code:src/a.rs"), (PATH_2, 2, "code:src/a.rs")] {
        std::fs::write(dir.join(path), format!("{}\nMore.\n", record(n, Some(loc)))).unwrap();
    }
    let (_, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    let found = promo_findings(&v);
    let mut files: Vec<_> = found.iter().map(|f| f["file"].as_str().unwrap()).collect();
    files.sort();
    assert_eq!(files, vec![PATH_1, PATH_2], "{v}");
    let p = &execution(&v, PROMO)["population"];
    assert_eq!(p["examined"], 2, "{p}");
    assert_eq!(p["deferred_by_scope"], 1, "{p}");

    // `check <path>` on each citer sees its own finding.
    for path in [PATH_1, PATH_2] {
        let (_, v) = urzua(&dir, &["check", "--base", "HEAD", path]);
        let found = promo_findings(&v);
        assert_eq!(found.len(), 1, "{path}: {v}");
        assert_eq!(found[0]["file"], path, "{v}");
    }

    // A file-scoped waiver on one citer does not hide the other.
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(
        dir.join("docs/waiver/W-1.md"),
        format!("---\nRule: {PROMO}\nScope: {PATH_1}\nReason: tracked elsewhere\n---\n# Waiver\n"),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let (_, v) = urzua(&dir, &["check", "--base", "HEAD"]);
    let found = promo_findings(&v);
    assert_eq!(found.len(), 2, "{v}");
    for f in found {
        let waived = !f["waived"].is_null();
        assert_eq!(waived, f["file"] == PATH_1, "{f}");
    }
}

/// The finding `RULE` raised on `file`, whole.
fn finding_on(v: &serde_json::Value, file: &str) -> serde_json::Value {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == RULE && f["file"] == file)
        .unwrap_or_else(|| panic!("no {RULE} finding on {file}: {v}"))
        .clone()
}

/// ADR-90 / MILE-203: the option changes which records a rule judges, never what
/// a judged record is told to do. A finding the rule emits, the rule's action
/// variants in `explain --rule`, and the row of `urzua rules` are the same with
/// and without `scope: changed` (ADR-90 point 9: no kind, no `target` key, no
/// variant is added).
#[test]
fn the_adr73_actions_of_a_rule_that_takes_the_option_are_unchanged() {
    let edit = |dir: &Path| {
        std::fs::write(
            dir.join(PATH_2),
            format!("{}\nMore.\n", record(2, Some("code:src/a.rs"))),
        )
        .unwrap();
    };
    let plain = fixture("actions-plain", &config(""));
    let scoped = fixture("actions-scoped", &config("    scope: changed\n"));
    edit(&plain);
    edit(&scoped);

    // The same changed record, nudged by the same diff run.
    let (_, plain_run) = urzua(&plain, &["check", "--base", "HEAD"]);
    let (_, scoped_run) = urzua(&scoped, &["check", "--base", "HEAD"]);
    let without = finding_on(&plain_run, PATH_2);
    let with = finding_on(&scoped_run, PATH_2);
    // The rule declares its variants exempt (the repair is a comment in a source
    // file), so a finding carries no `suggested_action`, with the option or without.
    assert!(
        without.get("suggested_action").is_none(),
        "an exempt variant offers no action: {without}"
    );
    assert_eq!(
        with, without,
        "the finding is byte for byte the same under the option"
    );
    assert_eq!(
        serde_json::to_string(&with).unwrap(),
        serde_json::to_string(&without).unwrap()
    );

    // The rule's declared action variants.
    let (_, plain_rule) = urzua(&plain, &["explain", "--rule", RULE]);
    let (_, scoped_rule) = urzua(&scoped, &["explain", "--rule", RULE]);
    let actions = &plain_rule["rule"]["actions"];
    assert!(
        actions
            .as_array()
            .is_some_and(|a| !a.is_empty() && a.iter().all(|v| v.get("exempt").is_some())),
        "the declared variants are exemptions: {plain_rule}"
    );
    assert_eq!(&scoped_rule["rule"]["actions"], actions, "{scoped_rule}");

    // The rule table every build prints.
    let table_actions = |dir: &Path| {
        let (_, rules) = urzua(dir, &["rules"]);
        let rows = rules["rules"]
            .as_array()
            .unwrap_or_else(|| panic!("{rules}"));
        rows.iter()
            .find(|r| r["id"] == RULE)
            .unwrap_or_else(|| panic!("no row for {RULE}: {rules}"))["actions"]
            .clone()
    };
    assert_eq!(table_actions(&scoped), table_actions(&plain));
}
