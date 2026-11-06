//! `urzua harness install` reports what it did, honestly, and says when a
//! hand-copied first-generation editor hook is still wired; `doctor` reads
//! wiring, not words.

mod common;

use std::path::Path;
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: warn}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\nharness:\n  adapters: [claude-code]\n  instructions_file: AGENTS.md\n  guardrails:\n    editor-hook: {enabled: true}\n    session-start: {enabled: true}\n";

fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-harness-report-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Draft\n---\n# A\n",
    )
    .unwrap();
    std::fs::write(dir.join("AGENTS.md"), "# Agents\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out: Output = Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    let v = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}"));
    (out.status.code().unwrap(), v)
}

fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

/// N3: a refused install wrote nothing, so no item may say it wrote something,
/// and the advice must be something the person can do.
#[test]
fn a_refused_install_reports_would_states_and_advice_that_can_be_followed() {
    let dir = fixture("conflict");
    write(&dir, ".claude/commands/adr.md", "my own command\n");

    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["status"], "not-run", "{v}");
    assert!(!dir.join(".urzua/harness.lock").exists());
    assert!(!dir.join(".claude/settings.json").exists());

    for item in v["items"].as_array().unwrap() {
        let state = item["state"].as_str().unwrap();
        assert!(
            !matches!(state, "created" | "merged" | "written"),
            "nothing was written, yet an item says {state}: {v}"
        );
    }
    let problems = v["problems"].to_string();
    assert!(problems.contains(".claude/commands/adr.md"), "{problems}");
    assert!(
        !problems.contains("--force"),
        "update --force cannot be reached before an install: {problems}"
    );
    assert!(problems.contains("move or delete"), "{problems}");
}

/// N4: install and doctor name a first-generation hook wired beside the bundle's.
#[test]
fn a_hand_copied_first_generation_editor_hook_is_warned_about_by_install_and_doctor() {
    let dir = fixture("legacy");
    write(
        &dir,
        ".claude/settings.json",
        "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Write|Edit\",\n        \"hooks\": [\n          {\"type\": \"command\", \"command\": \"scripts/hooks/record-edit-hook.sh\"},\n          {\"type\": \"command\", \"command\": \"python3 scripts/hooks/agent-record-edit.py\"}\n        ]\n      }\n    ]\n  }\n}\n",
    );

    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    let notices = v["notices"].to_string();
    assert!(notices.contains("record-edit-hook.sh"), "{notices}");
    assert!(notices.contains("agent-record-edit.py"), "{notices}");
    assert!(notices.contains("first-generation"), "{notices}");

    let (_, d) = urzua(&dir, &["doctor"]);
    let legacy: Vec<_> = d["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "harness-legacy-hook")
        .collect();
    assert_eq!(legacy.len(), 1, "{d}");
    assert_eq!(legacy[0]["status"], "warn", "{d}");
}

/// N4: a clean install is not warned.
#[test]
fn a_clean_install_has_no_legacy_hook_warning() {
    let dir = fixture("no-legacy");
    let (code, v) = urzua(&dir, &["harness", "install"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        !v["notices"].to_string().contains("first-generation"),
        "{v}"
    );
    let (_, d) = urzua(&dir, &["doctor"]);
    assert!(
        d["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["check"] != "harness-legacy-hook"),
        "{d}"
    );
}

fn hook_fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-doctor-wiring-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: warn}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Draft\n---\n# A\n",
    )
    .unwrap();
    dir
}

fn plant_hook(dir: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(dir, ".git/hooks/pre-commit", body);
    std::fs::set_permissions(
        dir.join(".git/hooks/pre-commit"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
}

fn check_of(d: &serde_json::Value, name: &str) -> serde_json::Value {
    d["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == name && (name != "hook-wired" || c["layer"] == "pre-commit"))
        .unwrap_or_else(|| panic!("no {name}: {d}"))
        .clone()
}

/// N5: the word `urzua` in a comment, an echo or a name is not a call, and
/// `make records` is one repository's target, not a vocabulary the engine knows.
#[test]
fn doctor_reads_a_real_call_in_a_hook_not_a_word_in_a_comment_or_a_make_target() {
    let dir = hook_fixture("hook");
    plant_hook(
        &dir,
        "#!/bin/sh\n# runs urzua check before every commit\necho \"urzua check passed\"\n",
    );
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(
        check_of(&d, "hook-wired")["state"],
        "installed-not-running-urzua",
        "{d}"
    );

    plant_hook(&dir, "#!/bin/sh\nmake records\n");
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(
        check_of(&d, "hook-wired")["state"],
        "installed-not-running-urzua",
        "{d}"
    );

    plant_hook(&dir, "#!/bin/sh\n# gate\nexec urzua check --base HEAD\n");
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(
        check_of(&d, "hook-wired")["state"],
        "installed-running-urzua",
        "{d}"
    );

    plant_hook(
        &dir,
        "#!/bin/sh\nbin=\"$(git rev-parse --show-toplevel)/target/release/urzua\"\n\"$bin\" check\n",
    );
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(
        check_of(&d, "hook-wired")["state"],
        "installed-running-urzua",
        "{d}"
    );
}

/// N5: the same for a workflow.
#[test]
fn doctor_reads_a_real_call_in_a_workflow_not_a_word_in_a_comment() {
    let dir = hook_fixture("workflow");
    write(
        &dir,
        ".github/workflows/ci.yml",
        "name: ci\non: push\njobs:\n  a:\n    runs-on: x\n    steps:\n      # TODO: run urzua check here\n      - run: make records\n",
    );
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(check_of(&d, "ci-wired")["status"], "warn", "{d}");

    write(
        &dir,
        ".github/workflows/ci.yml",
        "name: ci\non: push\njobs:\n  a:\n    runs-on: x\n    steps:\n      - run: urzua check\n",
    );
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(check_of(&d, "ci-wired")["status"], "ok", "{d}");

    write(
        &dir,
        ".github/workflows/ci.yml",
        "name: ci\non: push\njobs:\n  a:\n    runs-on: x\n    steps:\n      - uses: urzuahq/urzua/.github/actions/urzua-gate@v1\n",
    );
    let (_, d) = urzua(&dir, &["doctor"]);
    assert_eq!(check_of(&d, "ci-wired")["status"], "ok", "{d}");
}
