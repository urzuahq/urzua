//! The checklist reference page (`docs/reference/checklist.md`) cannot go
//! stale: its configuration block, its record, its load error and its
//! embodiment block are loaded and judged by the built binary, and the
//! statements it makes about evidence forms and `fix` are checked against it.

mod common;

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn page() -> String {
    std::fs::read_to_string(root().join("docs/reference/checklist.md")).unwrap()
}

/// The fenced block that follows `<!-- test:NAME -->`, with a trailing newline.
fn block(name: &str) -> String {
    let text = page();
    let at = text
        .find(&format!("<!-- test:{name} -->"))
        .unwrap_or_else(|| panic!("the page has no `test:{name}` block"));
    let open = text[at..].find("```").unwrap() + at;
    let start = text[open..].find('\n').unwrap() + open + 1;
    let end = text[start..].find("\n```").unwrap() + start;
    format!("{}\n", &text[start..end])
}

fn repo(label: &str, config: &str, records: &[(&str, String)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-checklist-page-{label}"));
    for d in ["docs/task", "docs/adr", "src", "tests", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("tests/export.rs"), "fn t() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-use-x.md"),
        "---\nStatus: Accepted\n---\n# 1 - Use X\n",
    )
    .unwrap();
    for (name, text) in records {
        std::fs::write(dir.join("docs/task").join(name), text).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["add", "-A"]);
    dir
}

fn page_config() -> String {
    format!("schema_version: 2\nrecord_types:\n{}", block("config"))
}

fn run(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "json: {e}\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap(), json)
}

fn findings<'a>(report: &'a serde_json::Value, rule: &str) -> Vec<&'a serde_json::Value> {
    report["findings"]
        .as_array()
        .map(|a| a.iter().filter(|f| f["rule"] == rule).collect())
        .unwrap_or_default()
}

fn with_status(record: &str, status: &str) -> String {
    record.replacen("Status: InProgress", &format!("Status: {status}"), 1)
}

#[test]
fn the_page_configuration_and_record_load_and_agree() {
    let dir = repo(
        "agree",
        &page_config(),
        &[("TASK-1-export.md", block("record"))],
    );
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["status"], "ok", "{report}");
    assert_eq!(report["files_examined"], 1, "{report}");
    for rule in [
        "status.delivery-agreement",
        "status.done-items-shape",
        "status.waived-item-stale",
    ] {
        let ran = report["rules_executed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["rule"] == rule && e["status"] == "ran");
        assert!(ran, "{rule} runs from the declaration alone: {report}");
    }
}

#[test]
fn the_record_shows_one_item_met_one_unmet_and_one_waived() {
    let record = block("record");
    assert!(record.contains("evidence: code:src/lib.rs"), "{record}");
    assert!(record.contains("waived: not-applicable --"), "{record}");
    let dir = repo(
        "counts",
        &page_config(),
        &[
            ("TASK-1-open.md", with_status(&record, "Open")),
            ("TASK-2-done.md", with_status(&record, "Done")),
        ],
    );
    let (_, report) = run(&dir, &["check"]);
    let found = findings(&report, "status.delivery-agreement");
    let message = |name: &str| {
        found
            .iter()
            .find(|f| f["file"].as_str().unwrap().contains(name))
            .unwrap_or_else(|| panic!("no finding on {name}: {report}"))["message"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(
        message("TASK-1").contains("1 of 3 items resolved, 1 waived")
            && message("TASK-1").contains("set Status to 'InProgress'"),
        "{}",
        message("TASK-1")
    );
    assert!(
        message("TASK-2").contains("\"The report is signed.\" is unresolved: no evidence clause"),
        "{}",
        message("TASK-2")
    );
}

#[test]
fn every_evidence_form_the_page_rejects_is_a_finding() {
    let text = page();
    let rejected: Vec<String> = text
        .lines()
        .filter(|l| l.starts_with('|') && l.contains("**Rejected.**"))
        .map(|l| {
            let cell = l.split('|').nth(1).unwrap();
            cell.split('`').nth(1).unwrap().to_string()
        })
        .collect();
    assert_eq!(
        rejected,
        ["4f2a9c1", "#512", "see the pull request", "spec:ADR-1"],
        "the page rejects a commit hash, a pull request number, free text and a record identifier"
    );
    for form in &rejected[..3] {
        let record = format!(
            "---\nStatus: Open\nKind: feature\nDate: 2026-11-17\nAuthor: kai\nStable-Id: 01JTASK000000000000000001\n---\n# 1 - T\n\n## Acceptance\n\n- Item. — evidence: {form}\n"
        );
        let dir = repo("rejected", &page_config(), &[("TASK-1-t.md", record)]);
        let (_, report) = run(&dir, &["check"]);
        let shape = findings(&report, "status.done-items-shape");
        assert_eq!(shape.len(), 1, "{form}: {report}");
        assert!(
            shape[0]["message"]
                .as_str()
                .unwrap()
                .contains("belongs to no locator entry"),
            "{form}: {report}"
        );
    }
    let record = "---\nStatus: Done\nKind: feature\nDate: 2026-11-17\nAuthor: kai\nStable-Id: 01JTASK000000000000000001\n---\n# 1 - T\n\n## Acceptance\n\n- Item. — evidence: spec:ADR-1\n".to_string();
    let dir = repo("id", &page_config(), &[("TASK-1-t.md", record)]);
    let (_, report) = run(&dir, &["check"]);
    let message = findings(&report, "status.delivery-agreement")[0]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains("it was read as a path"), "{message}");
}

#[test]
fn the_load_error_the_page_prints_is_the_one_the_binary_prints() {
    let broken = page_config().replace("          partial_status: InProgress\n", "");
    let dir = repo("load-error", &broken, &[]);
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 2, "{report}");
    assert_eq!(
        report["error"].as_str().unwrap().trim(),
        block("load-error").trim(),
        "update the page's `test:load-error` block"
    );
}

