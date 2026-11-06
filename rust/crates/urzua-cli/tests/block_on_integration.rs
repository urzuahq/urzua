//! MILE-195, MILE-204: the editor hook's `block_on: introduced | all` option (`ADR-78`
//! amendment of 2026-11-05, `SPEC-3`, `SPEC-8`), run through the engine core
//! (`urzua harness run edit`) and the installed translator. Needs `python3` and `sh`;
//! a missing tool fails the test, never skips it.

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

const CONFIG: &str = "schema_version: 2\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrules: {embodiment.consistency: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\nharness:\n  adapters: [claude-code]\n  guardrails:\n    editor-hook: {enabled: true}\n";
/// `Realized-by: code:` computes `Implemented`, so the on-disk record already
/// carries an `embodiment.consistency` error.
const RECORD: &str =
    "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";
const PATH: &str = "docs/adr/ADR-1-a.md";

fn with_key(line: &str) -> String {
    CONFIG.replacen(
        "schema_version: 2\n",
        &format!("schema_version: 2\n{line}\n"),
        1,
    )
}

fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-block-on-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(PATH), RECORD).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
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

/// The engine core, as the translator calls it: a request on stdin, a verdict on stdout.
/// `args` are `[--override NAME [--reason TEXT]] <record-path>`.
fn core(root: &Path, args: &[&str], stdin: &str) -> (i32, serde_json::Value) {
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
    let mut c = Command::new(env!("CARGO_BIN_EXE_urzua"));
    c.args(["harness", "run", "edit"])
        .env_remove("URZUA_HOOK_OVERRIDE")
        .env_remove("URZUA_HOOK_REASON");
    let out = run(&mut c, root, &request.to_string());
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("core stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn check(root: &Path) -> (i32, serde_json::Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .arg("check")
        .current_dir(root)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

#[test]
fn introduced_does_not_block_an_error_already_on_disk_and_lists_it_as_existing() {
    for (name, config) in [
        ("absent", CONFIG.to_string()),
        ("explicit", with_key("block_on: introduced")),
    ] {
        let root = fixture(name, &config);
        let (code, v) = core(&root, &[PATH], RECORD);
        assert_eq!(
            (code, v["verdict"].as_str()),
            (0, Some("allow-with-warnings")),
            "{name}: {v}"
        );
        assert!(
            v["message"]
                .as_str()
                .unwrap()
                .contains("Already failing on disk"),
            "{name}: {v}"
        );
    }
}

#[test]
fn an_explicit_introduced_is_byte_identical_to_an_absent_key() {
    let absent = fixture("same-absent", CONFIG);
    let explicit = fixture("same-explicit", &with_key("block_on: introduced"));
    let a = core(&absent, &[PATH], RECORD);
    let b = core(&explicit, &[PATH], RECORD);
    assert_eq!(a, b);
    let wrong = RECORD.replace(
        "Realized-by: code:src/a.rs",
        "Realized-by: code:src/a.rs\nEmbodiment: Verified",
    );
    let a = core(&absent, &[PATH], &wrong);
    let b = core(&explicit, &[PATH], &wrong);
    assert_eq!(a, b);
}

#[test]
fn all_blocks_an_error_already_on_disk_and_lists_no_existing_set() {
    let root = fixture("all", &with_key("block_on: all"));
    let (code, v) = core(&root, &[PATH], RECORD);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    let msg = v["message"].as_str().unwrap();
    assert!(
        msg.contains("[embodiment.consistency]"),
        "lists the error: {msg}"
    );
    assert!(!msg.contains("Already failing"), "no existing set: {msg}");
    assert_eq!(v["findings"].as_array().unwrap().len(), 1, "{v}");
}

#[test]
fn all_lets_a_clean_proposal_through() {
    let root = fixture("all-clean", &with_key("block_on: all"));
    let fixed = RECORD.replace("Not started", "Implemented");
    let (code, v) = core(&root, &[PATH], &fixed);
    assert_eq!((code, v["verdict"].as_str()), (0, Some("allow")), "{v}");
}

/// The hook command the installed settings carry for an edit, run the way the agent runs it.
fn translator(root: &Path) -> Command {
    let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(["harness", "install"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".claude/settings.json")).unwrap())
            .unwrap();
    let cmd = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    let mut c = Command::new("sh");
    c.args(["-c", &cmd])
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env_remove("URZUA_HOOK_OVERRIDE");
    c
}

#[test]
fn all_is_relayed_unchanged_by_the_claude_code_adapter() {
    let root = fixture("all-adapter", &with_key("block_on: all"));
    let payload = std::fs::read_to_string(
        repo_root().join("rust/crates/urzua-cli/tests/fixtures/hook-payloads/edit-prose.json"),
    )
    .unwrap()
    .replace("@ROOT@", root.to_str().unwrap());
    let mut c = translator(&root);
    let out = run(&mut c, &root, &payload);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny", "{v}");

    // The same edit passes under the default.
    let default = fixture("all-adapter-default", CONFIG);
    let payload = payload.replace(root.to_str().unwrap(), default.to_str().unwrap());
    let mut c = translator(&default);
    let out = run(&mut c, &default, &payload);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_ne!(v["hookSpecificOutput"]["permissionDecision"], "deny", "{v}");
}

#[test]
fn the_named_override_works_under_all_and_is_recorded() {
    let config = with_key("block_on: all").replacen(
        "schema_version: 2\n",
        "schema_version: 2\noverride_log: .urzua/override-log.md\n",
        1,
    );
    let root = fixture("all-override", &config);
    let (code, v) = core(
        &root,
        &["--override", "erin", "--reason", "legacy record", PATH],
        RECORD,
    );
    assert_eq!(
        (code, v["verdict"].as_str(), v["override"].as_str()),
        (0, Some("allow-override"), Some("erin")),
        "{v}"
    );
    let log = std::fs::read_to_string(root.join(".urzua/override-log.md")).unwrap();
    assert!(
        log.contains("| erin |")
            && log.contains("embodiment.consistency")
            && log.contains("legacy record"),
        "{log}"
    );
}

#[test]
fn an_invalid_value_is_a_load_error_naming_the_key_and_the_value() {
    let root = fixture("invalid", &with_key("block_on: sometimes"));
    let (code, v) = check(&root);
    assert_eq!(code, 2, "{v}");
    let err = v["error"].as_str().unwrap();
    assert!(
        err.contains("block_on") && err.contains("sometimes"),
        "{err}"
    );
}

#[test]
fn the_hook_names_an_invalid_value_and_says_it_did_not_judge() {
    let root = fixture("invalid-hook", &with_key("block_on: sometimes"));
    let (code, v) = core(&root, &[PATH], RECORD);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
    let msg = v["message"].as_str().unwrap();
    assert!(
        msg.contains("block_on") && msg.contains("sometimes"),
        "{msg}"
    );
}

#[test]
fn both_valid_values_load() {
    for value in ["introduced", "all"] {
        let root = fixture(
            &format!("valid-{value}"),
            &with_key(&format!("block_on: {value}")),
        );
        let (code, v) = check(&root);
        assert_ne!(code, 2, "{value}: {v}");
    }
}
