//! MILE-178, MILE-204: the record-edit hook (ADR-69, ADR-89). Recorded Claude Code
//! PreToolUse payloads (`tests/fixtures/hook-payloads`) go through the installed
//! translator and the engine core (`urzua harness run edit`) against the real urzua
//! binary in a git fixture. Needs `python3` and `sh`; a missing tool fails the test,
//! never skips it.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success());
}

const CONFIG: &str = "schema_version: 2\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrules: {embodiment.consistency: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n";
const RECORD: &str =
    "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";

/// A clean git repo; `Realized-by: code:src/a.rs` computes `Implemented`.
fn fixture(name: &str, record: &str) -> common::TempDir {
    fixture_with(name, record, CONFIG)
}

fn fixture_with(name: &str, record: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-hook-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    let harness =
        "harness:\n  adapters: [claude-code]\n  guardrails:\n    editor-hook: {enabled: true}\n";
    let config = if config.contains("harness:") {
        config.to_string()
    } else {
        format!("{config}{harness}")
    };
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-a.md"), record).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn payload(name: &str, root: &Path) -> String {
    let p = repo_root().join(format!(
        "rust/crates/urzua-cli/tests/fixtures/hook-payloads/{name}.json"
    ));
    std::fs::read_to_string(p)
        .unwrap()
        .replace("@ROOT@", root.to_str().unwrap())
}

fn run(cmd: &mut Command, root: &Path, stdin: &str) -> Output {
    let mut child = cmd
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash/python3 must be installed");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// The installed translator's stdout as JSON (null when it printed nothing): the hook
/// command the installed settings carry, run the way the agent runs it.
fn adapter(root: &Path, name: &str, env: &[(&str, &str)]) -> serde_json::Value {
    if !root.join(".claude/settings.json").exists() {
        let out = common::urzua()
            .args(["harness", "install"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".claude/settings.json")).unwrap())
            .unwrap();
    let cmd = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    let mut c = Command::new("sh");
    c.args(["-c", &cmd]);
    c.env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env_remove("URZUA_HOOK_OVERRIDE");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = run(&mut c, root, &payload(name, root));
    assert!(out.status.success(), "{out:?}");
    if out.stdout.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_slice(&out.stdout).unwrap()
}

/// The engine core, as the translator calls it: a request on stdin, a verdict on stdout.
/// `args` are `[--override NAME [--reason TEXT]] <record-path>`.
fn core(root: &Path, args: &[&str], env: &[(&str, &str)], stdin: &str) -> (i32, serde_json::Value) {
    let mut request = serde_json::json!({
        "event": "edit", "tool": "Write", "actor": "agent", "cwd": root.to_string_lossy(),
        "content": stdin,
    });
    let mut it = args.iter();
    let mut ovr = serde_json::Map::new();
    while let Some(a) = it.next() {
        match *a {
            "--override" => {
                ovr.insert("name".into(), it.next().unwrap().to_string().into());
            }
            "--reason" => {
                ovr.insert("reason".into(), it.next().unwrap().to_string().into());
            }
            path => request["paths"] = serde_json::json!([path]),
        }
    }
    if !ovr.is_empty() {
        request["override"] = ovr.into();
    }
    let mut c = common::urzua();
    c.args(["harness", "run", "edit"])
        .env_remove("URZUA_HOOK_OVERRIDE")
        .env_remove("URZUA_HOOK_REASON");
    for (k, v) in env {
        c.env(k, v);
    }
    let out = run(&mut c, root, &request.to_string());
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("core stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn hook(v: &serde_json::Value) -> &serde_json::Value {
    &v["hookSpecificOutput"]
}

fn denied(v: &serde_json::Value) -> bool {
    hook(v)["permissionDecision"] == "deny"
}

#[test]
fn a_hand_set_embodiment_is_blocked_with_the_computed_value_and_evidence() {
    let root = fixture("block", RECORD);
    for name in ["write-wrong", "edit-wrong", "multiedit-wrong"] {
        let v = adapter(&root, name, &[]);
        assert!(denied(&v), "{name}: {v}");
        let why = hook(&v)["permissionDecisionReason"].as_str().unwrap();
        assert!(
            why.contains("'Verified'") && why.contains("'Implemented'"),
            "{why}"
        );
        assert!(why.contains("computed from Realized-by"), "{why}");
        assert!(
            !why.contains("(run:"),
            "a repair command cannot apply to an edit that is not on disk: {why}"
        );
        assert!(
            why.contains("[embodiment.consistency]"),
            "names the rule: {why}"
        );
        assert!(
            why.contains("docs/adr/ADR-1-a.md"),
            "names the record: {why}"
        );
    }
}

#[test]
fn the_computed_value_and_edits_to_other_fields_pass() {
    let root = fixture("pass", RECORD);
    // The record on disk already disagrees (Not started vs Implemented); an
    // edit that leaves that field alone is not blocked.
    for name in ["write-correct", "edit-prose"] {
        let v = adapter(&root, name, &[]);
        assert!(!denied(&v), "{name}: {v}");
    }
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], RECORD);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "allow-with-warnings");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("Already failing on disk"),
        "{v}"
    );
}

