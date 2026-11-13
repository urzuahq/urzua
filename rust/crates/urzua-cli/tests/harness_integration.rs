//! MILE-198, ADR-89: `urzua harness` installs an agent harness bundle and runs
//! its cores. Real git-repo fixtures, the real binary; all output is JSON.
//! Needs `python3` and `sh` (the adapter's translator runs under it); a missing
//! tool fails a test, never skips it.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const BASE: &str = "schema_version: 2\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n    - {state: Specified, evidence: spec}\n  none: Not started\nrules: {embodiment.consistency: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n";
const HARNESS: &str = "harness:\n  adapters: [claude-code]\n  instructions_file: AGENTS.md\n  guardrails:\n    editor-hook: {enabled: true}\n    session-start: {enabled: true}\n";
const RECORD: &str =
    "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: T\n\nprose\n";
const PATH_1: &str = "docs/adr/ADR-1-a.md";
const AGENTS: &str = "# Agents\n\nKeep this text.\n";

fn config(extra: &str) -> String {
    format!("{BASE}{HARNESS}{extra}")
}

/// A clean git repo: a config, one record whose `Realized-by` computes
/// `Implemented`, an instructions file, and nothing installed.
fn fixture(name: &str, config: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-harness-{name}"));
    for d in ["docs/adr", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), config).unwrap();
    std::fs::write(dir.join("src/a.rs"), "fn a() {}\n").unwrap();
    std::fs::write(dir.join(PATH_1), RECORD).unwrap();
    std::fs::write(dir.join("AGENTS.md"), AGENTS).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> Command {
    let mut c = common::urzua();
    c.args(args).current_dir(dir);
    c
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"))
}

fn harness(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let mut all = vec!["harness"];
    all.extend_from_slice(args);
    let out = urzua(dir, &all).output().unwrap();
    (out.status.code().unwrap(), json(&out))
}

fn install(dir: &Path) -> serde_json::Value {
    let (code, v) = harness(dir, &["install"]);
    assert_eq!(code, 0, "{v}");
    v
}

