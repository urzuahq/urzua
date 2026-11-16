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
    // `set -e`, as husky v9 runs hooks: the block must still reach its policy.
    write_exec(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nset -e\necho orig\n",
    );
    let (code, _) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0);
    let old_dir = common::TempDir::new("urzua-hooks-old");
    let old = old_urzua(&old_dir);
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
    assert!(
        stderr.contains("upgrade urzua to 0.4.6 or later"),
        "{stderr}"
    );
    assert!(stderr.contains("orig"), "{stderr}");
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

#[test]
fn the_line_printed_for_an_outside_hook_follows_the_not_run_policy() {
    let dir = fixture("printed-line");
    let outside = common::TempDir::new("urzua-hooks-printed-target");
    common::git(
        &dir,
        &["config", "core.hooksPath", outside.to_str().unwrap()],
    );
    let (_, report) = urzua(&dir, &["hooks", "install"]);
    let problem = report["problems"][0].as_str().unwrap().to_string();
    let line = problem.split("to run urzua from it: ").nth(1).unwrap();
    write_exec(&outside.join("pre-commit"), &format!("#!/bin/sh\n{line}\n"));
    record(&dir, "ADR-41-x", false);
    let out = common::isolated_git()
        .args(["commit", "-q", "-m", "x"])
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
    assert!(stderr.contains("NOT checked"), "{stderr}");
}

/// One original hook body, with `{R}` standing for a command whose status
/// decides whether the original hook passes (`true`) or fails (`false`).
const SHAPES: &[(&str, &str)] = &[
    ("exit-status", "echo orig\n{R}\nexit $?\n"),
    ("exit-literal", "echo orig\n{R} && exit 0\nexit 1\n"),
    ("exec-args", "echo orig\nexec {R} \"$@\"\n"),
    ("if-exec", "if true; then exec {R}; fi\n"),
    ("function-call", "main() { echo orig; {R}; }\nmain\n"),
    ("trap-exit", "trap 'echo bye' EXIT\n{R}\n"),
    ("case-exit", "case x in x) {R}; exit $? ;; esac\n"),
    ("or-exit", "{R} || exit 1\n"),
    ("continuation", "{R} || \\\n  exit 1\n"),
    ("heredoc", "cat <<EOF >/dev/null\nexit 0\nEOF\n{R}\n"),
    ("exec-redirect", "exec 2>&1\necho orig\n{R}\n"),
    ("return", "{R} || return 1\nreturn 0\n"),
    ("one-line", "{R}\n"),
    ("set-e", "set -e\necho orig\n{R}\n"),
    ("set-u", "set -u\n{R}\n"),
];

fn record(dir: &Path, name: &str, broken: bool) {
    let body = if broken {
        BROKEN
    } else {
        "---\nStatus: Draft\n---\n# C\n"
    };
    std::fs::write(dir.join(format!("docs/adr/{name}.md")), body).unwrap();
    common::git(dir, &["add", "-A"]);
}

fn commit(dir: &Path) -> Output {
    commit_with(dir, env!("CARGO_BIN_EXE_urzua"))
}