#[test]
fn the_page_embodiment_block_lets_a_record_be_cited_by_its_path() {
    let config = page_config()
        .replace("schema_version: 2\n", &format!("schema_version: 2\n{}", block("embodiment")))
        .replace(
            "          partial_status: InProgress\n",
            "          partial_status: InProgress\n          resolving_tiers: [Implemented, Verified, Decided]\n",
        )
        + "  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status]\n    header_shape: yaml-frontmatter\n";
    let record = "---\nStatus: Done\nKind: feature\nDate: 2026-11-17\nAuthor: kai\nStable-Id: 01JTASK000000000000000001\n---\n# 1 - T\n\n## Acceptance\n\n- Built. — evidence: code:src/lib.rs\n- Decided. — evidence: record:docs/adr/ADR-1-use-x.md\n".to_string();
    let dir = repo("record-category", &config, &[("TASK-1-t.md", record)]);
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 0, "{report}");
    assert!(
        report["findings"].as_array().unwrap().is_empty(),
        "{report}"
    );
}

#[test]
fn what_fix_does_and_does_not_change_is_as_the_page_says() {
    let record = block("record");
    let met = record.replace("- The report is signed.\n", "");
    let dir = repo(
        "fix",
        &page_config(),
        &[
            ("TASK-1-open.md", with_status(&met, "Open")),
            ("TASK-2-partial.md", with_status(&record, "Open")),
            ("TASK-3-done.md", with_status(&record, "Done")),
        ],
    );
    let (_, report) = run(&dir, &["fix"]);
    let computed = |name: &str| {
        report["repairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["record"].as_str().unwrap().contains(name))
            .map(|r| r["computed_value"].as_str().unwrap().to_string())
    };
    assert_eq!(computed("TASK-1").as_deref(), Some("Done"), "{report}");
    assert_eq!(
        computed("TASK-2").as_deref(),
        Some("InProgress"),
        "{report}"
    );
    assert_eq!(
        computed("TASK-3"),
        None,
        "a finished record is never reopened: {report}"
    );
    assert!(
        report["notices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["message"].as_str().unwrap().contains("not downgraded")),
        "{report}"
    );
}

#[test]
fn a_record_whose_items_carry_no_clause_is_judged_only_once_finished() {
    let body = "- First.\n- Second.\n";
    let record = |status: &str| {
        format!(
            "---\nStatus: {status}\nKind: feature\nDate: 2026-11-17\nAuthor: kai\nStable-Id: 01JTASK000000000000000001\n---\n# 1 - T\n\n## Acceptance\n\n{body}"
        )
    };
    for open in ["Open", "InProgress"] {
        let dir = repo("unjudged", &page_config(), &[("TASK-1-t.md", record(open))]);
        let (code, report) = run(&dir, &["check"]);
        assert_eq!(code, 0, "{open}: {report}");
        assert!(
            report["findings"].as_array().unwrap().is_empty(),
            "{open}: {report}"
        );
    }
    let dir = repo("judged", &page_config(), &[("TASK-1-t.md", record("Done"))]);
    let (code, report) = run(&dir, &["check"]);
    assert_eq!(code, 1, "{report}");
    assert_eq!(
        findings(&report, "status.delivery-agreement").len(),
        1,
        "{report}"
    );
    assert!(
        page().contains("not judged until it reaches a finished status"),
        "the page states it"
    );
}

#[test]
fn the_page_is_linked_from_the_reference_index_and_the_config_page() {
    for file in ["docs/reference/README.md", "docs/reference/config.md"] {
        let text = std::fs::read_to_string(root().join(file)).unwrap();
        assert!(
            text.contains("checklist.md"),
            "{file} links the checklist page"
        );
    }
}
