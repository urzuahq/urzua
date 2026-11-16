//! A hook the repository maintains itself is declared in configuration and
//! proven by running it (`urzua hooks verify --run`), never by reading its text
//! (`MILE-220`, third review of PR 547).

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Status]\n";
const BROKEN: &str = "---\nDate: 2026-01-01\n---\n# B\n";
const DECLARED: &str =
    "hooks:\n  pre-commit: {managed: declared}\n  pre-push: {managed: declared}\n";

fn fixture(name: &str, extra: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-hooks-verify-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    std::fs::write(dir.join(".urzua/config.yaml"), format!("{CONFIG}{extra}")).unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Draft\n---\n# A\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "--no-verify", "-m", "fixture"]);
    dir
}

fn urzua(dir: &Path, args: &[&str]) -> (i32, serde_json::Value) {
    let out = common::urzua()
        .args(args)
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .current_dir(dir)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or_else(|_| {
        panic!(
            "urzua {args:?} printed no JSON: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.code().unwrap(), json)
}

fn write_exec(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn hook_check(dir: &Path, layer: &str) -> serde_json::Value {
    let (_, doctor) = urzua(dir, &["doctor"]);
    doctor["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "hook-wired" && c["layer"] == layer)
        .cloned()
        .unwrap()
}

fn states(report: &serde_json::Value) -> Vec<String> {
    report["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            format!(
                "{}={}",
                i["hook"].as_str().unwrap(),
                i["state"].as_str().unwrap()
            )
        })
        .collect()
}

fn commit(dir: &Path, envs: &[(&str, &str)]) -> Output {
    let mut cmd = common::isolated_git();
    cmd.args(["commit", "-q", "-m", "x"])
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .current_dir(dir);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn reached(report: &serde_json::Value, hook: &str) -> serde_json::Value {
    report["hooks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["hook"] == hook)
        .map(|h| h["reached"].clone())
        .unwrap()
}

#[test]
fn a_declared_hook_is_not_written_and_is_ok_only_while_plausible() {
    let dir = fixture("declared", DECLARED);
    let hook = dir.join(".git/hooks/pre-commit");
    let mine = "#!/bin/sh\nset -e\nmake lint\n\"${URZUA_BIN:-urzua}\" hooks run pre-commit\n";
    write_exec(&hook, mine);
    write_exec(&dir.join(".git/hooks/pre-push"), "#!/bin/sh\nurzua check\n");
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(
        states(&report),
        ["pre-commit=declared", "pre-push=declared"]
    );
    assert_eq!(std::fs::read_to_string(&hook).unwrap(), mine);
    let c = hook_check(&dir, "pre-commit");
    assert_eq!(
        (c["status"].as_str(), c["state"].as_str()),
        (Some("ok"), Some("declared")),
        "{c}"
    );

    write_exec(&hook, "#!/bin/sh\nmake lint\n");
    let c = hook_check(&dir, "pre-commit");
    assert_eq!(c["state"], "declared-no-call", "{c}");
    assert_eq!(c["status"], "warn");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        hook_check(&dir, "pre-commit")["state"],
        "declared-not-executable"
    );
    std::fs::remove_file(&hook).unwrap();
    assert_eq!(hook_check(&dir, "pre-commit")["state"], "declared-missing");
}

#[test]
fn verify_explains_without_run_and_runs_only_with_it() {
    let dir = fixture("explain", "");
    let marker = dir.join("side-effect");
    write_exec(
        &dir.join(".git/hooks/pre-commit"),
        &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    );
    urzua(&dir, &["hooks", "install"]);
    let (code, report) = urzua(&dir, &["hooks", "verify"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["run"], false);
    assert!(!marker.exists(), "verify without --run ran a hook");
    assert_eq!(report["next_action"], "urzua hooks verify --run");

    let (code, report) = urzua(&dir, &["hooks", "verify", "--run"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(reached(&report, "pre-commit"), true);
    assert_eq!(reached(&report, "pre-push"), true);
    assert!(marker.exists(), "the rest of the hook runs for real");
    assert!(report.to_string().contains("ran for real"), "{report}");
}

#[test]
fn verify_reports_a_hook_that_never_reaches_urzua() {
    let dir = fixture("unreached", DECLARED);
    write_exec(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nexit 0; urzua check\n",
    );
    write_exec(
        &dir.join(".git/hooks/pre-push"),
        "#!/bin/sh\n\"$URZUA_BIN\" check\n",
    );
    assert_eq!(hook_check(&dir, "pre-commit")["status"], "ok");
    let (code, report) = urzua(&dir, &["hooks", "verify", "--run", "pre-commit"]);
    assert_eq!(code, 1, "{report}");
    assert_eq!(reached(&report, "pre-commit"), false);
    let (code, report) = urzua(&dir, &["hooks", "verify", "--run", "pre-push"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(reached(&report, "pre-push"), true);
}

#[test]
fn the_probe_set_by_hand_does_not_skip_the_check() {
    let dir = fixture("probe", "");
    urzua(&dir, &["hooks", "install"]);
    let fake = common::TempDir::new("urzua-hooks-fake-probe");
    std::fs::write(fake.join("token"), "abc").unwrap();
    std::fs::write(dir.join("docs/adr/ADR-2-b.md"), BROKEN).unwrap();
    common::git(&dir, &["add", "-A"]);
    for value in [
        format!("1:abc:{}", fake.display()),
        format!("{}:abc:{}", std::process::id(), fake.display()),
        "garbage".to_string(),
    ] {
        let out = commit(&dir, &[("URZUA_HOOKS_PROBE", &value)]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{value}: {stderr}");
        assert!(stderr.contains("header.required-fields"), "{stderr}");
    }
}

#[test]
fn fail_on_in_the_hook_arguments_prints_the_findings_it_blocks_on() {
    let dir = fixture(
        "fail-on",
        "hooks:\n  pre-commit: [check, --base, HEAD, --fail-on, warning]\n",
    );
    let cfg = std::fs::read_to_string(dir.join(".urzua/config.yaml"))
        .unwrap()
        .replace(
            "header.required-fields: error",
            "header.required-fields: warn",
        );
    std::fs::write(dir.join(".urzua/config.yaml"), cfg).unwrap();
    urzua(&dir, &["hooks", "install"]);
    std::fs::write(dir.join("docs/adr/ADR-2-b.md"), BROKEN).unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = commit(&dir, &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("1 blocking finding(s)"), "{stderr}");
    assert!(
        stderr.contains("header.required-fields docs/adr/ADR-2-b.md"),
        "{stderr}"
    );
}

#[test]
fn an_env_dash_s_shebang_is_a_shell_hook() {
    let dir = fixture("env-s", "");
    write_exec(
        &dir.join(".git/hooks/pre-commit"),
        "#!/usr/bin/env -S bash -eu\necho orig\n",
    );
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(states(&report)[0], "pre-commit=extended");
}

const HUSKY_SH: &str = "if [ -z \"$husky_skip_init\" ]; then\n  readonly husky_skip_init=1\n  export husky_skip_init\n  sh -e \"$0\" \"$@\"\n  exitCode=\"$?\"\n  exit $exitCode\nfi\n";

#[test]
fn husky_v8_with_the_source_line_later_still_runs_the_block_once() {
    let dir = fixture("husky-later", "");
    write_exec(&dir.join(".husky/_/husky.sh"), HUSKY_SH);
    let hook = dir.join(".husky/pre-commit");
    write_exec(
        &hook,
        "#!/usr/bin/env sh\nset -e\n. \"$(dirname -- \"$0\")/_/husky.sh\"\necho orig\n",
    );
    common::git(&dir, &["config", "core.hooksPath", ".husky"]);
    urzua(&dir, &["hooks", "install"]);
    let text = std::fs::read_to_string(&hook).unwrap();
    assert!(
        text.find("husky.sh").unwrap() < text.find("urzua-hooks begin").unwrap(),
        "{text}"
    );
    std::fs::write(
        dir.join("docs/adr/ADR-3-c.md"),
        "---\nStatus: Draft\n---\n# C\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = commit(&dir, &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert_eq!(
        stderr.matches("no blocking findings").count(),
        1,
        "{stderr}"
    );
}

#[test]
fn a_hard_linked_hook_is_refused() {
    let dir = fixture("hardlink", "");
    let other = dir.join("shared-hook");
    write_exec(&other, "#!/bin/sh\necho shared\n");
    std::fs::create_dir_all(dir.join(".git/hooks")).unwrap();
    std::fs::hard_link(&other, dir.join(".git/hooks/pre-commit")).unwrap();
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["problems"].to_string().contains("hard link"),
        "{report}"
    );
    assert_eq!(
        std::fs::read_to_string(&other).unwrap(),
        "#!/bin/sh\necho shared\n"
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

/// This repository's own hooks, declared in a temporary copy: `ok (declared)`
/// and reached when run. The copy gets the binary at the path the hooks call
/// and stand-ins for `cargo` and `make rust-build`, so the run is fast.
#[test]
fn this_repositorys_hooks_declared_are_ok_and_reached() {
    let dir = fixture("dogfood", DECLARED);
    for layer in ["pre-commit", "pre-push"] {
        let body = std::fs::read_to_string(repo_root().join(".git-hooks").join(layer)).unwrap();
        write_exec(&dir.join(".git-hooks").join(layer), &body);
    }
    common::git(&dir, &["config", "core.hooksPath", ".git-hooks"]);
    std::fs::create_dir_all(dir.join("rust/target/release")).unwrap();
    std::os::unix::fs::symlink(
        env!("CARGO_BIN_EXE_urzua"),
        dir.join("rust/target/release/urzua"),
    )
    .unwrap();
    std::fs::write(dir.join("Makefile"), "rust-build:\n\t@true\n").unwrap();
    let stubs = common::TempDir::new("urzua-hooks-dogfood-stubs");
    write_exec(&stubs.join("cargo"), "#!/bin/sh\nexit 0\n");
    for layer in ["pre-commit", "pre-push"] {
        let c = hook_check(&dir, layer);
        assert_eq!(
            (c["status"].as_str(), c["state"].as_str()),
            (Some("ok"), Some("declared")),
            "{c}"
        );
    }
    let mut path = vec![stubs.to_path_buf()];
    path.extend(std::env::split_paths(&common::path_with_stub_gh()));
    let out = common::urzua()
        .args(["hooks", "verify", "--run"])
        .env("PATH", std::env::join_paths(path).unwrap())
        .current_dir(&dir)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{report}");
    assert_eq!(reached(&report, "pre-commit"), true, "{report}");
    assert_eq!(reached(&report, "pre-push"), true, "{report}");
}
