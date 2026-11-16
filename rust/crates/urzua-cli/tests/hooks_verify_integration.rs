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
    assert!(report["next_action"].is_null(), "{report}");
    assert!(
        report
            .to_string()
            .contains("executes the repository's hooks"),
        "{report}"
    );

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

/// A directory shaped like a verify run's probe directory, made by hand.
fn fake_probe_dir(label: &str) -> common::TempDir {
    let d = common::TempDir::new(label);
    std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(d.join("token"), "0123456789abcdef0123456789abcdef").unwrap();
    d
}

fn isolated_env(cmd: &mut std::process::Command) {
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh());
}

#[test]
fn the_probe_never_changes_what_urzua_does() {
    let dir = fixture("probe", "");
    urzua(&dir, &["hooks", "install"]);
    let fake = fake_probe_dir("urzua-hooks-fake-probe");
    std::fs::write(dir.join("docs/adr/ADR-2-b.md"), BROKEN).unwrap();
    common::git(&dir, &["add", "-A"]);
    let value = fake.to_string_lossy().into_owned();
    let out = commit(&dir, &[("URZUA_HOOKS_PROBE", &value)]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("header.required-fields"), "{stderr}");

    // The review's spoof: an ancestor whose argv reads `urzua hooks verify --run`.
    let mut spoof = std::process::Command::new("bash");
    spoof
        .args([
            "-c",
            &format!(
                "exec -a urzua bash -c 'URZUA_HOOKS_PROBE=$$:0123456789abcdef0123456789abcdef:{value} git commit -q -m spoof; true' x hooks verify --run"
            ),
        ])
        .current_dir(&dir);
    isolated_env(&mut spoof);
    let out = spoof.output().unwrap();
    let log = common::isolated_git()
        .args(["log", "--oneline"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&log.stdout).contains("spoof"),
        "the broken commit landed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("header.required-fields"));

    let check = common::urzua()
        .args(["check", "--fail-on", "warning"])
        .env("URZUA_HOOKS_PROBE", &value)
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&check.stdout).contains("header.required-fields"));
}