fn states(v: &serde_json::Value) -> Vec<(String, String)> {
    v["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["path"].as_str().unwrap().to_string(),
                i["state"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn state_of(v: &serde_json::Value, needle: &str) -> String {
    v["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            i["path"].as_str().unwrap().contains(needle)
                || i["key"].as_str().is_some_and(|k| k.contains(needle))
        })
        .unwrap_or_else(|| panic!("no item {needle}: {v}"))["state"]
        .as_str()
        .unwrap()
        .to_string()
}

const TRANSLATOR: &str = ".claude/hooks/urzua-claude-code.py";
const SETTINGS: &str = ".claude/settings.json";
const LOCK: &str = ".urzua/harness.lock";

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

// ---------------------------------------------------------------- install

#[test]
fn install_writes_the_translator_settings_block_and_lock_and_dry_run_writes_nothing() {
    let dir = fixture("install", &config(""));
    let (code, dry) = harness(&dir, &["install", "--dry-run"]);
    assert_eq!(code, 0, "{dry}");
    assert_eq!(dry["dry_run"], true);
    for rel in [TRANSLATOR, SETTINGS, LOCK] {
        assert!(!dir.join(rel).exists(), "{rel} was written by a dry run");
    }
    assert_eq!(read(&dir, "AGENTS.md"), AGENTS);

    let v = install(&dir);
    assert_eq!(v["status"], "ok");
    for rel in [TRANSLATOR, SETTINGS, LOCK] {
        assert!(dir.join(rel).is_file(), "{rel}");
    }
    // The dry run listed the same things the install did.
    let paths = |v: &serde_json::Value| {
        let mut p: Vec<String> = states(v).into_iter().map(|(p, _)| p).collect();
        p.sort();
        p
    };
    assert_eq!(paths(&dry), paths(&v));
    // The block sits between markers at the end; the rest of the file is untouched.
    let agents = read(&dir, "AGENTS.md");
    assert!(agents.starts_with(AGENTS), "{agents}");
    assert!(
        agents.contains("<!-- urzua-harness:begin -->")
            && agents.contains("<!-- urzua-harness:end -->")
    );
    // The merged settings run the engine, not a script of the repository.
    let settings = read(&dir, SETTINGS);
    assert!(
        settings.contains("urzua harness run edit")
            && settings.contains("urzua harness run session-start")
    );
    assert!(settings.contains("Write|Edit|MultiEdit"));
    let lock = read(&dir, LOCK);
    assert!(
        lock.contains("\"claude-code\"") && lock.contains(TRANSLATOR) && lock.contains("sha256:")
    );
}

#[test]
fn a_second_install_changes_nothing_byte_for_byte() {
    let dir = fixture("idempotent", &config(""));
    install(&dir);
    let before: Vec<String> = [TRANSLATOR, SETTINGS, LOCK, "AGENTS.md"]
        .iter()
        .map(|r| read(&dir, r))
        .collect();
    let v = install(&dir);
    assert!(
        states(&v)
            .iter()
            .all(|(_, s)| s == "unchanged" || s == "written"),
        "{v}"
    );
    let after: Vec<String> = [TRANSLATOR, SETTINGS, LOCK, "AGENTS.md"]
        .iter()
        .map(|r| read(&dir, r))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn an_absent_harness_key_installs_and_enables_nothing() {
    let dir = fixture("absent", BASE);
    let (code, v) = harness(&dir, &["install"]);
    assert_eq!(code, 0, "{v}");
    assert!(v["items"].as_array().unwrap().is_empty(), "{v}");
    assert!(
        v["notices"][0]["message"]
            .as_str()
            .unwrap()
            .contains("nothing was installed"),
        "{v}"
    );
    for rel in [TRANSLATOR, SETTINGS, LOCK] {
        assert!(!dir.join(rel).exists());
    }
    // The core judges nothing: the guardrail is not enabled, and it says so.
    let (code, verdict) = edit(&dir, PATH_1, RECORD, "agent", &[]);
    assert_eq!(code, 0);
    assert_eq!(verdict["verdict"], "allow-unjudged", "{verdict}");
    assert!(
        verdict["message"].as_str().unwrap().contains("not enabled"),
        "{verdict}"
    );
}

#[test]
fn the_adapters_command_lists_the_adapter_directories_of_the_bundle() {
    let dir = fixture("adapters", BASE);
    let (code, v) = harness(&dir, &["adapters"]);
    assert_eq!(code, 0, "{v}");
    let adapters = v["adapters"].as_array().unwrap();
    assert_eq!(adapters.len(), 1);
    assert_eq!(adapters[0]["name"], "claude-code");
    assert_eq!(adapters[0]["files"][0], TRANSLATOR);
    assert!(adapters[0]["provides"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p == "editor-hook"));
}

/// `ADR-89` amendment (2026-11-07): a name `harness.guardrails` does not ship is a
/// config load error for every command, so `check` exits 2 instead of running
/// without the gate the author meant, and `doctor` does not call the config clean.
#[test]
fn a_misspelled_guardrail_stops_check_and_doctor_names_it_observed_failing() {
    let bad = config("").replace(
        "    session-start",
        "    acceptence: {enabled: true}\n    session-start",
    );
    let dir = fixture("misspelled-guardrail", &bad);
    let out = urzua(&dir, &["check"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let v = json(&out);
    let error = v["error"].as_str().unwrap();
    assert!(error.contains("'acceptence'"), "{v}");
    assert!(error.contains("did you mean 'acceptance'?"), "{v}");

    let out = urzua(&dir, &["doctor"]).output().unwrap();
    let v = json(&out);
    let checks = v["checks"].as_array().unwrap();
    let parses = checks
        .iter()
        .find(|c| c["check"] == "config-parses")
        .unwrap();
    assert_eq!(parses["status"], "error", "{v}");
    assert!(
        parses["message"].as_str().unwrap().contains("acceptence"),
        "{v}"
    );
    assert!(
        !checks.iter().any(|c| c["message"]
            .as_str()
            .unwrap_or_default()
            .contains("no unrecognized keys")),
        "{v}"
    );
}

#[test]
fn doctor_does_not_claim_no_unrecognized_keys_beside_an_unknown_harness_key_observed_failing() {
    let bad = config("").replace(
        "  instructions_file: AGENTS.md\n",
        "  instructions_file: AGENTS.md\n  teleport: true\n",
    );
    let dir = fixture("doctor-unknown-key", &bad);
    let out = urzua(&dir, &["doctor"]).output().unwrap();
    let v = json(&out);
    let checks = v["checks"].to_string();
    assert!(
        checks.contains("teleport") && checks.contains("harness-config"),
        "{checks}"
    );
    assert!(!checks.contains("no unrecognized keys"), "{checks}");
}

#[test]
fn an_unknown_adapter_guardrail_or_key_is_reported_and_an_unprovided_guardrail_is_never_skipped() {
    let dir = fixture("unknown-adapter", &config(""));
    let (code, v) = harness(&dir, &["install", "--adapter", "no-such-agent"]);
    assert_eq!(code, 2, "{v}");
    assert!(
        v["problems"][0].as_str().unwrap().contains("claude-code"),
        "{v}"
    );
    assert!(!dir.join(LOCK).exists());

    let bad = config("").replace(
        "  instructions_file: AGENTS.md\n",
        "  instructions_file: AGENTS.md\n  teleport: true\n",
    );
    let dir = fixture("unknown-key", &bad);
    let (code, v) = harness(&dir, &["install"]);
    assert_eq!(code, 2, "{v}");
    assert!(
        v["problems"][0].as_str().unwrap().contains("'teleport'"),
        "{v}"
    );
    assert!(
        !dir.join(LOCK).exists(),
        "nothing is written when the harness key is wrong"
    );
    let out = urzua(&dir, &["doctor"]).output().unwrap();
    let checks = json(&out)["checks"].to_string();
    assert!(
        checks.contains("teleport") && checks.contains("harness-config"),
        "{checks}"
    );

    // A guardrail no chosen adapter provides is reported, never skipped. The
    // bundled adapter provides every guardrail the engine knows, so the fixture
    // names no adapter: `acceptance` is enabled (with a well-formed authority
    // table) and nothing can wire it.
    let wants_more = config("")
        .replace("  adapters: [claude-code]\n", "  adapters: []\n")
        .replace(
            "    required_fields: []\n",
            "    required_fields: []\n    statuses:\n      values: [Proposed, Accepted]\n",
        )
        .replace(
            "    session-start",
            "    acceptance: {enabled: true}\n    session-start",
        )
        + "authority:\n  deciders:\n    maintainers: {logins: [alice]}\n  gates:\n    - {type: adr, to: [Accepted], by: maintainers}\n";
    let dir = fixture("unprovided", &wants_more);
    let (code, v) = harness(&dir, &["install"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        v.to_string().contains("no adapter is named"),
        "install says nothing was installed: {v}"
    );
    assert!(!dir.join(LOCK).exists());
    // And doctor says it too.
    let out = urzua(&dir, &["doctor"]).output().unwrap();
    let checks = json(&out)["checks"].to_string();
    assert!(
        checks.contains("acceptance") && checks.contains("harness-config"),
        "{checks}"
    );
}

#[test]
fn a_gate_enabled_without_an_authority_table_does_not_load() {
    let cfg = config("").replace(
        "    session-start",
        "    acceptance: {enabled: true}\n    session-start",
    );
    let dir = fixture("no-authority", &cfg);
    for args in [vec!["check"], vec!["harness", "install"]] {
        let out = urzua(&dir, &args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let v = json(&out);
        assert!(v["error"].as_str().unwrap().contains("'acceptance'"), "{v}");
    }
}

// ----------------------------------------------------------- the merge rules

const PERSON: &str = "{\n  \"model\": \"x\",\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Bash\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"mine\"\n          }\n        ]\n      }\n    ]\n  },\n  \"permissions\": {\n    \"allow\": [\n      \"Read\"\n    ]\n  }\n}";

#[test]
fn an_existing_settings_file_is_merged_and_every_other_key_and_entry_stays_as_written() {
    let dir = fixture("merge", &config(""));
    std::fs::create_dir_all(dir.join(".claude")).unwrap();
    std::fs::write(dir.join(SETTINGS), PERSON).unwrap();
    install(&dir);
    let merged = read(&dir, SETTINGS);
    assert!(merged.contains("\"command\": \"mine\""));
    assert!(merged.contains("\"permissions\": {\n    \"allow\": [\n      \"Read\"\n    ]\n  }"));
    assert!(merged.starts_with("{\n  \"model\": \"x\",\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Bash\""));
    assert!(merged.contains("urzua harness run edit"));
    // A second install changes nothing.
    install(&dir);
    assert_eq!(read(&dir, SETTINGS), merged);
    // Uninstalling takes out only urzua's entries and restores the file.
    let (code, v) = harness(&dir, &["uninstall"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(read(&dir, SETTINGS), PERSON);
}

#[test]
fn a_settings_file_that_is_not_json_or_holds_an_edited_entry_is_refused() {
    let dir = fixture("bad-json", &config(""));
    std::fs::create_dir_all(dir.join(".claude")).unwrap();
    std::fs::write(dir.join(SETTINGS), "{ not json").unwrap();
    let (code, v) = harness(&dir, &["install"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["status"], "not-run");
    assert!(v["problems"][0].as_str().unwrap().contains(SETTINGS), "{v}");
    assert_eq!(read(&dir, SETTINGS), "{ not json");
    assert!(!dir.join(LOCK).exists() && !dir.join(TRANSLATOR).exists());

    // An entry with our key that a person edited.
    let dir = fixture("edited-entry", &config(""));
    install(&dir);
    let edited =
        read(&dir, SETTINGS).replace("urzua harness run edit", "urzua harness run edit --quiet");
    std::fs::write(dir.join(SETTINGS), &edited).unwrap();
    let (code, v) = harness(&dir, &["install"]);
    assert_eq!(code, 2, "{v}");
    assert!(
        v["problems"][0]
            .as_str()
            .unwrap()
            .contains("PreToolUse|Write|Edit|MultiEdit|NotebookEdit|mcp__.*|edit"),
        "{v}"
    );
    assert_eq!(
        read(&dir, SETTINGS),
        edited,
        "the edited file is left as it is"
    );
}

#[test]
fn the_instructions_block_is_rewritten_only_between_its_markers() {
    let dir = fixture("block", &config(""));
    install(&dir);
    let once = read(&dir, "AGENTS.md");
    // A person's text before and after the block, and a stale block body.
    let stale = once.replace("Do not accept or reject", "STALE LINE");
    let tail = "\n## After\n\nTail text.\n";
    std::fs::write(dir.join("AGENTS.md"), format!("{stale}{tail}")).unwrap();
    let (code, v) = harness(&dir, &["update"]);
    // The block changed since install, so `update` leaves it and reports it.
    assert_eq!(code, 1, "{v}");
    assert_eq!(state_of(&v, "AGENTS.md"), "kept-modified");
    let (code, v) = harness(&dir, &["update", "--force"]);
    assert_eq!(code, 0, "{v}");
    let fixed = read(&dir, "AGENTS.md");
    assert!(
        fixed.starts_with(AGENTS) && fixed.ends_with(tail),
        "{fixed}"
    );
    assert_eq!(fixed, format!("{once}{tail}"));
    // And installing again rewrites identical bytes.
    install(&dir);
    assert_eq!(read(&dir, "AGENTS.md"), fixed);
    // Uninstall removes the block and leaves the person's text.
    harness(&dir, &["uninstall"]);
    assert_eq!(read(&dir, "AGENTS.md"), format!("{AGENTS}{tail}"));
}

// ------------------------------------------------ status, update, uninstall

#[test]
fn status_reports_a_missing_a_modified_and_an_out_of_date_file_and_exits_1() {
    let dir = fixture("status", &config(""));
    install(&dir);
    let (code, v) = harness(&dir, &["status"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["status"], "ok");

    // Modified.
    let original = read(&dir, TRANSLATOR);
    std::fs::write(dir.join(TRANSLATOR), format!("{original}# local change\n")).unwrap();
    let (code, v) = harness(&dir, &["status"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["status"], "drift");
    assert_eq!(state_of(&v, "urzua-claude-code.py"), "modified");

    // Missing.
    std::fs::remove_file(dir.join(TRANSLATOR)).unwrap();
    let (code, v) = harness(&dir, &["status"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(state_of(&v, "urzua-claude-code.py"), "missing");

    // Out of date: the file is what an older bundle wrote and the lock agrees,
    // but this binary's bundle differs.
    let older = format!("{original}# older bundle\n");
    std::fs::write(dir.join(TRANSLATOR), &older).unwrap();
    let hash = format!("sha256:{}", urzua_io::sha256_hex_of_str(&older));
    let lock = read(&dir, LOCK);
    let lock = replace_hash(&lock, TRANSLATOR, &hash);
    std::fs::write(dir.join(LOCK), lock).unwrap();
    let (code, v) = harness(&dir, &["status"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(state_of(&v, "urzua-claude-code.py"), "out-of-date");

    // A hook entry taken out of the settings is missing too.
    std::fs::write(dir.join(SETTINGS), "{}\n").unwrap();
    let (_, v) = harness(&dir, &["status"]);
    assert_eq!(state_of(&v, "session-start"), "missing");
    // The shared files that status cannot see are said to be unseen.
    assert!(
        v["notices"].to_string().contains("settings.local.json"),
        "{v}"
    );
}

fn replace_hash(lock: &str, path: &str, hash: &str) -> String {
    let mut v: serde_json::Value = serde_json::from_str(lock).unwrap();
    v["files"][path] = serde_json::Value::String(hash.to_string());
    format!("{}\n", serde_json::to_string_pretty(&v).unwrap())
}

#[test]
fn update_rewrites_an_unmodified_file_and_reports_a_modified_one_unless_forced() {
    let dir = fixture("update", &config(""));
    install(&dir);
    let original = read(&dir, TRANSLATOR);
    // An older bundle's file, unmodified since install: update replaces it.
    let older = format!("{original}# older bundle\n");
    std::fs::write(dir.join(TRANSLATOR), &older).unwrap();
    let lock = replace_hash(
        &read(&dir, LOCK),
        TRANSLATOR,
        &format!("sha256:{}", urzua_io::sha256_hex_of_str(&older)),
    );
    std::fs::write(dir.join(LOCK), lock).unwrap();
    let (code, v) = harness(&dir, &["update", "--dry-run"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(read(&dir, TRANSLATOR), older, "a dry run writes nothing");
    let (code, v) = harness(&dir, &["update"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(state_of(&v, "urzua-claude-code.py"), "updated");
    assert_eq!(read(&dir, TRANSLATOR), original);
    assert_eq!(harness(&dir, &["status"]).0, 0);

    // A file a person changed is reported and left.
    let mine = format!("{original}# mine\n");
    std::fs::write(dir.join(TRANSLATOR), &mine).unwrap();
    let (code, v) = harness(&dir, &["update"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(state_of(&v, "urzua-claude-code.py"), "kept-modified");
    assert_eq!(read(&dir, TRANSLATOR), mine);
    // Still drifted afterwards: the lock kept the old hash.
    assert_eq!(harness(&dir, &["status"]).0, 1);
    let (code, v) = harness(&dir, &["update", "--force"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(read(&dir, TRANSLATOR), original);
}

#[test]
fn uninstall_removes_what_matches_the_lock_and_reports_what_a_person_changed() {
    let dir = fixture("uninstall", &config(""));
    install(&dir);
    let original = read(&dir, TRANSLATOR);
    std::fs::write(dir.join(TRANSLATOR), format!("{original}# mine\n")).unwrap();
    let (code, v) = harness(&dir, &["uninstall"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(state_of(&v, "urzua-claude-code.py"), "kept-modified");
    assert!(
        dir.join(TRANSLATOR).exists(),
        "a changed file is not deleted"
    );
    assert!(!dir.join(LOCK).exists(), "the lock goes last, always");
    assert!(
        !dir.join(SETTINGS).exists(),
        "a settings file the install created goes with its entries"
    );
    assert_eq!(read(&dir, "AGENTS.md"), AGENTS);

    let dir = fixture("uninstall-clean", &config(""));
    install(&dir);
    let (code, _) = harness(&dir, &["uninstall"]);
    assert_eq!(code, 0);
    for rel in [TRANSLATOR, SETTINGS, LOCK] {
        assert!(!dir.join(rel).exists(), "{rel}");
    }
    assert!(
        !dir.join(".claude").exists(),
        "the empty directories go too"
    );
    assert_eq!(
        harness(&dir, &["status"]).0,
        2,
        "no lock, nothing to compare"
    );
}

// ------------------------------------------------------ the editor hook core

fn edit(
    dir: &Path,
    path: &str,
    content: &str,
    actor: &str,
    env: &[(&str, &str)],
) -> (i32, serde_json::Value) {
    edit_request(
        dir,
        serde_json::json!({
            "event": "edit", "tool": "Write", "paths": [path], "content": content,
            "actor": actor, "cwd": dir.to_string_lossy(),
        }),
        env,
    )
}

fn edit_request(
    dir: &Path,
    request: serde_json::Value,
    env: &[(&str, &str)],
) -> (i32, serde_json::Value) {
    run_core(dir, "edit", &request.to_string(), env)
}

fn run_core(
    dir: &Path,
    event: &str,
    stdin: &str,
    env: &[(&str, &str)],
) -> (i32, serde_json::Value) {
    let mut c = urzua(dir, &["harness", "run", event]);
    c.env_remove("URZUA_HOOK_OVERRIDE")
        .env_remove("URZUA_HOOK_REASON");
    for (k, v) in env {
        c.env(k, v);
    }
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.code().unwrap(), json(&out))
}

fn msg(v: &serde_json::Value) -> &str {
    v["message"].as_str().unwrap()
}

#[test]
fn a_hand_set_embodiment_is_blocked_with_the_computed_value_and_evidence() {
    let dir = fixture("block", &config(""));
    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = edit(&dir, PATH_1, &wrong, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    let why = msg(&v);
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
        why.contains("[embodiment.consistency]") && why.contains(PATH_1),
        "{why}"
    );
    assert_eq!(v["findings"][0]["rule"], "embodiment.consistency");
}

#[test]
fn the_computed_value_and_an_error_already_on_disk_pass() {
    let dir = fixture("pass", &config(""));
    // The record on disk disagrees (Not started vs Implemented); an edit that
    // leaves the field alone is not blocked, and the message lists it.
    let (code, v) = edit(&dir, PATH_1, RECORD, "agent", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-with-warnings")),
        "{v}"
    );
    assert!(msg(&v).contains("Already failing on disk"), "{v}");
    let right = RECORD.replace("Not started", "Implemented");
    let (code, v) = edit(&dir, PATH_1, &right, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (0, Some("allow")), "{v}");
    assert!(msg(&v).contains(" rules ran on this record"), "{v}");
}

#[test]
fn an_ungoverned_path_is_not_governed_and_a_new_record_is_judged_from_the_proposal() {
    let dir = fixture("ungoverned", &config(""));
    let (code, v) = edit(&dir, "src/a.rs", "fn a() {}\n", "agent", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("not-governed")),
        "{v}"
    );
    let new = RECORD.replace("ADR-1: T", "ADR-3: V");
    let (code, v) = edit(&dir, "docs/adr/ADR-3-c.md", &new, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
}

#[test]
fn when_urzua_cannot_judge_the_edit_proceeds_and_says_so_and_never_silently() {
    let silent = "schema_version: 2\nrules: {}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n".to_string()
        + HARNESS;
    let dir = fixture("unjudged", &silent);
    let (code, v) = edit(&dir, PATH_1, RECORD, "agent", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
    assert!(msg(&v).contains("did NOT judge"), "{v}");
    // An unreadable request is a stated verdict, not a silent exit.
    let (code, v) = run_core(&dir, "edit", "not json at all", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
    assert!(
        msg(&v).contains("did NOT judge") && msg(&v).contains("not valid JSON"),
        "{v}"
    );
    // So is a request with no content or two paths.
    for request in [
        serde_json::json!({"event": "edit", "paths": [PATH_1], "cwd": dir.to_string_lossy()}),
        serde_json::json!({"event": "edit", "paths": [PATH_1, "x"], "content": "y", "cwd": dir.to_string_lossy()}),
    ] {
        let (code, v) = edit_request(&dir, request, &[]);
        assert_eq!(
            (code, v["verdict"].as_str()),
            (0, Some("allow-unjudged")),
            "{v}"
        );
    }
}

#[test]
fn a_shallow_clone_still_blocks_what_needs_no_history_and_says_what_it_skipped() {
    let src = fixture("shallow-src", &config(""));
    std::fs::write(src.join("src/a.rs"), "fn a() { }\n").unwrap();
    git(&src, &["commit", "-q", "-am", "second"]);
    let dst = common::TempDir::new("urzua-harness-shallow-clone");
    assert!(Command::new("git")
        .args(["clone", "-q", "--depth", "1"])
        .arg(format!("file://{}", src.display()))
        .arg(&*dst)
        .status()
        .unwrap()
        .success());
    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = edit(&dst, PATH_1, &wrong, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(
        msg(&v).contains("embodiment drift") && msg(&v).contains("shallow clone"),
        "{v}"
    );
    let ok = RECORD.replace("Not started", "Implemented");
    let (code, v) = edit(&dst, PATH_1, &ok, "agent", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-with-warnings")),
        "{v}"
    );
    assert!(
        msg(&v).contains("NOT judged") && msg(&v).contains("shallow clone"),
        "{v}"
    );
}

const WIDE: &str = "schema_version: 2\nrules: {header.required-fields: error, section.required: warn, embodiment.consistency: error}\nembodiment:\n  drift: Drift detected\n  tiers:\n    - {state: Verified, evidence: test, claims_built: true}\n    - {state: Implemented, evidence: code, claims_built: true}\n  none: Not started\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n    known_fields: [\"Embodiment\", \"Realized-by\"]\n    companion_fields:\n      - [embodiment_state, embodiment_locator]\n    sections:\n      from: headings\n      required:\n        - {name: Context}\n";
const GOOD: &str =
    "---\nStatus: Proposed\nDate: 2026-01-01\n---\n# ADR-1: T\n\n## Context\n\nwhy\n";

fn wide(name: &str, extra: &str, record: &str) -> common::TempDir {
    let dir = fixture(name, &format!("{WIDE}{HARNESS}{extra}"));
    std::fs::write(dir.join(PATH_1), record).unwrap();
    git(&dir, &["commit", "-q", "-am", "record"]);
    dir
}

#[test]
fn a_header_error_blocks_a_warning_is_shown_and_the_message_names_what_ran() {
    let dir = wide("wide", "", GOOD);
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = edit(&dir, PATH_1, &no_date, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(
        msg(&v).contains("[header.required-fields]") && msg(&v).contains("'Date'"),
        "{v}"
    );
    let no_context = GOOD.replace("## Context\n\nwhy\n", "prose\n");
    let (code, v) = edit(&dir, PATH_1, &no_context, "agent", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-with-warnings")),
        "{v}"
    );
    assert!(msg(&v).contains("[section.required]"), "{v}");
    let (code, v) = edit(&dir, PATH_1, GOOD, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (0, Some("allow")), "{v}");
    assert!(!msg(&v).contains("every enabled rule"), "{v}");
}

#[test]
fn an_error_already_on_disk_does_not_block_an_unrelated_edit_but_a_new_one_does() {
    let broken = GOOD.replace("Date: 2026-01-01\n", "");
    let dir = wide("preexisting", "", &broken);
    let edited = broken.replace("why", "because");
    let (code, v) = edit(&dir, PATH_1, &edited, "agent", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-with-warnings")),
        "{v}"
    );
    assert!(msg(&v).contains("Already failing on disk"), "{v}");
    let worse = edited.replace(
        "---\nStatus",
        "---\nEmbodiment: Verified\nRealized-by: code:src/a.rs\nStatus",
    );
    let (code, v) = edit(&dir, PATH_1, &worse, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("[embodiment.consistency]"), "{v}");
    // `block_on: all` blocks any unwaived error and lists nothing as pre-existing.
    let dir = wide("block-all", "block_on: all\n", &broken);
    let (code, v) = edit(&dir, PATH_1, &edited, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(!msg(&v).contains("Already failing"), "{v}");
}

#[test]
fn a_named_override_lets_the_edit_through_and_a_nameless_one_does_not() {
    let dir = fixture("override", &config(""));
    let wrong = RECORD.replace("Not started", "Verified");
    let (code, v) = edit(
        &dir,
        PATH_1,
        &wrong,
        "agent",
        &[("URZUA_HOOK_OVERRIDE", "alice-reviewed")],
    );
    assert_eq!(
        (code, v["verdict"].as_str(), v["override"].as_str()),
        (0, Some("allow-override"), Some("alice-reviewed")),
        "{v}"
    );
    assert!(
        msg(&v).contains("OVERRIDE 'alice-reviewed'") && msg(&v).contains("'Implemented'"),
        "{v}"
    );
    assert!(msg(&v).contains("NOT recorded"), "{v}");
    // The override named in the request, not the environment, works the same.
    let (code, v) = edit_request(
        &dir,
        serde_json::json!({"paths": [PATH_1], "content": wrong, "cwd": dir.to_string_lossy(), "override": {"name": "bob"}}),
        &[],
    );
    assert_eq!((code, v["override"].as_str()), (0, Some("bob")), "{v}");
    // A blank name is not accepted: the block stands.
    let (code, v) = edit_request(
        &dir,
        serde_json::json!({"paths": [PATH_1], "content": wrong, "cwd": dir.to_string_lossy(), "override": {"name": "  "}}),
        &[],
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("needs a name"), "{v}");
}

const LOGGED: &str = "override_log: .urzua/override-log.md\n";

fn logged(name: &str) -> common::TempDir {
    let cfg = "schema_version: 2\noverride_log: .urzua/override-log.md\nrules: {header.required-fields: error, override.unreviewed: warn}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\"]\n".to_string() + HARNESS;
    let dir = fixture(name, &cfg);
    std::fs::write(dir.join(PATH_1), GOOD).unwrap();
    git(&dir, &["commit", "-q", "-am", "record"]);
    dir
}

#[test]
fn an_override_is_recorded_in_the_declared_log_and_check_then_reports_it_unreviewed() {
    let _ = LOGGED;
    let dir = logged("logged");
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = edit(
        &dir,
        PATH_1,
        &no_date,
        "agent",
        &[
            ("URZUA_HOOK_OVERRIDE", "dana"),
            ("URZUA_HOOK_REASON", "importing a legacy record"),
        ],
    );
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-override")),
        "{v}"
    );
    assert!(
        msg(&v).contains("Recorded in .urzua/override-log.md"),
        "{v}"
    );
    let log = read(&dir, ".urzua/override-log.md");
    let row = log
        .lines()
        .find(|l| l.contains("dana"))
        .unwrap_or_else(|| panic!("{log}"));
    for cell in [
        PATH_1,
        "header.required-fields",
        "importing a legacy record",
    ] {
        assert!(row.contains(cell), "{cell} in {row}");
    }
    assert!(
        row.trim_end().ends_with("| |"),
        "Reviewed-by is blank: {row}"
    );
    let out = urzua(&dir, &["check"]).output().unwrap();
    let unreviewed = json(&out)["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "override.unreviewed")
        .count();
    assert_eq!(unreviewed, 1);
}

#[test]
fn with_a_log_declared_an_override_needs_a_reason_and_an_unwritable_log_refuses_it() {
    let dir = logged("logged-noreason");
    let no_date = GOOD.replace("Date: 2026-01-01\n", "");
    let (code, v) = edit(
        &dir,
        PATH_1,
        &no_date,
        "agent",
        &[("URZUA_HOOK_OVERRIDE", "dana")],
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("needs a reason"), "{v}");
    assert!(!dir.join(".urzua/override-log.md").exists());

    let dir = logged("logged-unwritable");
    std::fs::create_dir_all(dir.join(".urzua/override-log.md")).unwrap();
    std::fs::write(dir.join(".urzua/override-log.md/keep"), "x").unwrap();
    let (code, v) = edit(
        &dir,
        PATH_1,
        &no_date,
        "agent",
        &[
            ("URZUA_HOOK_OVERRIDE", "dana"),
            ("URZUA_HOOK_REASON", "why"),
        ],
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("could not be recorded"), "{v}");
}

#[test]
fn on_unjudged_deny_refuses_an_agent_allows_a_person_and_names_the_escape_hatch() {
    let deny = "schema_version: 2\nrules: {}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\nharness:\n  adapters: [claude-code]\n  guardrails:\n    editor-hook: {enabled: true, on_unjudged: deny}\n    session-start: {enabled: true}\n";
    let dir = fixture("deny", deny);
    let (code, v) = edit(&dir, PATH_1, RECORD, "agent", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("outside the agent harness"), "{v}");
    assert!(
        msg(&v).contains("not accepted"),
        "the ADR-78 override is not accepted for this verdict: {v}"
    );
    // A named override does not lift it.
    let (code, v) = edit(
        &dir,
        PATH_1,
        RECORD,
        "agent",
        &[("URZUA_HOOK_OVERRIDE", "me")],
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    let (code, v) = edit(&dir, PATH_1, RECORD, "human", &[]);
    assert_eq!(
        (code, v["verdict"].as_str()),
        (0, Some("allow-unjudged")),
        "{v}"
    );
    assert!(msg(&v).contains("did NOT judge"), "{v}");
    // doctor names the same escape hatch.
    let out = urzua(&dir, &["doctor"]).output().unwrap();
    let hatch: Vec<_> = json(&out)["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "harness-escape-hatch")
        .map(|c| c["message"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(hatch.len(), 1, "{hatch:?}");
    assert!(hatch[0].contains("outside the agent harness"), "{hatch:?}");
    // A configuration that does not load still honours what was declared.
    let broken = deny.replace("rules: {}", "rules: {no.such-rule: error}");
    std::fs::write(dir.join(".urzua/config.yaml"), broken).unwrap();
    let (_, v) = edit(&dir, PATH_1, RECORD, "agent", &[]);
    assert_eq!(v["verdict"], "block", "{v}");
    let (_, v) = edit(&dir, PATH_1, RECORD, "human", &[]);
    assert_eq!(v["verdict"], "allow-unjudged", "{v}");
}

#[test]
fn on_unjudged_deny_also_refuses_an_unreadable_request_and_an_unenterable_cwd() {
    // The request itself cannot be read, or names a directory that cannot be
    // entered: the repository's declaration is still read from the process's
    // own directory, so a denying guardrail does not turn into an allowing one.
    let deny = "schema_version: 2\nrules: {}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\nharness:\n  adapters: [claude-code]\n  guardrails:\n    editor-hook: {enabled: true, on_unjudged: deny}\n    session-start: {enabled: true}\n";
    let dir = fixture("deny-request", deny);
    let (code, v) = run_core(&dir, "edit", "not json at all", &[]);
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("not valid JSON"), "{v}");
    assert!(msg(&v).contains("outside the agent harness"), "{v}");
    let (code, v) = edit_request(
        &dir,
        serde_json::json!({"event": "edit", "paths": [PATH_1], "content": RECORD, "cwd": "/nonexistent-urzua-dir"}),
        &[],
    );
    assert_eq!((code, v["verdict"].as_str()), (1, Some("block")), "{v}");
    assert!(msg(&v).contains("cannot be entered"), "{v}");
}

// ------------------------------------------------------- the translator

fn run_with_stdin(c: &mut Command, stdin: &str) -> Output {
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    let write = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    // A hook that cannot judge (no engine, no interpreter) states so and exits
    // without reading its input, so the child may be gone before the write
    // lands: a broken pipe then means "not read", which the callers assert on
    // through the output. Any other write error is a real failure.
    if let Err(e) = write {
        assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe, "{e}");
    }
    child.wait_with_output().unwrap()
}

/// A command that exits without reading its input is not a failure of the
/// helper that fed it. The input is larger than a pipe's buffer, so the write
/// can only end in a broken pipe once the child is gone, whichever side wins
/// the race; the helper must return the child's output either way.
#[test]
fn feeding_a_command_that_never_reads_its_input_returns_its_output() {
    let mut c = Command::new("sh");
    c.args(["-c", "echo done"]);
    let out = run_with_stdin(&mut c, &"x".repeat(4 << 20));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "done");
}

/// The hook command the installed settings carry for `handler`, run the way
/// the agent runs it: through `sh -c`, in the repository.
fn hook_command(dir: &Path, handler: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(&read(dir, SETTINGS)).unwrap();
    let events = v["hooks"].as_object().unwrap();
    events
        .values()
        .flat_map(|groups| groups.as_array().unwrap())
        .flat_map(|g| g["hooks"].as_array().unwrap())
        .map(|h| h["command"].as_str().unwrap().to_string())
        .find(|c| c.contains(&format!("urzua harness run {handler}")))
        .unwrap_or_else(|| panic!("no {handler} hook command"))
}

fn payload(dir: &Path, content: &str) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse", "tool_name": "Write", "cwd": dir.to_string_lossy(),
        "tool_input": {"file_path": dir.join(PATH_1).to_string_lossy(), "content": content},
    })
    .to_string()
}

/// A directory holding only symlinks to the named programs, as a `PATH`.
fn path_with(names: &[&str]) -> common::TempDir {
    let dir = common::TempDir::new("urzua-harness-path");
    for name in names {
        let target = if *name == "urzua" {
            PathBuf::from(env!("CARGO_BIN_EXE_urzua"))
        } else {
            which(name)
        };
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, dir.join(name)).unwrap();
    }
    dir
}

fn which(name: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} must be installed"))
}

#[test]
fn the_installed_hook_command_judges_an_edit_and_does_not_depend_on_the_project_variable() {
    let dir = fixture("command", &config(""));
    install(&dir);
    let cmd = hook_command(&dir, "edit");
    let wrong = RECORD.replace("Not started", "Verified");
    // The variable is unset: the command still reaches the engine and the
    // agent gets the verdict, not a bare exit 2.
    let mut c = Command::new("sh");
    c.args(["-c", &cmd])
        .current_dir(&dir)
        .env_remove("CLAUDE_PROJECT_DIR")
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, &payload(&dir, &wrong));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v = json(&out);
    assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny", "{v}");
    assert!(v["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .contains("'Implemented'"));
    // A correct edit is silent; an ungoverned tool is not touched.
    let right = RECORD.replace("Not started", "Implemented");
    let mut c = Command::new("sh");
    c.args(["-c", &cmd])
        .current_dir(&dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, &payload(&dir, &right));
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty(), "a clean edit says nothing: {out:?}");
}

#[test]
fn without_the_engine_or_the_interpreter_the_command_states_that_the_edit_was_not_judged() {
    let dir = fixture("command-missing", &config(""));
    install(&dir);
    let cmd = hook_command(&dir, "edit");
    let wrong = RECORD.replace("Not started", "Verified");
    // No urzua binary at all.
    let only_python = path_with(&["python3", "git", "sh"]);
    let mut c = Command::new(which("sh"));
    c.args(["-c", &cmd])
        .current_dir(&dir)
        .env("PATH", &*only_python)
        .env_remove("URZUA_BIN");
    let out = run_with_stdin(&mut c, &payload(&dir, &wrong));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        json(&out)["systemMessage"]
            .as_str()
            .unwrap()
            .contains("did NOT judge"),
        "{out:?}"
    );
    // No python3: the shell fallback says so.
    let no_python = path_with(&["git", "sh"]);
    let mut c = Command::new(which("sh"));
    c.args(["-c", &cmd])
        .current_dir(&dir)
        .env("PATH", &*no_python);
    let out = run_with_stdin(&mut c, &payload(&dir, &wrong));
    assert_eq!(out.status.code(), Some(0), "not 127: {out:?}");
    assert!(
        json(&out)["systemMessage"]
            .as_str()
            .unwrap()
            .contains("could NOT run"),
        "{out:?}"
    );
    // An unreadable payload is stated, not swallowed.
    let mut c = Command::new("sh");
    c.args(["-c", &cmd])
        .current_dir(&dir)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(&mut c, "{ garbage");
    assert_eq!(out.status.code(), Some(0));
    assert!(
        json(&out)["systemMessage"]
            .as_str()
            .unwrap()
            .contains("NOT judged"),
        "{out:?}"
    );
}

// ---------------------------------------------------------- session start

fn wire_git_hooks(dir: &Path) {
    let hooks = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    for layer in ["pre-commit", "pre-push"] {
        let p = hooks.join(layer);
        std::fs::write(&p, "#!/bin/sh\nurzua check\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

fn session_start(dir: &Path, path: &Path, project_dir: bool) -> serde_json::Value {
    let mut c = urzua(dir, &["harness", "run", "session-start"]);
    c.env("PATH", path);
    c.env_remove("CLAUDE_PROJECT_DIR");
    if project_dir {
        c.env("CLAUDE_PROJECT_DIR", dir);
    }
    let out = run_with_stdin(
        &mut c,
        &serde_json::json!({"event": "session-start", "cwd": dir.to_string_lossy()}).to_string(),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    json(&out)
}

fn checks_of(v: &serde_json::Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["check"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn session_start_is_quiet_when_ready_and_names_each_thing_that_would_fail_open() {
    let dir = fixture("session", &config(""));
    install(&dir);
    wire_git_hooks(&dir);
    let ok = path_with(&["urzua", "python3", "git", "sh"]);
    let v = session_start(&dir, &ok, true);
    assert_eq!(v["verdict"], "allow", "{v}");

    // The project-directory variable unset.
    let v = session_start(&dir, &ok, false);
    assert_eq!(checks_of(&v), ["environment"], "{v}");
    assert!(msg(&v).contains("CLAUDE_PROJECT_DIR"), "{v}");

    // A missing interpreter.
    let no_python = path_with(&["urzua", "git", "sh"]);
    let v = session_start(&dir, &no_python, true);
    assert_eq!(checks_of(&v), ["interpreter"], "{v}");
    assert!(msg(&v).contains("127"), "{v}");

    // A missing binary.
    let no_urzua = path_with(&["python3", "git", "sh"]);
    let v = session_start(&dir, &no_urzua, true);
    assert_eq!(checks_of(&v), ["binary"], "{v}");

    // A bundle older than the binary, and one newer.
    let lock = read(&dir, LOCK);
    std::fs::write(
        dir.join(LOCK),
        lock.replace(env!("CARGO_PKG_VERSION"), "0.0.1"),
    )
    .unwrap();
    let v = session_start(&dir, &ok, true);
    assert_eq!(checks_of(&v), ["version"], "{v}");
    assert!(msg(&v).contains("older than this binary"), "{v}");
    std::fs::write(
        dir.join(LOCK),
        lock.replace(env!("CARGO_PKG_VERSION"), "999.0.0"),
    )
    .unwrap();
    let v = session_start(&dir, &ok, true);
    assert!(msg(&v).contains("newer than this binary"), "{v}");
    std::fs::write(dir.join(LOCK), lock).unwrap();

    // An unwired hook entry.
    let settings = read(&dir, SETTINGS);
    std::fs::write(dir.join(SETTINGS), "{}\n").unwrap();
    let v = session_start(&dir, &ok, true);
    assert_eq!(
        checks_of(&v),
        ["hook-entry", "hook-entry", "hook-entry"],
        "{v}"
    );
    std::fs::write(dir.join(SETTINGS), settings).unwrap();

    // An unwired git hook.
    std::fs::remove_file(dir.join(".git/hooks/pre-push")).unwrap();
    let v = session_start(&dir, &ok, true);
    assert_eq!(checks_of(&v), ["git-hook"], "{v}");
    assert_eq!(v["verdict"], "allow-with-warnings");
}

#[test]
fn the_session_start_command_relays_its_message_to_the_agent() {
    let dir = fixture("session-command", &config(""));
    install(&dir);
    let cmd = hook_command(&dir, "session-start");
    let mut c = Command::new("sh");
    c.args(["-c", &cmd])
        .current_dir(&dir)
        .env_remove("CLAUDE_PROJECT_DIR")
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"));
    let out = run_with_stdin(
        &mut c,
        &serde_json::json!({"hook_event_name": "SessionStart", "cwd": dir.to_string_lossy()})
            .to_string(),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let v = json(&out);
    let message = v["systemMessage"].as_str().unwrap();
    assert!(message.contains("CLAUDE_PROJECT_DIR"), "{v}");
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "SessionStart");
}

// ----------------------------------------------------- vendor neutrality

#[test]
fn no_engine_source_file_names_an_agent_vendor_and_the_bundle_is_the_one_exclusion() {
    const VENDORS: &[&str] = &[
        "claude",
        "anthropic",
        "codex",
        "copilot",
        "gemini",
        "openai",
        "windsurf",
    ];
    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                rust_files(&p, out);
            } else if p.extension().is_some_and(|e| e == "rs") {
                out.push(p);
            }
        }
    }
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in std::fs::read_dir(&crates).unwrap() {
        let src = krate.unwrap().path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(files.len() > 30, "the scan found the engine source");
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap().to_lowercase();
        for v in VENDORS {
            assert!(!text.contains(v), "{} names '{v}'", f.display());
        }
    }
    // The bundle's adapter directory is outside src/ and does name its vendor, by design.
    let bundle = Path::new(env!("CARGO_MANIFEST_DIR")).join("bundle");
    assert!(!files.iter().any(|f| f.starts_with(&bundle)));
    let adapter = std::fs::read_to_string(bundle.join("claude-code/adapter.json")).unwrap();
    assert!(adapter.contains("claude-code"));
}