#[test]
fn an_ungoverned_path_and_other_tools_are_not_touched() {
    let root = fixture("ungoverned", RECORD);
    assert!(adapter(&root, "write-ungoverned", &[]).is_null());
    assert!(adapter(&root, "read-tool", &[]).is_null());
    let (code, v) = core(&root, &["src/a.rs"], &[], "fn a() {}\n");
    assert_eq!((code, v["verdict"].as_str()), (0, Some("not-governed")));
}

#[test]
fn when_urzua_cannot_judge_the_edit_proceeds_and_says_so() {
    let root = fixture("unjudged", RECORD);
    // No rule is enabled: urzua answers not-run, which is not a clean pass.
    let silent = fixture_with(
        "unjudged-no-rules",
        RECORD,
        "schema_version: 2\nrules: {}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    );
    let v = adapter(&silent, "write-no-embodiment", &[]);
    assert!(!denied(&v));
    assert!(
        v["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{v}"
    );
    assert!(hook(&v)["additionalContext"]
        .as_str()
        .unwrap()
        .contains("did NOT judge"));
    // No binary at all.
    let v = adapter(
        &root,
        "write-wrong",
        &[
            ("URZUA_BIN", "/nonexistent/urzua"),
            ("PATH", "/usr/bin:/bin"),
        ],
    );
    assert!(!denied(&v), "{v}");
    assert!(
        v["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{v}"
    );
    // An Edit whose text is not in the file cannot be turned into a proposal.
    let v = adapter(&root, "edit-missing", &[]);
    assert!(
        v["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{v}"
    );
}

/// A depth-1 clone of a fixture whose history has two commits.
fn shallow_clone(name: &str) -> common::TempDir {
    let src = fixture(name, RECORD);
    std::fs::write(src.join("src/a.rs"), "fn a() { }\n").unwrap();
    git(&src, &["commit", "-q", "-am", "second"]);
    let dst = common::TempDir::new(&format!("urzua-hook-{name}-clone"));
    assert!(Command::new("git")
        .args(["clone", "-q", "--depth", "1"])
        .arg(format!("file://{}", src.display()))
        .arg(&dst)
        .status()
        .unwrap()
        .success());
    dst
}

#[test]
fn a_shallow_clone_still_blocks_what_needs_no_history_and_says_what_it_skipped() {
    let dst = shallow_clone("shallow-block");
    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = core(&dst, &["docs/adr/ADR-1-a.md"], &[], &wrong);
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "block", "{v}");
    let msg = v["message"].as_str().unwrap();
    assert!(msg.contains("[embodiment.consistency]"), "{msg}");
    assert!(
        msg.contains("embodiment drift") && msg.contains("shallow clone"),
        "the skipped rule is disclosed: {msg}"
    );
}

#[test]
fn a_shallow_clone_with_nothing_wrong_allows_with_the_skipped_rule_disclosed() {
    let dst = shallow_clone("shallow-clean");
    let ok = RECORD.replace("Not started", "Implemented");
    let (code, v) = core(&dst, &["docs/adr/ADR-1-a.md"], &[], &ok);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "allow-with-warnings", "{v}");
    let msg = v["message"].as_str().unwrap();
    assert!(
        msg.contains("NOT judged")
            && msg.contains("embodiment drift")
            && msg.contains("shallow clone"),
        "{msg}"
    );
}

#[test]
fn a_named_override_lets_the_edit_through_and_appears_in_the_output() {
    let root = fixture("override", RECORD);
    let v = adapter(
        &root,
        "write-wrong",
        &[("URZUA_HOOK_OVERRIDE", "alice-reviewed")],
    );
    assert!(!denied(&v), "{v}");
    let msg = v["systemMessage"].as_str().unwrap();
    assert!(
        msg.contains("OVERRIDE 'alice-reviewed'") && msg.contains("'Implemented'"),
        "{msg}"
    );

    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = core(
        &root,
        &["--override", "bob", "docs/adr/ADR-1-a.md"],
        &[],
        &wrong,
    );
    assert_eq!(
        (code, v["verdict"].as_str(), v["override"].as_str()),
        (0, Some("allow-override"), Some("bob"))
    );

    // An override without a name is not accepted: the block stands.
    let (code, v) = core(
        &root,
        &["--override", "  ", "docs/adr/ADR-1-a.md"],
        &[],
        &wrong,
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")));
    assert!(
        v["message"].as_str().unwrap().contains("needs a name"),
        "{v}"
    );
}

#[test]
fn a_new_record_is_judged_from_the_proposal_alone() {
    let root = fixture("new", RECORD);
    let new = RECORD.replace("ADR-1: T", "ADR-3: V");
    let (code, v) = core(&root, &["docs/adr/ADR-3-c.md"], &[], &new);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
}

const WIDE: &str = "schema_version: 2\nrules: {header.required-fields: error, section.required: warn, embodiment.consistency: error}\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n  none: Not started\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n    sections:\n      from: headings\n      required:\n        - {name: Context}\n";
const GOOD: &str =
    "---\nStatus: Proposed\nDate: 2026-01-01\n---\n# ADR-1: T\n\n## Context\n\nwhy\n";

#[test]
fn a_header_error_in_the_edited_record_blocks_and_a_warning_is_shown() {
    let root = fixture_with("wide", GOOD, WIDE);
    // The Date header is gone: an error the old, Embodiment-only hook never saw.
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], &no_date);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    let msg = v["message"].as_str().unwrap();
    assert!(
        msg.contains("[header.required-fields]") && msg.contains("'Date'"),
        "{msg}"
    );

    // A missing section is a warning here: shown, not blocking.
    let no_context = GOOD.replace("## Context\n\nwhy\n", "prose\n");
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], &no_context);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-with-warnings")),
        "{v}"
    );
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("[section.required]"),
        "{v}"
    );

    // The adapter relays the warning to the agent and does not deny.
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], GOOD);
    assert_eq!((code, v["verdict"].as_str()), (0, Some("allow")), "{v}");
}

