//! `urzua hooks install|status|uninstall|run` in real git repositories, one per
//! hook layout (`MILE-220`), and `doctor` reading the same wiring.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Output;

const CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [Status]\n";

fn fixture(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-hooks-{name}"));
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

fn hook_checks(dir: &Path) -> Vec<serde_json::Value> {
    let (_, doctor) = urzua(dir, &["doctor"]);
    doctor["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "hook-wired")
        .cloned()
        .collect()
}

fn assert_doctor_wired(dir: &Path, layout: &str) {
    let checks = hook_checks(dir);
    assert_eq!(checks.len(), 2, "{checks:?}");
    for c in &checks {
        assert_eq!(c["status"], "ok", "{c}");
        assert_eq!(c["hook_layout"], layout, "{c}");
    }
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode()
}

fn write_exec(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// `git commit` with the hook able to find this build through `URZUA_BIN`.
fn commit(dir: &Path, msg: &str) -> Output {
    common::isolated_git()
        .args(["commit", "-q", "-m", msg])
        .env("URZUA_BIN", env!("CARGO_BIN_EXE_urzua"))
        .env("PATH", common::path_with_stub_gh())
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn default_layout_gets_both_hooks_and_doctor_then_passes() {
    let dir = fixture("default");
    let checks = hook_checks(&dir);
    assert!(
        checks.iter().all(|c| c["fix"] == "urzua hooks install"),
        "{checks:?}"
    );
    let (_, doctor) = urzua(&dir, &["doctor"]);
    assert_eq!(doctor["next_action"], "urzua hooks install");

    let (code, dry) = urzua(&dir, &["hooks", "install", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(
        states(&dry),
        ["pre-commit=would-create", "pre-push=would-create"]
    );
    assert!(!dir.join(".git/hooks/pre-commit").exists());

    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["layout"], "default");
    assert_eq!(states(&report), ["pre-commit=created", "pre-push=created"]);
    for hook in ["pre-commit", "pre-push"] {
        let path = dir.join(".git/hooks").join(hook);
        assert_ne!(mode(&path) & 0o111, 0);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.starts_with("#!/bin/sh\n# >>> urzua-hooks begin"),
            "{text}"
        );
        assert!(text.contains(&format!("hooks run {hook}")), "{text}");
    }
    assert_doctor_wired(&dir, "default");
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 0, "{status}");
    assert_eq!(states(&status), ["pre-commit=ok", "pre-push=ok"]);

    let (_, again) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(
        states(&again),
        ["pre-commit=unchanged", "pre-push=unchanged"]
    );

    let (code, removed) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(code, 0);
    assert_eq!(states(&removed), ["pre-commit=removed", "pre-push=removed"]);
    assert!(!dir.join(".git/hooks/pre-commit").exists());
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 1);
    assert_eq!(status["next_action"], "urzua hooks install");
}

#[test]
fn a_versioned_hooks_path_is_extended_in_place() {
    let dir = fixture("hooks-path");
    common::git(&dir, &["config", "core.hooksPath", "tools/hooks"]);
    write_exec(
        &dir.join("tools/hooks/pre-commit"),
        "#!/usr/bin/env bash\nmake lint\n",
    );
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["layout"], "hooks-path");
    assert_eq!(states(&report), ["pre-commit=extended", "pre-push=created"]);
    let text = std::fs::read_to_string(dir.join("tools/hooks/pre-commit")).unwrap();
    assert!(
        text.starts_with("#!/usr/bin/env bash\nmake lint\n# >>> urzua-hooks begin"),
        "{text}"
    );
    assert!(!dir.join(".git/hooks/pre-push").exists());
    assert_doctor_wired(&dir, "hooks-path");

    let (_, removed) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(
        states(&removed),
        ["pre-commit=block-removed", "pre-push=removed"]
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("tools/hooks/pre-commit")).unwrap(),
        "#!/usr/bin/env bash\nmake lint\n"
    );
}

