//! MILE-350, MILE-88: a decided record's body changes only by an added dated
//! amendment section or an added revision-log row. Judged against the merge
//! base's copy, so it runs only on a `--base` run.

mod common;

use std::path::Path;
use std::process::Command;

const RULE: &str = "section.decided-body-changed";
const REC: &str = "docs/adr/ADR-1-x.md";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn config(rule: bool) -> String {
    let rule = if rule {
        format!(
            "rules:\n  {RULE}:\n    level: error\n    decision_types: {{adr: [Accepted]}}\n    appendable_sections: ['^Amendment \\(\\d{{4}}-\\d{{2}}-\\d{{2}}\\)']\n"
        )
    } else {
        "rules:\n  header.required-fields: error\n".to_string()
    };
    format!(
        "schema_version: 2\n{rule}record_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses: {{values: [Proposed, Accepted], success: [Accepted], initial: [Proposed], transitions: {{Proposed: [Accepted]}}}}\n    revision_log: {{marker: \"**Revision log**\"}}\n"
    )
}

fn adr(status: &str, decision: &str, rows: &[&str]) -> String {
    let mut s = format!(
        "---\nStatus: {status}\n---\n# 1 - X\n\n## Decision\n\n{decision}\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n## References\n\nNone.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n"
    );
    for r in rows {
        s.push_str(&format!("> | {r} |\n"));
    }
    s
}

const ROW1: &str = "2026-11-01 | Initial. | **substantive**";
const ROW2: &str = "2026-11-20 | Amended. | **substantive**";
const DECISION: &str = "We use a table.";

fn fixture(name: &str, rule: bool, status: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-decided-{name}"));
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config(rule)).unwrap();
    std::fs::write(dir.join(REC), adr(status, DECISION, &[ROW1])).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    dir
}

fn write(dir: &Path, text: String) {
    std::fs::write(dir.join(REC), text).unwrap();
    git(dir, &["add", "-A"]);
}

fn run(dir: &Path, args: &[&str]) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {out:?}"))
}

fn found(v: &serde_json::Value) -> Vec<&serde_json::Value> {
    v["findings"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|f| f["rule"] == RULE)
        .collect()
}

fn execution(v: &serde_json::Value) -> &serde_json::Value {
    v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == RULE)
        .unwrap_or_else(|| panic!("{RULE} not in rules_executed: {v}"))
}