#[test]
fn an_error_already_on_disk_does_not_block_an_unrelated_edit() {
    // The record on disk already lacks Date. An edit to its prose is not
    // blocked; an edit that adds a new error is.
    let broken = GOOD.replace("Date: 2026-01-01\n", "");
    let root = fixture_with("preexisting", &broken, WIDE);
    let edited = broken.replace("why", "because");
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], &edited);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-with-warnings")),
        "{v}"
    );
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("Already failing on disk"),
        "{v}"
    );
    let worse = edited.replace(
        "---\nStatus",
        "---\nEmbodiment: Verified\nRealized-by: code:src/a.rs\nStatus",
    );
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], &worse);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("[embodiment.consistency]"),
        "{v}"
    );
}

#[test]
fn a_header_error_can_be_overridden_by_name() {
    let root = fixture_with("wide-override", GOOD, WIDE);
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = core(
        &root,
        &["--override", "carol", "docs/adr/ADR-1-a.md"],
        &[],
        &no_date,
    );
    assert_eq!(
        (code, v["verdict"].as_str(), v["override"].as_str()),
        (0, Some("allow-override"), Some("carol")),
        "{v}"
    );
}

#[test]
fn the_allow_message_names_what_ran_and_what_could_not_be_judged() {
    let root = fixture_with("ran", GOOD, WIDE);
    let (code, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], GOOD);
    assert_eq!((code, v["verdict"].as_str()), (0, Some("allow")), "{v}");
    let msg = v["message"].as_str().unwrap();
    assert!(!msg.contains("every enabled rule"), "{msg}");
    assert!(msg.contains(" rules ran on this record"), "{msg}");
}