/// Install into `hook` (already written), and check the four promises: the
/// original's outcome is unchanged, a broken record is stopped with its
/// finding printed, a second install changes nothing, and uninstall restores
/// the bytes.
fn assert_round_trip(dir: &Path, hook: &Path, label: &str) {
    let original = std::fs::read(hook).unwrap();
    record(dir, "ADR-10-before", false);
    let before = commit(dir).status.success();
    let (code, report) = urzua(dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{label}: {report}");
    record(dir, "ADR-11-after", false);
    let out = commit(dir);
    assert_eq!(
        out.status.success(),
        before,
        "{label}: the original outcome changed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    record(dir, "ADR-12-broken", true);
    let out = commit(dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{label}: {stderr}");
    assert!(
        stderr.contains("header.required-fields docs/adr/ADR-12-broken.md"),
        "{label}: {stderr}"
    );
    std::fs::remove_file(dir.join("docs/adr/ADR-12-broken.md")).unwrap();
    common::git(dir, &["add", "-A"]);
    let (_, again) = urzua(dir, &["hooks", "install"]);
    assert!(
        states(&again).iter().all(|s| s.ends_with("=unchanged")),
        "{label}: {again}"
    );
    urzua(dir, &["hooks", "uninstall"]);
    assert_eq!(std::fs::read(hook).unwrap(), original, "{label}");
}

#[test]
fn every_shape_runs_urzua_first_and_keeps_the_hooks_own_outcome() {
    for (name, body) in SHAPES {
        for r in ["true", "false"] {
            let dir = fixture(name);
            let hook = dir.join(".git/hooks/pre-commit");
            write_exec(&hook, &format!("#!/bin/sh\n{}", body.replace("{R}", r)));
            assert_round_trip(&dir, &hook, &format!("{name}/{r}"));
        }
    }
}

#[test]
fn shebang_only_empty_crlf_and_no_trailing_newline_are_restored_exactly() {
    for (label, content) in [
        ("shebang-only", "#!/bin/sh\n"),
        ("shebang-no-newline", "#!/bin/sh"),
        ("empty", ""),
        ("crlf", "#!/bin/sh\r\necho orig\r\n"),
        ("no-trailing-newline", "#!/bin/sh\necho orig"),
    ] {
        let dir = fixture(label);
        let hook = dir.join(".git/hooks/pre-commit");
        write_exec(&hook, content);
        let (code, report) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(code, 0, "{label}: {report}");
        assert_eq!(pre_commit_check(&dir)["status"], "ok", "{label}");
        let (_, again) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(states(&again)[0], "pre-commit=unchanged", "{label}");
        urzua(&dir, &["hooks", "uninstall"]);
        assert_eq!(std::fs::read_to_string(&hook).unwrap(), content, "{label}");
    }
    let dir = fixture("shebang-commit");
    let hook = dir.join(".git/hooks/pre-commit");
    write_exec(&hook, "#!/bin/sh\n");
    assert_round_trip(&dir, &hook, "shebang-only");
}

/// husky v8: `.husky/_/husky.sh` re-runs the hook under `sh -e`, so a block
/// above the source line would run twice.
const HUSKY_SH: &str = "if [ -z \"$husky_skip_init\" ]; then\n  readonly husky_skip_init=1\n  export husky_skip_init\n  sh -e \"$0\" \"$@\"\n  exitCode=\"$?\"\n  exit $exitCode\nfi\n";

#[test]
fn husky_v8_runs_the_block_once_after_its_source_line() {
    for r in ["true", "false"] {
        let dir = fixture("husky-v8");
        write_exec(&dir.join(".husky/_/husky.sh"), HUSKY_SH);
        let hook = dir.join(".husky/pre-commit");
        write_exec(
            &hook,
            &format!("#!/usr/bin/env sh\n. \"$(dirname -- \"$0\")/_/husky.sh\"\n\n{r}\n"),
        );
        common::git(&dir, &["config", "core.hooksPath", ".husky"]);
        urzua(&dir, &["hooks", "install"]);
        let text = std::fs::read_to_string(&hook).unwrap();
        assert!(
            text.find("husky.sh").unwrap() < text.find("urzua-hooks begin").unwrap(),
            "{text}"
        );
        urzua(&dir, &["hooks", "uninstall"]);
        assert_round_trip(&dir, &hook, &format!("husky-v8/{r}"));
        record(&dir, "ADR-20-once", false);
        let out = commit(&dir);
        let runs = String::from_utf8_lossy(&out.stderr)
            .matches("no blocking findings")
            .count();
        assert!(runs <= 1, "the block ran {runs} times");
    }
}

#[test]
fn husky_v9_under_sh_e_prints_findings_and_keeps_the_outcome() {
    for r in ["true", "false"] {
        let dir = fixture("husky-v9");
        write_exec(
            &dir.join(".husky/_/pre-commit"),
            "#!/usr/bin/env sh\nsh -e \"$(dirname \"$0\")/../$(basename \"$0\")\" \"$@\"\n",
        );
        let hook = dir.join(".husky/pre-commit");
        std::fs::write(&hook, format!("echo orig\n{r}\n")).unwrap();
        common::git(&dir, &["config", "core.hooksPath", ".husky/_"]);
        assert_round_trip(&dir, &hook, &format!("husky-v9/{r}"));
    }
}

fn status_of(dir: &Path, content: &str) -> serde_json::Value {
    write_exec(&dir.join(".git/hooks/pre-commit"), content);
    pre_commit_check(dir)
}

#[test]
fn text_proves_nothing_but_the_managed_block() {
    let dir = fixture("status");
    for hook in [
        "\"${URZUA_BIN:-urzua}\" hooks run pre-commit || exit $?",
        "exit 0; urzua check",
        "x=\"urzua check\"",
        ": urzua check",
        "true || urzua check",
        "if false; then urzua check; fi",
        "urzua check &",
        "urzua check || true",
        "urzua version",
        "urzua hooks status",
    ] {
        let c = status_of(&dir, &format!("#!/bin/sh\n{hook}\n"));
        assert_eq!(c["state"], "installed-unmanaged-call", "{hook}: {c}");
        assert_eq!(c["status"], "warn");
        assert_eq!(c["fix"], "urzua hooks install");
        let m = c["message"].as_str().unwrap();
        assert!(
            m.contains("managed: declared") && m.contains("urzua hooks verify --run"),
            "{m}"
        );
        let (_, report) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(states(&report)[0], "pre-commit=extended", "{hook}");
        record(&dir, "ADR-40-broken", true);
        let out = commit(&dir);
        assert!(!out.status.success(), "{hook}: a broken commit passed");
        std::fs::remove_file(dir.join("docs/adr/ADR-40-broken.md")).unwrap();
        common::git(&dir, &["add", "-A"]);
        urzua(&dir, &["hooks", "uninstall"]);
    }
    write_exec(&dir.join(".git/hooks/pre-commit"), "#!/bin/sh\necho a\n");
    urzua(&dir, &["hooks", "install"]);
    let hook = dir.join(".git/hooks/pre-commit");
    let text = std::fs::read_to_string(&hook).unwrap();
    std::fs::write(&hook, text.replace("urzua_code=0", "urzua_code=0; true")).unwrap();
    assert_eq!(pre_commit_check(&dir)["state"], "installed-stale-block");

    let push = std::fs::read_to_string(dir.join(".git/hooks/pre-push")).unwrap();
    write_exec(&hook, &push);
    assert_ne!(
        pre_commit_check(&dir)["status"],
        "ok",
        "a pre-push block in pre-commit"
    );
}

/// Points a repository's hooks at an outside directory.
type Setup = dyn Fn(&Path, &Path);

#[test]
fn uninstall_never_writes_outside_the_repository() {
    let block_file = |dir: &Path| {
        let probe = fixture("uninstall-probe");
        urzua(&probe, &["hooks", "install"]);
        let text = std::fs::read_to_string(probe.join(".git/hooks/pre-commit")).unwrap();
        write_exec(&dir.join("pre-commit"), &text);
        text
    };
    let cases: Vec<(&str, Box<Setup>)> = vec![
        (
            "symlinked-dir",
            Box::new(|repo, out| {
                std::os::unix::fs::symlink(out, repo.join("myhooks")).unwrap();
                common::git(repo, &["config", "core.hooksPath", "myhooks"]);
            }),
        ),
        (
            "husky-symlink",
            Box::new(|repo, out| {
                std::os::unix::fs::symlink(out, repo.join(".husky")).unwrap();
                common::git(repo, &["config", "core.hooksPath", ".husky"]);
            }),
        ),
        (
            "symlinked-parent",
            Box::new(|repo, out| {
                std::fs::create_dir_all(out.join("hooks")).unwrap();
                std::fs::rename(out.join("pre-commit"), out.join("hooks/pre-commit")).unwrap();
                std::os::unix::fs::symlink(out, repo.join("link")).unwrap();
                common::git(repo, &["config", "core.hooksPath", "link/hooks"]);
            }),
        ),
        (
            "absolute",
            Box::new(|repo, out| {
                common::git(repo, &["config", "core.hooksPath", out.to_str().unwrap()]);
            }),
        ),
    ];
    for (label, setup) in cases {
        let repo = fixture(label);
        let outside = common::TempDir::new("urzua-hooks-uninstall-outside");
        block_file(&outside);
        setup(&repo, &outside);
        let file = if label == "symlinked-parent" {
            outside.join("hooks/pre-commit")
        } else {
            outside.join("pre-commit")
        };
        let before = std::fs::read(&file).unwrap();
        let (code, report) = urzua(&repo, &["hooks", "uninstall"]);
        assert_eq!(code, 2, "{label}: {report}");
        assert!(
            report["problems"].to_string().contains("by hand"),
            "{label}: {report}"
        );
        assert_eq!(std::fs::read(&file).unwrap(), before, "{label}");
    }

    let repo = fixture("uninstall-symlinked-file");
    let shared = common::TempDir::new("urzua-hooks-uninstall-shared");
    block_file(&shared);
    std::fs::create_dir_all(repo.join(".git/hooks")).unwrap();
    std::os::unix::fs::symlink(
        shared.join("pre-commit"),
        repo.join(".git/hooks/pre-commit"),
    )
    .unwrap();
    let before = std::fs::read(shared.join("pre-commit")).unwrap();
    let (code, _) = urzua(&repo, &["hooks", "uninstall"]);
    assert_eq!(code, 2);
    assert_eq!(std::fs::read(shared.join("pre-commit")).unwrap(), before);
}

#[test]
fn an_unreadable_config_still_blocks_when_block_was_installed() {
    let dir = fixture("unreadable");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  on_not_run: block\n"),
    )
    .unwrap();
    urzua(&dir, &["hooks", "install"]);
    std::fs::write(dir.join(".urzua/config.yaml"), "schema_version: [\n").unwrap();
    record(&dir, "ADR-30-x", false);
    let out = commit(&dir);
    assert!(
        !out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn hooks_may_not_read_another_file() {
    let dir = fixture("config-flag");
    for bad in [
        "[check, --config, /etc/passwd]",
        "[check, --stdin-path, x.md]",
    ] {
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            format!("{CONFIG}hooks:\n  pre-commit: {bad}\n"),
        )
        .unwrap();
        let (code, report) = urzua(&dir, &["hooks", "status"]);
        assert_eq!(code, 2, "{bad}: {report}");
        assert!(!report.to_string().contains("root:"), "{report}");
    }
}