#[test]
fn a_probe_pointing_anywhere_else_writes_nothing() {
    let dir = fixture("probe-paths", "");
    let file = dir.join("a-file");
    std::fs::write(&file, "x").unwrap();
    let no_token = common::TempDir::new("urzua-hooks-probe-no-token");
    std::fs::set_permissions(no_token.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let open = fake_probe_dir("urzua-hooks-probe-open");
    std::fs::set_permissions(open.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    let real = fake_probe_dir("urzua-hooks-probe-target");
    let link = dir.join("probe-link");
    std::os::unix::fs::symlink(real.path(), &link).unwrap();
    let listing = |p: &Path| -> Vec<String> {
        std::fs::read_dir(p)
            .map(|r| {
                r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    let baseline = common::urzua()
        .arg("check")
        .current_dir(&dir)
        .output()
        .unwrap();
    for target in [
        file.clone(),
        PathBuf::from("/etc"),
        link,
        no_token.to_path_buf(),
        open.to_path_buf(),
    ] {
        let watched = if target.is_dir() {
            target.clone()
        } else {
            dir.to_path_buf()
        };
        let before = listing(&watched);
        let out = common::urzua()
            .arg("check")
            .env("URZUA_HOOKS_PROBE", &target)
            .current_dir(&dir)
            .output()
            .unwrap();
        let shown = target.display();
        assert_eq!(out.status.code(), baseline.status.code(), "{shown}");
        assert_eq!(out.stdout, baseline.stdout, "{shown}");
        assert_eq!(listing(&watched), before, "{shown}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "x");
    }
    assert_eq!(
        std::fs::read_dir(real.path()).unwrap().count(),
        1,
        "nothing is written through the symlink"
    );
}

#[test]
fn a_commit_made_inside_a_verify_run_is_checked() {
    let dir = fixture("inner-commit", "hooks:\n  pre-push: {managed: declared}\n");
    urzua(&dir, &["hooks", "install"]);
    write_exec(
        &dir.join(".git/hooks/pre-push"),
        "#!/bin/sh\nprintf -- '---\\nDate: 2026-01-01\\n---\\n# B\\n' > docs/adr/ADR-9-inner.md\ngit add -A\ngit commit -q -m inner\n\"$URZUA_BIN\" check >/dev/null\n",
    );
    let mut verify = common::urzua();
    verify
        .args(["hooks", "verify", "--run", "pre-push"])
        .current_dir(&dir);
    isolated_env(&mut verify);
    let out = verify.output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(reached(&report, "pre-push"), true, "{report}");
    let log = common::isolated_git()
        .args(["log", "--oneline"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&log.stdout).contains("inner"),
        "the inner commit landed unchecked"
    );
}

#[test]
fn a_hook_with_no_shebang_runs_as_git_runs_it() {
    let dir = fixture("no-shebang", "");
    std::fs::create_dir_all(dir.join("githooks")).unwrap();
    std::fs::write(dir.join("githooks/pre-commit"), "echo orig\n").unwrap();
    std::fs::set_permissions(
        dir.join("githooks/pre-commit"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    common::git(&dir, &["config", "core.hooksPath", "githooks"]);
    urzua(&dir, &["hooks", "install"]);
    let (code, report) = urzua(&dir, &["hooks", "verify", "--run", "pre-commit"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(reached(&report, "pre-commit"), true, "{report}");
}

#[test]
fn a_hook_that_runs_too_long_is_killed_and_reported() {
    let dir = fixture(
        "timeout",
        "hooks:\n  pre-commit: {managed: declared}\n  verify_timeout_seconds: 2\n",
    );
    write_exec(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nsleep 60\nurzua check\n",
    );
    let started = std::time::Instant::now();
    let (code, report) = urzua(&dir, &["hooks", "verify", "--run", "pre-commit"]);
    assert!(started.elapsed().as_secs() < 30, "the hook was not killed");
    assert_eq!(code, 1, "{report}");
    assert_eq!(report["hooks"][0]["timed_out"], true, "{report}");
}

#[test]
fn pre_push_gets_a_remote_that_cannot_be_mistaken_for_a_real_one() {
    let dir = fixture("push-args", "");
    let seen = dir.join("seen");
    write_exec(
        &dir.join(".git/hooks/pre-push"),
        &format!(
            "#!/bin/sh\necho \"$1 $2\" > '{0}'\ncat >> '{0}'\n",
            seen.display()
        ),
    );
    urzua(&dir, &["hooks", "install"]);
    let (_, report) = urzua(&dir, &["hooks", "verify", "--run", "pre-push"]);
    let text = std::fs::read_to_string(&seen).unwrap();
    assert!(
        text.starts_with("urzua-verify urzua-verify://not-a-remote\n"),
        "{text:?}"
    );
    assert_eq!(text.lines().count(), 1, "stdin was not empty: {text:?}");
    assert!(
        report.to_string().contains("urzua-verify://not-a-remote"),
        "{report}"
    );
}

#[test]
fn hooks_outside_the_repository_need_an_explicit_flag() {
    let dir = fixture(
        "verify-outside",
        "hooks:\n  pre-commit: {managed: declared}\n",
    );
    let outside = common::TempDir::new("urzua-hooks-verify-outside");
    write_exec(
        &outside.join("pre-commit"),
        "#!/bin/sh\n\"$URZUA_BIN\" check\n",
    );
    common::git(
        &dir,
        &["config", "core.hooksPath", outside.to_str().unwrap()],
    );
    let (code, report) = urzua(&dir, &["hooks", "verify", "--run", "pre-commit"]);
    assert_eq!(code, 2, "{report}");
    assert!(report.to_string().contains("--allow-outside"), "{report}");
    let (code, report) = urzua(
        &dir,
        &["hooks", "verify", "--run", "--allow-outside", "pre-commit"],
    );
    assert_eq!(code, 0, "{report}");
    assert!(
        report.to_string().contains("outside the repository"),
        "{report}"
    );
}

#[test]
fn a_declared_hook_that_still_holds_the_block_says_uninstall_removes_it() {
    let dir = fixture("declared-block", "");
    urzua(&dir, &["hooks", "install"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DECLARED}"),
    )
    .unwrap();
    let c = hook_check(&dir, "pre-commit");
    assert_eq!(c["state"], "declared", "{c}");
    assert!(
        c["message"]
            .as_str()
            .unwrap()
            .contains("urzua hooks uninstall"),
        "{c}"
    );
    let (code, report) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(code, 0, "{report}");
    assert!(!dir.join(".git/hooks/pre-commit").exists());
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
    assert!(
        report["problems"].to_string().contains("managed: declared"),
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