#[test]
fn a_rule_anchored_outside_the_record_is_listed_as_not_judged() {
    let config = WIDE.replace("rules: {", "rules: {type.no-declared-spec: warn, ");
    let root = fixture_with("notjudged", GOOD, &config);
    let (_, v) = core(&root, &["docs/adr/ADR-1-a.md"], &[], GOOD);
    let msg = v["message"].as_str().unwrap();
    assert!(msg.contains("type.no-declared-spec"), "{msg}");
}

const LOGGED: &str = "schema_version: 2\noverride_log: .urzua/override-log.md\nrules: {header.required-fields: error, override.unreviewed: warn}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n";

fn urzua_check(dir: &Path) -> serde_json::Value {
    let out = common::urzua()
        .arg("check")
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn an_override_is_recorded_in_the_declared_log_and_check_then_reports_it_unreviewed() {
    let root = fixture_with("logged", GOOD, LOGGED);
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = core(
        &root,
        &[
            "--override",
            "dana",
            "--reason",
            "importing a legacy record",
            "docs/adr/ADR-1-a.md",
        ],
        &[],
        &no_date,
    );
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-override")),
        "{v}"
    );
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("Recorded in .urzua/override-log.md"),
        "{v}"
    );
    let log = std::fs::read_to_string(root.join(".urzua/override-log.md")).unwrap();
    let row = log
        .lines()
        .find(|l| l.contains("dana"))
        .unwrap_or_else(|| panic!("a row for the override: {log}"));
    for cell in [
        "docs/adr/ADR-1-a.md",
        "header.required-fields",
        "importing a legacy record",
    ] {
        assert!(row.contains(cell), "{cell} in {row}");
    }
    assert!(
        row.trim_end().ends_with("| |"),
        "Reviewed-by is blank: {row}"
    );

    // The log is a committed file; check reports the entry until it is reviewed.
    let report = urzua_check(&root);
    let findings = report["findings"].as_array().unwrap();
    let unreviewed: Vec<_> = findings
        .iter()
        .filter(|f| f["rule"] == "override.unreviewed")
        .collect();
    assert_eq!(unreviewed.len(), 1, "{report}");
    assert!(unreviewed[0]["message"]
        .as_str()
        .unwrap()
        .contains("'dana'"));

    // A person reviews it by naming themselves in the last cell.
    let reviewed = log.replace("| |\n", "| erin |\n");
    std::fs::write(root.join(".urzua/override-log.md"), reviewed).unwrap();
    let report = urzua_check(&root);
    assert!(
        !report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "override.unreviewed"),
        "{report}"
    );
}

#[test]
fn with_a_log_declared_an_override_needs_a_reason_and_nothing_is_written_without_one() {
    let root = fixture_with("logged-noreason", GOOD, LOGGED);
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = core(
        &root,
        &["--override", "dana", "docs/adr/ADR-1-a.md"],
        &[],
        &no_date,
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(
        v["message"].as_str().unwrap().contains("needs a reason"),
        "{v}"
    );
    assert!(!root.join(".urzua/override-log.md").exists());
}

#[test]
fn without_a_declared_log_the_override_says_it_is_not_recorded() {
    let root = fixture("unlogged", RECORD);
    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = core(
        &root,
        &["--override", "bob", "docs/adr/ADR-1-a.md"],
        &[],
        &wrong,
    );
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-override")),
        "{v}"
    );
    assert!(
        v["message"].as_str().unwrap().contains("NOT recorded"),
        "{v}"
    );
}

#[test]
fn an_override_that_cannot_be_recorded_is_not_allowed() {
    // The declared log is a directory, so no row can be appended.
    let root = fixture_with("logged-unwritable", GOOD, LOGGED);
    std::fs::create_dir_all(root.join(".urzua/override-log.md")).unwrap();
    std::fs::write(root.join(".urzua/override-log.md/keep"), "x").unwrap();
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = core(
        &root,
        &[
            "--override",
            "dana",
            "--reason",
            "why",
            "docs/adr/ADR-1-a.md",
        ],
        &[],
        &no_date,
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap()
            .contains("could not be recorded"),
        "{v}"
    );
}
