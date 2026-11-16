//! `urzua hooks` never reports a block that cannot run as wired, never writes
//! through a symlink or outside the repository, and recognises every line it
//! prints as the fix (`MILE-220`, review of PR 547).

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Status]\n";
const BROKEN: &str = "---\nDate: 2026-01-01\n---\n# B\n";

fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-hooks-reach-{name}"));
    for d in ["docs/adr", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    common::git(&dir, &["init", "-q"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
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

fn pre_commit_check(dir: &Path) -> serde_json::Value {
    let (_, doctor) = urzua(dir, &["doctor"]);
    doctor["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "hook-wired" && c["layer"] == "pre-commit")
        .cloned()
        .unwrap()
}

fn write_exec(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn commit_with(dir: &Path, bin: &str) -> Output {
    common::isolated_git()
        .args(["commit", "-q", "-m", "x"])
        .env("URZUA_BIN", bin)
        .env("PATH", common::path_with_stub_gh())
        .current_dir(dir)
        .output()
        .unwrap()
}

fn commit_broken(dir: &Path) -> Output {
    std::fs::write(dir.join("docs/adr/ADR-2-b.md"), BROKEN).unwrap();
    common::git(dir, &["add", "-A"]);
    commit_with(dir, env!("CARGO_BIN_EXE_urzua"))
}

#[test]
fn a_trailing_exit_gets_the_block_before_it_and_a_broken_commit_is_blocked() {
    for tail in ["exit 0", "exit $?"] {
        let dir = fixture("exit");
        let hook = dir.join(".git/hooks/pre-commit");
        write_exec(&hook, &format!("#!/bin/sh\necho orig\n{tail}\n"));
        let (code, report) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(code, 0, "{report}");
        let text = std::fs::read_to_string(&hook).unwrap();
        assert!(
            text.find("urzua-hooks begin").unwrap() < text.find(tail).unwrap(),
            "{text}"
        );
        assert!(text.ends_with(&format!("{tail}\n")), "{text}");
        assert_eq!(pre_commit_check(&dir)["status"], "ok");
        let out = commit_broken(&dir);
        assert!(
            !out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn a_hook_that_can_leave_early_is_refused_with_the_line_and_where() {
    for (body, at) in [
        ("if true; then exec true; fi\n", "line 2"),
        ("main() { exec true; }\nmain\n", "line 2"),
        ("main(){ exec true; }; main\n", "line 2"),
        ("trap 'exit 0' EXIT\necho hi\n", "line 2"),
        ("echo a\nexit 0\necho b\n", "line 3"),
        ("if true; then\n  echo a\nfi\n", "line 4"),
    ] {
        let dir = fixture("early");
        let hook = dir.join(".git/hooks/pre-commit");
        let original = format!("#!/bin/sh\n{body}");
        write_exec(&hook, &original);
        let (code, report) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(code, 2, "{body}: {report}");
        let problems = report["problems"].to_string();
        assert!(
            problems.contains("hooks run pre-commit || exit $?"),
            "{problems}"
        );
        assert!(problems.contains(at), "{body}: {problems}");
        assert_eq!(std::fs::read_to_string(&hook).unwrap(), original);
        assert_ne!(pre_commit_check(&dir)["status"], "ok", "{body}");
    }
}

#[test]
fn a_block_after_an_unconditional_exit_is_reported_unreachable() {
    for before in ["exit 0\n", "exec true\n"] {
        let dir = fixture("unreachable");
        write_exec(&dir.join(".git/hooks/pre-commit"), "#!/bin/sh\n");
        let (code, _) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(code, 0);
        let hook = dir.join(".git/hooks/pre-commit");
        let text = std::fs::read_to_string(&hook).unwrap();
        std::fs::write(
            &hook,
            text.replacen("#!/bin/sh\n", &format!("#!/bin/sh\n{before}"), 1),
        )
        .unwrap();
        let check = pre_commit_check(&dir);
        assert_eq!(check["status"], "warn", "{check}");
        assert_eq!(check["state"], "installed-unreachable", "{check}");
        let (code, status) = urzua(&dir, &["hooks", "status"]);
        assert_eq!(code, 1);
        assert_eq!(states(&status)[0], "pre-commit=unreachable");
    }
}

#[test]
fn a_symlinked_hook_is_never_written_through() {
    let dir = fixture("symlink");
    let shared = common::TempDir::new("urzua-hooks-shared");
    let outside = shared.join("pre-commit");
    write_exec(&outside, "#!/bin/sh\necho shared\n");
    std::fs::create_dir_all(dir.join(".git/hooks")).unwrap();
    std::os::unix::fs::symlink(&outside, dir.join(".git/hooks/pre-commit")).unwrap();
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["problems"].to_string().contains("symlink"),
        "{report}"
    );
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "#!/bin/sh\necho shared\n"
    );

    let inside = fixture("symlink-inside");
    write_exec(&inside.join("tools/pre-commit"), "#!/bin/sh\necho mine\n");
    std::fs::create_dir_all(inside.join(".git/hooks")).unwrap();
    std::os::unix::fs::symlink(
        inside.join("tools/pre-commit"),
        inside.join(".git/hooks/pre-commit"),
    )
    .unwrap();
    let (code, report) = urzua(&inside, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert_eq!(
        std::fs::read_to_string(inside.join("tools/pre-commit")).unwrap(),
        "#!/bin/sh\necho mine\n"
    );
}

#[test]
fn a_hooks_path_that_resolves_outside_through_a_symlink_is_refused() {
    let dir = fixture("symlink-dir");
    let target = common::TempDir::new("urzua-hooks-symlink-target");
    std::os::unix::fs::symlink(target.path(), dir.join("myhooks")).unwrap();
    common::git(&dir, &["config", "core.hooksPath", "myhooks"]);
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["problems"]
            .to_string()
            .contains("outside the repository"),
        "{report}"
    );
    assert!(!target.join("pre-commit").exists());
}

/// An urzua release without `hooks run` exits 2 with a usage error.
fn old_urzua(dir: &Path) -> String {
    let path = dir.join("old-urzua");
    write_exec(
        &path,
        "#!/bin/sh\necho \"error: unrecognized subcommand 'hooks'\" >&2\necho '{\"status\":\"not-run\"}'\nexit 2\n",
    );
    path.to_string_lossy().into_owned()
}

#[test]
fn a_binary_without_hooks_run_is_not_run_and_obeys_the_policy() {
    let dir = fixture("old-binary");
    let (code, _) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0);
    let old = old_urzua(&common::TempDir::new("urzua-hooks-old"));
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Draft\n---\n# B\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = commit_with(&dir, &old);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("upgrade urzua"), "{stderr}");
    assert!(stderr.contains(env!("CARGO_PKG_VERSION")), "{stderr}");
    assert!(!stderr.contains("unrecognized subcommand"), "{stderr}");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("not-run"));

    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  on_not_run: block\n"),
    )
    .unwrap();
    urzua(&dir, &["hooks", "install"]);
    common::git(&dir, &["add", "-A"]);
    let out = commit_with(&dir, &old);
    assert!(!out.status.success());
}

#[test]
fn every_line_urzua_prints_as_the_fix_is_recognised_as_wired() {
    let dir = fixture("roundtrip-outside");
    let outside = common::TempDir::new("urzua-hooks-roundtrip-target");
    common::git(
        &dir,
        &["config", "core.hooksPath", outside.to_str().unwrap()],
    );
    let (_, report) = urzua(&dir, &["hooks", "install"]);
    let problems = report["problems"].as_array().unwrap();
    for (hook, problem) in ["pre-commit", "pre-push"].iter().zip(problems) {
        let line = problem
            .as_str()
            .unwrap()
            .rsplit(": ")
            .next()
            .unwrap()
            .to_string();
        write_exec(&outside.join(hook), &format!("#!/bin/sh\n{line}\n"));
    }
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 0, "{status}");

    for shebang in ["#!/usr/bin/env node", "#!/usr/bin/env python3"] {
        let dir = fixture("roundtrip-foreign");
        let hook = dir.join(".git/hooks/pre-commit");
        write_exec(&hook, &format!("{shebang}\n"));
        let (code, report) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(code, 2);
        let problem = report["problems"][0].as_str().unwrap();
        let line = problem.split("add this line to it: ").nth(1).unwrap();
        write_exec(&hook, &format!("{shebang}\n{line}\n"));
        assert_eq!(pre_commit_check(&dir)["status"], "ok", "{line}");
    }
}

#[test]
fn hooks_may_run_only_a_read_only_check() {
    let dir = fixture("restrict");
    for bad in [
        "[hooks, run, pre-commit]",
        "[refs, refresh]",
        "[fix, --apply]",
        "[harness, install]",
    ] {
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            format!("{CONFIG}hooks:\n  pre-commit: {bad}\n"),
        )
        .unwrap();
        let (code, report) = urzua(&dir, &["hooks", "status"]);
        assert_eq!(code, 2, "{bad}: {report}");
        assert!(report.to_string().contains("'pre-commit'"), "{report}");
    }
    for good in ["[check]", "[check, --base, HEAD]", "[audit]"] {
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            format!("{CONFIG}hooks:\n  pre-commit: {good}\n"),
        )
        .unwrap();
        let (code, report) = urzua(&dir, &["hooks", "status"]);
        assert_ne!(code, 2, "{good}: {report}");
    }
}