#[test]
fn husky_hooks_get_the_block_before_a_trailing_exec() {
    let dir = fixture("husky");
    std::fs::create_dir_all(dir.join(".husky/_")).unwrap();
    std::fs::write(
        dir.join(".husky/pre-commit"),
        "npx lint-staged\nexec node scripts/verify.js\n",
    )
    .unwrap();
    common::git(&dir, &["config", "core.hooksPath", ".husky/_"]);
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["layout"], "husky");
    assert_eq!(states(&report), ["pre-commit=extended", "pre-push=created"]);
    let text = std::fs::read_to_string(dir.join(".husky/pre-commit")).unwrap();
    let block = text.find("# >>> urzua-hooks begin").unwrap();
    let exec = text.find("exec node").unwrap();
    assert!(
        text.starts_with("npx lint-staged\n") && block < exec,
        "{text}"
    );
    assert!(text.ends_with("exec node scripts/verify.js\n"), "{text}");
    assert!(dir.join(".husky/pre-push").is_file());
    assert_doctor_wired(&dir, "husky");
}

#[test]
fn a_hook_that_already_runs_urzua_is_left_alone() {
    let dir = fixture("wired");
    let original = "#!/bin/sh\nbin=\"$HOME/bin/urzua\"\n\"$bin\" check --base HEAD\n";
    write_exec(&dir.join(".git/hooks/pre-commit"), original);
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(states(&report), ["pre-commit=wired", "pre-push=created"]);
    assert_eq!(
        std::fs::read_to_string(dir.join(".git/hooks/pre-commit")).unwrap(),
        original
    );
    let (_, removed) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(
        states(&removed),
        ["pre-commit=not-ours", "pre-push=removed"]
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".git/hooks/pre-commit")).unwrap(),
        original
    );
}

#[test]
fn a_node_or_python_hook_is_refused_with_the_line_to_add() {
    for (shebang, line) in [
        (
            "#!/usr/bin/env node",
            "require(\"child_process\").execFileSync(process.env.URZUA_BIN || \"urzua\"",
        ),
        (
            "#!/usr/bin/python3",
            "subprocess.run([os.environ.get(\"URZUA_BIN\", \"urzua\")",
        ),
    ] {
        let dir = fixture("foreign");
        let original = format!("{shebang}\nconsole.log('x')\n");
        write_exec(&dir.join(".git/hooks/pre-commit"), &original);
        let (code, report) = urzua(&dir, &["hooks", "install"]);
        assert_eq!(code, 2, "{report}");
        assert_eq!(report["status"], "not-run");
        let problems = report["problems"].to_string();
        assert!(
            problems.contains("will not add a shell block"),
            "{problems}"
        );
        assert!(problems.contains(&line.replace('"', "\\\"")), "{problems}");
        assert_eq!(
            std::fs::read_to_string(dir.join(".git/hooks/pre-commit")).unwrap(),
            original
        );
        assert!(
            !dir.join(".git/hooks/pre-push").exists(),
            "a refused run writes nothing"
        );
    }
}

#[test]
fn a_hooks_path_outside_the_repository_is_refused_with_the_line_to_add() {
    let dir = fixture("outside");
    let outside = common::TempDir::new("urzua-hooks-outside-target");
    common::git(
        &dir,
        &["config", "core.hooksPath", outside.to_str().unwrap()],
    );
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    let problems = report["problems"].to_string();
    assert!(problems.contains("outside the repository"), "{problems}");
    assert!(
        problems.contains("hooks run pre-commit || exit $?"),
        "{problems}"
    );
    assert!(!outside.join("pre-commit").exists());
    let checks = hook_checks(&dir);
    assert!(
        checks[0]["fix"].as_str().unwrap().starts_with("add `"),
        "{checks:?}"
    );
}

#[test]
fn a_read_only_hooks_path_is_refused() {
    if !common::permission_test_can_run("a_read_only_hooks_path_is_refused") {
        return;
    }
    let dir = fixture("read-only");
    std::fs::create_dir_all(dir.join("ro-hooks")).unwrap();
    common::git(&dir, &["config", "core.hooksPath", "ro-hooks"]);
    std::fs::set_permissions(dir.join("ro-hooks"), std::fs::Permissions::from_mode(0o555)).unwrap();
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    std::fs::set_permissions(dir.join("ro-hooks"), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 2, "{report}");
    assert!(
        report["problems"].to_string().contains("cannot be written"),
        "{report}"
    );
}

