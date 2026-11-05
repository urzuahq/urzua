//! MILE-188: the committed override log and `override.unreviewed`.
//! Real git-repo fixtures, real binary; all output is JSON.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const HEADER: &str = "# Override log\n\n| Date | Name | Record | Field | Rule | Reason | Reviewed-by |\n|---|---|---|---|---|---|---|\n";
const CONFIG: &str = "schema_version: 2\noverride_log: .urzua/override-log.md\nrules: {override.unreviewed: warn}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";

fn fixture(name: &str, config: &str, log: Option<&str>) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("urzua-override-log-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Proposed\n---\n# ADR-1: T\n",
    )
    .unwrap();
    if let Some(log) = log {
        std::fs::write(dir.join(".urzua/override-log.md"), log).unwrap();
    }
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn check(dir: &Path) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn execution(v: &serde_json::Value) -> &serde_json::Value {
    v["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["rule"] == "override.unreviewed")
        .unwrap_or_else(|| panic!("rule listed: {v}"))
}

#[test]
fn an_unreviewed_override_is_a_warning_on_the_log_and_a_reviewed_one_is_not() {
    let log = format!(
        "{HEADER}| 2026-11-05 | alice | docs/adr/ADR-1-a.md | Embodiment | embodiment.consistency | draft ahead of the code | |\n| 2026-11-05 | bob | docs/adr/ADR-1-a.md | Status | status.undeclared-value | fixture | carol |\n"
    );
    let dir = fixture("unreviewed", CONFIG, Some(&log));
    let (code, v) = check(&dir);
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{v}");
    assert_eq!(findings[0]["rule"], "override.unreviewed");
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(findings[0]["file"], ".urzua/override-log.md");
    assert_eq!(findings[0]["line"], 5);
    let msg = findings[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("'alice'") && msg.contains("draft ahead of the code"),
        "{msg}"
    );
    assert_eq!(execution(&v)["population"]["examined"], 2);
    // A warning does not exit 1 under the default fail_on.
    assert_eq!(code, 0);
}

#[test]
fn reviewing_the_entry_clears_the_finding() {
    let log = format!(
        "{HEADER}| 2026-11-05 | alice | docs/adr/ADR-1-a.md | Embodiment | embodiment.consistency | draft | dana |\n"
    );
    let dir = fixture("reviewed", CONFIG, Some(&log));
    let (_, v) = check(&dir);
    assert!(v["findings"].as_array().unwrap().is_empty(), "{v}");
    assert_eq!(execution(&v)["population"]["examined"], 1);
}

#[test]
fn a_row_that_is_not_an_entry_is_reported() {
    let log = format!("{HEADER}| 2026-11-05 | alice | only three cells |\n");
    let dir = fixture("malformed", CONFIG, Some(&log));
    let (_, v) = check(&dir);
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{v}");
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("not an entry"),
        "{v}"
    );
}

#[test]
fn a_log_that_does_not_exist_yet_is_disclosed_as_not_examined() {
    let dir = fixture("absent", CONFIG, None);
    let (_, v) = check(&dir);
    assert!(v["findings"].as_array().unwrap().is_empty(), "{v}");
    let p = &execution(&v)["population"];
    assert_eq!(
        (p["eligible"].as_u64(), p["examined"].as_u64()),
        (Some(1), Some(0)),
        "{v}"
    );
}

#[test]
fn enabling_the_rule_without_declaring_a_log_is_a_could_not_run() {
    let config = CONFIG.replace("override_log: .urzua/override-log.md\n", "");
    let dir = fixture("undeclared", &config, None);
    let (code, v) = check(&dir);
    assert_eq!(code, 2, "{v}");
    assert!(v["error"].as_str().unwrap().contains("override_log"), "{v}");
}

#[test]
fn a_log_path_that_leaves_the_repository_is_refused() {
    let config = CONFIG.replace(".urzua/override-log.md", "../override-log.md");
    let dir = fixture("escapes", &config, None);
    let (code, v) = check(&dir);
    assert_eq!(code, 2, "{v}");
    assert!(v["error"].as_str().unwrap().contains("override_log"), "{v}");
}