#[test]
fn uninstall_removes_every_duplicated_block_and_status_flags_them() {
    let dir = fixture("dup");
    urzua(&dir, &["hooks", "install"]);
    let hook = dir.join(".git/hooks/pre-commit");
    let text = std::fs::read_to_string(&hook).unwrap();
    let block = &text["#!/bin/sh\n".len()..];
    std::fs::write(&hook, format!("{text}{block}")).unwrap();
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 1, "{status}");
    assert_eq!(states(&status)[0], "pre-commit=duplicated");
    urzua(&dir, &["hooks", "uninstall"]);
    assert!(!hook.exists());
}

#[test]
fn the_chmod_fix_quotes_the_path() {
    let dir = fixture("chmod");
    let hook = dir.join(".git/hooks/pre-commit");
    write_exec(&hook, "#!/bin/sh\necho a\n");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o644)).unwrap();
    let check = pre_commit_check(&dir);
    let fix = check["fix"].as_str().unwrap();
    assert!(fix.starts_with("chmod +x '"), "{fix}");
}

#[test]
fn the_framework_stanza_prints_urzuas_not_run_message_when_urzua_is_missing() {
    let dir = fixture("framework-missing");
    std::fs::write(dir.join(".pre-commit-config.yaml"), "repos:\n").unwrap();
    let (code, _) = urzua(&dir, &["hooks", "install", "--write-config"]);
    assert_eq!(code, 0);
    let text = std::fs::read_to_string(dir.join(".pre-commit-config.yaml")).unwrap();
    let doc: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
    let entry = doc["repos"][0]["hooks"][0]["entry"].as_str().unwrap();
    let script = entry
        .strip_prefix("sh -c '")
        .and_then(|e| e.strip_suffix('\''))
        .unwrap();
    let out = std::process::Command::new("sh")
        .args(["-c", script])
        .env_remove("URZUA_BIN")
        .env("PATH", "/usr/bin:/bin")
        .current_dir(&dir)
        .output()
        .unwrap();
    if Path::new("/usr/bin/urzua").exists() {
        return;
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("urzua did not run"), "{stderr}");
}

#[test]
fn a_linked_worktree_and_a_submodule_write_into_their_git_directory() {
    let main = fixture("worktree-main");
    let linked = common::TempDir::new("urzua-hooks-linked");
    std::fs::remove_dir_all(linked.path()).unwrap();
    common::git(&main, &["worktree", "add", "-q", linked.to_str().unwrap()]);
    let (code, report) = urzua(&linked, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert!(main.join(".git/hooks/pre-commit").is_file(), "{report}");

    let sub_origin = fixture("submodule-origin");
    let outer = fixture("submodule-outer");
    common::git(
        &outer,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            sub_origin.to_str().unwrap(),
            "sub",
        ],
    );
    let (code, report) = urzua(&outer.join("sub"), &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert!(
        outer.join(".git/modules/sub/hooks/pre-commit").is_file(),
        "{report}"
    );
}