const FRAMEWORK: &str = "repos:\n  - repo: https://example.com/hooks\n    rev: v1\n    hooks:\n      - id: trailing-whitespace\n";

#[test]
fn the_pre_commit_framework_gets_a_stanza_not_hook_files() {
    let dir = fixture("framework");
    std::fs::write(dir.join(".pre-commit-config.yaml"), FRAMEWORK).unwrap();
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(report["layout"], "pre-commit-framework");
    assert_eq!(states(&report), ["pre-commit-framework=stanza-needed"]);
    assert_eq!(report["next_action"], "urzua hooks install --write-config");
    assert!(!dir.join(".git/hooks/pre-commit").exists());
    assert_eq!(
        std::fs::read_to_string(dir.join(".pre-commit-config.yaml")).unwrap(),
        FRAMEWORK
    );
    let checks = hook_checks(&dir);
    assert!(checks
        .iter()
        .all(|c| c["fix"] == "urzua hooks install --write-config"));

    let (code, report) = urzua(&dir, &["hooks", "install", "--write-config"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(states(&report), ["pre-commit-framework=extended"]);
    let text = std::fs::read_to_string(dir.join(".pre-commit-config.yaml")).unwrap();
    assert!(text.starts_with(FRAMEWORK));
    assert_stanza_shape(&text);
    assert!(
        !dir.join(".git/hooks/pre-commit").exists(),
        "no hook file is written"
    );
    // The framework's own scripts are not installed yet.
    let checks = hook_checks(&dir);
    assert_eq!(
        checks[0]["fix"],
        "pre-commit install --hook-type pre-commit"
    );
    write_exec(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\n# framework script\n",
    );
    write_exec(
        &dir.join(".git/hooks/pre-push"),
        "#!/bin/sh\n# framework script\n",
    );
    assert_doctor_wired(&dir, "pre-commit-framework");

    let (_, again) = urzua(&dir, &["hooks", "install", "--write-config"]);
    assert_eq!(states(&again), ["pre-commit-framework=wired"]);
    let (_, removed) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(states(&removed), ["pre-commit-framework=block-removed"]);
    assert_eq!(
        std::fs::read_to_string(dir.join(".pre-commit-config.yaml")).unwrap(),
        FRAMEWORK
    );
}

/// The stanza as YAML: one `repo: local` entry whose two hooks run urzua at
/// their own stage, with no filenames passed.
fn assert_stanza_shape(text: &str) {
    let doc: yaml_serde::Value = yaml_serde::from_str(text).expect("the config stays YAML");
    let repos = doc["repos"].as_sequence().unwrap();
    let local = repos
        .iter()
        .find(|r| r["repo"].as_str() == Some("local"))
        .expect("a local repo");
    let hooks = local["hooks"].as_sequence().unwrap();
    assert_eq!(hooks.len(), 2);
    for (hook, stage) in hooks.iter().zip(["pre-commit", "pre-push"]) {
        assert_eq!(hook["id"].as_str(), Some(format!("urzua-{stage}").as_str()));
        assert_eq!(
            hook["entry"].as_str(),
            Some(format!("urzua hooks run {stage}").as_str())
        );
        assert_eq!(hook["language"].as_str(), Some("system"));
        assert_eq!(hook["pass_filenames"].as_bool(), Some(false));
        assert_eq!(hook["always_run"].as_bool(), Some(true));
        let stages: Vec<_> = hook["stages"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        assert_eq!(stages, [stage]);
    }
}

/// Runs the stanza through the real pre-commit framework when it is on `PATH`.
/// CI's runners do not carry it, and installing a Python tool from the network
/// inside a test is not something this suite does, so elsewhere the test says it
/// was skipped and [`the_pre_commit_framework_gets_a_stanza_not_hook_files`]
/// checks the stanza's shape instead.
#[test]
fn the_stanza_runs_under_the_pre_commit_framework_when_it_is_installed() {
    let found = std::process::Command::new("pre-commit")
        .arg("--version")
        .output();
    if !found.as_ref().is_ok_and(|o| o.status.success()) {
        eprintln!("SKIPPED the_stanza_runs_under_the_pre_commit_framework_when_it_is_installed: `pre-commit` is not on PATH");
        return;
    }
    let dir = fixture("framework-run");
    std::fs::write(dir.join(".pre-commit-config.yaml"), "repos:\n").unwrap();
    let (code, report) = urzua(&dir, &["hooks", "install", "--write-config"]);
    assert_eq!(code, 0, "{report}");
    let bin = common::TempDir::new("urzua-hooks-bin");
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_urzua"), bin.join("urzua")).unwrap();
    let mut path = vec![bin.to_path_buf()];
    path.extend(std::env::split_paths(&common::path_with_stub_gh()));
    let path = std::env::join_paths(path).unwrap();
    let run = |stage: &str| {
        std::process::Command::new("pre-commit")
            .args(["run", "--all-files", "--hook-stage", stage])
            .env("PATH", &path)
            .current_dir(&dir)
            .output()
            .unwrap()
    };
    common::git(&dir, &["add", "-A"]);
    let ok = run("pre-commit");
    assert!(
        ok.status.success(),
        "{}{}",
        String::from_utf8_lossy(&ok.stdout),
        String::from_utf8_lossy(&ok.stderr)
    );
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nDate: 2026-01-01\n---\n# B\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let blocked = run("pre-commit");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&blocked.stdout),
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(!blocked.status.success(), "{out}");
    assert!(out.contains("header.required-fields"), "{out}");
}

#[test]
fn a_commit_with_a_broken_record_is_blocked_and_a_clean_one_passes() {
    let dir = fixture("commit");
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");

    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nDate: 2026-01-01\n---\n# B\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = commit(&dir, "broken");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("1 blocking finding(s); this commit was stopped"),
        "{stderr}"
    );
    assert!(
        stderr.contains("header.required-fields docs/adr/ADR-2-b.md"),
        "{stderr}"
    );
    assert!(
        stderr.contains("the full report: urzua check --base HEAD"),
        "{stderr}"
    );

    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Draft\n---\n# B\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = commit(&dir, "clean");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        stderr.contains("pre-commit: urzua check --base HEAD: no blocking findings"),
        "{stderr}"
    );
}

#[test]
fn a_missing_binary_blocks_with_how_to_install_and_skip_and_warn_lets_it_through() {
    let dir = fixture("missing");
    let (code, _) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0);
    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nStatus: Draft\n---\n# B\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let commit_without_urzua = |dir: &Path| {
        common::isolated_git()
            .args(["commit", "-q", "-m", "x"])
            .env_remove("URZUA_BIN")
            .env("PATH", "/usr/bin:/bin")
            .current_dir(dir)
            .output()
            .unwrap()
    };
    if std::path::Path::new("/usr/bin/urzua").exists()
        || std::path::Path::new("/bin/urzua").exists()
    {
        eprintln!("SKIPPED a_missing_binary_blocks...: urzua is installed system-wide");
        return;
    }
    let out = commit_without_urzua(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("urzua is not installed"), "{stderr}");
    assert!(stderr.contains("git commit --no-verify"), "{stderr}");
    assert!(stderr.contains("install.md"), "{stderr}");

    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  on_not_run: warn\n"),
    )
    .unwrap();
    let (_, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(states(&report), ["pre-commit=updated", "pre-push=updated"]);
    common::git(&dir, &["add", "-A"]);
    let out = commit_without_urzua(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("urzua is not installed"), "{stderr}");
}

#[test]
fn the_hooks_key_names_what_each_hook_runs_and_rejects_an_empty_list() {
    let dir = fixture("config");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  pre-commit: [check]\n"),
    )
    .unwrap();
    let out = common::urzua()
        .args(["hooks", "run", "pre-commit"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("pre-commit: urzua check: no blocking findings"));

    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  pre-push: []\n"),
    )
    .unwrap();
    let (code, report) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report.to_string().contains("hooks: 'pre-push' must list"),
        "{report}"
    );
}