#[test]
fn a_reworded_decision_is_reported_and_the_rule_off_reports_nothing() {
    let dir = fixture("reword", true, "Accepted");
    write(&dir, adr("Accepted", "We use a list.", &[ROW1]));
    let v = run(&dir, &["--base", "base"]);
    let f = found(&v);
    assert_eq!(f.len(), 1, "{v}");
    assert_eq!(f[0]["line"], 8, "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 1, "{v}");

    let off = fixture("reword-off", false, "Accepted");
    write(&off, adr("Accepted", "We use a list.", &[ROW1]));
    let v = run(&off, &["--base", "base"]);
    assert!(found(&v).is_empty(), "{v}");
}

#[test]
fn an_appended_amendment_and_an_appended_row_are_clean() {
    let dir = fixture("amend", true, "Accepted");
    let amended = adr("Accepted", DECISION, &[ROW1, ROW2]).replace(
        "## References",
        "## Amendment (2026-11-20): wider\n\nMore.\n\n## References",
    );
    write(&dir, amended);
    let v = run(&dir, &["--base", "base"]);
    assert!(found(&v).is_empty(), "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 1, "{v}");
}

#[test]
fn a_formatter_style_rewrite_is_clean() {
    let dir = fixture("format", true, "Accepted");
    let text = adr("Accepted", "We use\na table.", &[ROW1])
        .replace(
            "| A | B |\n|---|---|\n| 1 | 2 |",
            "| A   | B   |\n| --- | --- |\n| 1   | 2   |",
        )
        .replace(
            "> | Date | Change | Class |\n> |---|---|---|",
            "> | Date       | Change   | Class           |\n> | ---------- | -------- | --------------- |",
        );
    write(&dir, text);
    let v = run(&dir, &["--base", "base"]);
    assert!(found(&v).is_empty(), "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 1, "{v}");
}

#[test]
fn an_undated_or_mid_section_amendment_and_a_changed_row_are_reported() {
    let dir = fixture("bad", true, "Accepted");
    write(
        &dir,
        adr("Accepted", DECISION, &[ROW1]).replace(
            "## References",
            "## Amendment: wider\n\nMore.\n\n## References",
        ),
    );
    assert_eq!(found(&run(&dir, &["--base", "base"])).len(), 1, "undated");
    write(
        &dir,
        adr(
            "Accepted",
            "We use\n\n## Amendment (2026-11-20): x\n\na table.",
            &[ROW1],
        ),
    );
    assert_eq!(
        found(&run(&dir, &["--base", "base"])).len(),
        1,
        "mid-section"
    );
    write(
        &dir,
        adr(
            "Accepted",
            DECISION,
            &["2026-11-01 | Edited. | **substantive**"],
        ),
    );
    assert_eq!(
        found(&run(&dir, &["--base", "base"])).len(),
        1,
        "changed row"
    );
}

#[test]
fn a_record_not_decided_at_the_base_is_edited_freely_and_a_whole_run_says_it_did_not_run() {
    let dir = fixture("proposed", true, "Proposed");
    write(&dir, adr("Accepted", "We use a list.", &[ROW1]));
    let v = run(&dir, &["--base", "base"]);
    assert!(found(&v).is_empty(), "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 0, "{v}");

    let v = run(&dir, &[]);
    assert_eq!(execution(&v)["status"], "not-applicable", "{v}");
}

#[test]
fn a_renamed_decided_record_is_compared_with_its_copy_under_the_old_name() {
    let dir = fixture("rename", true, "Accepted");
    git(&dir, &["mv", REC, "docs/adr/ADR-1-y.md"]);
    std::fs::write(
        dir.join("docs/adr/ADR-1-y.md"),
        adr("Accepted", "We use a list.", &[ROW1]),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let v = run(&dir, &["--base", "base"]);
    assert_eq!(found(&v).len(), 1, "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 1, "{v}");

    // A rename that only moves the file reports nothing.
    std::fs::write(
        dir.join("docs/adr/ADR-1-y.md"),
        adr("Accepted", DECISION, &[ROW1]),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let v = run(&dir, &["--base", "base"]);
    assert!(found(&v).is_empty(), "{v}");
}

#[test]
fn a_renamed_decided_record_is_reported_with_the_pin_rule_off() {
    let dir = fixture("rename-pins-off", true, "Accepted");
    let off = config(true).replacen(
        "rules:\n",
        "rules:\n  diff.pin-changed-unattested: off\n",
        1,
    );
    // The base's level holds for the diff, so the rule is off at the base too.
    std::fs::write(dir.join(".urzua/config.yaml"), off).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "pins off"]);
    git(&dir, &["tag", "-f", "base"]);
    git(&dir, &["mv", REC, "docs/adr/ADR-1-y.md"]);
    std::fs::write(
        dir.join("docs/adr/ADR-1-y.md"),
        adr("Accepted", "We use a list.", &[ROW1]),
    )
    .unwrap();
    git(&dir, &["add", "-A"]);
    let v = run(&dir, &["--base", "base"]);
    assert_eq!(found(&v).len(), 1, "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 1, "{v}");
}

#[test]
fn a_prefix_lines_record_is_judged_on_its_base_status_and_its_body_alone() {
    let dir = common::TempDir::new("urzua-decided-prefix");
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    let cfg = config(true).replace(
        "    header_shape: \"yaml-frontmatter\"\n",
        "    fields: {from: prefix-lines}\n",
    );
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    let doc = |status: &str, date: &str, decision: &str| {
        format!(
            "# 1 - X\n\nStatus: {status}\nDate: {date}\n\n## Decision\n\n{decision}\n\n## References\n\nNone.\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | {ROW1} |\n"
        )
    };
    std::fs::write(dir.join(REC), doc("Accepted", "2026-11-01", DECISION)).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);

    write(&dir, doc("Accepted", "2026-11-20", DECISION));
    let v = run(&dir, &["--base", "base"]);
    assert!(
        found(&v).is_empty(),
        "a header-field change is not a body change: {v}"
    );
    assert_eq!(execution(&v)["population"]["examined"], 1, "{v}");

    write(&dir, doc("Accepted", "2026-11-01", "We use a list."));
    let v = run(&dir, &["--base", "base"]);
    assert_eq!(found(&v).len(), 1, "{v}");
    assert_eq!(found(&v)[0]["line"], 8, "{v}");

    // The title above the header is body text too.
    write(
        &dir,
        doc("Accepted", "2026-11-01", DECISION).replace("# 1 - X", "# 1 - No queue at all"),
    );
    let v = run(&dir, &["--base", "base"]);
    assert_eq!(found(&v).len(), 1, "{v}");
    assert_eq!(found(&v)[0]["line"], 1, "{v}");
}
