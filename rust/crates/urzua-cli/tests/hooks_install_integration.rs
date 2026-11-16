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
        text.starts_with("#!/usr/bin/env bash\n# >>> urzua-hooks begin")
            && text.ends_with("# <<< urzua-hooks end\nmake lint\n"),
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
fn husky_hooks_get_the_block_at_the_top() {
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
    let exec = text.find("npx lint-staged").unwrap();
    assert!(
        text.starts_with("# >>> urzua-hooks begin") && block < exec,
        "{text}"
    );
    assert!(text.ends_with("exec node scripts/verify.js\n"), "{text}");
    assert!(dir.join(".husky/pre-push").is_file());
    assert_doctor_wired(&dir, "husky");
}

#[test]
fn a_hand_written_call_gets_the_managed_block_too() {
    let dir = fixture("wired");
    let original = "#!/bin/sh\n# runs the gate\n\"$HOME/bin/urzua\" check --base HEAD\n";
    write_exec(&dir.join(".git/hooks/pre-commit"), original);
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(states(&report), ["pre-commit=extended", "pre-push=created"]);
    let (_, removed) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(
        states(&removed),
        ["pre-commit=block-removed", "pre-push=removed"]
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
        problems.contains("hooks run pre-commit || exit 1"),
        "{problems}"
    );
    assert!(!outside.join("pre-commit").exists());
    let checks = hook_checks(&dir);
    assert!(checks[0].get("fix").is_none(), "{checks:?}");
    assert!(
        checks[0]["message"]
            .as_str()
            .unwrap()
            .contains("hooks run pre-commit || exit 1"),
        "{checks:?}"
    );
    let (_, doctor) = urzua(&dir, &["doctor"]);
    assert!(doctor["next_action"].is_null(), "{doctor}");
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
        let entry = hook["entry"].as_str().unwrap();
        assert!(entry.starts_with("sh -c '"), "{entry}");
        assert!(entry.contains(&format!("hooks run {stage}")), "{entry}");
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
    assert!(!stderr.contains("no blocking findings"), "{stderr}");
}

#[test]
fn a_missing_binary_warns_loudly_by_default_and_blocks_when_configured() {
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
    if Path::new("/usr/bin/urzua").exists() || Path::new("/bin/urzua").exists() {
        eprintln!("SKIPPED a_missing_binary_warns...: urzua is installed system-wide");
        return;
    }
    let out = commit_without_urzua(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(stderr.contains("urzua did not run"), "{stderr}");
    assert!(stderr.contains("NOT checked"), "{stderr}");
    assert!(stderr.contains("CI will check"), "{stderr}");
    assert!(stderr.contains("install.md"), "{stderr}");
    assert!(!stderr.contains("--no-verify"), "{stderr}");

    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  on_not_run: block\n"),
    )
    .unwrap();
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 1, "{status}");
    assert_eq!(
        states(&status),
        ["pre-commit=stale-policy", "pre-push=stale-policy"]
    );
    assert_eq!(status["next_action"], "urzua hooks install");
    let (_, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(states(&report), ["pre-commit=updated", "pre-push=updated"]);
    std::fs::write(
        dir.join("docs/adr/ADR-3-c.md"),
        "---\nStatus: Draft\n---\n# C\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = commit_without_urzua(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(stderr.contains("git commit --no-verify"), "{stderr}");
}

#[test]
fn the_hooks_key_names_what_each_hook_runs_and_rejects_an_empty_list() {
    let dir = fixture("config");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  pre-commit: [check]\n  print_success: true\n"),
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
        report
            .to_string()
            .contains("hooks: 'pre-push' must run `check`"),
        "{report}"
    );
}

#[test]
fn a_clean_hooks_run_prints_nothing_and_a_refusal_is_unchanged() {
    let dir = fixture("quiet");
    let run = |dir: &Path| {
        common::urzua()
            .args(["hooks", "run", "pre-commit"])
            .current_dir(dir)
            .output()
            .unwrap()
    };
    let out = run(&dir);
    assert!(out.status.success(), "{out:?}");
    assert!(out.stdout.is_empty() && out.stderr.is_empty(), "{out:?}");

    std::fs::write(
        dir.join("docs/adr/ADR-2-b.md"),
        "---\nDate: 2026-01-01\n---\n# B\n",
    )
    .unwrap();
    common::git(&dir, &["add", "-A"]);
    let out = run(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("1 blocking finding(s)"), "{stderr}");
    assert!(
        stderr.contains("header.required-fields docs/adr/ADR-2-b.md"),
        "{stderr}"
    );
    assert!(
        stderr.contains("the full report: urzua check --base HEAD"),
        "{stderr}"
    );
}

fn commit_hook_dir(dir: &Path) {
    for hook in ["pre-commit", "pre-push"] {
        write_exec(
            &dir.join(".git-hooks").join(hook),
            &format!("#!/bin/sh\n\"${{URZUA_BIN:-urzua}}\" hooks run {hook}\n"),
        );
    }
}

const DIR_CONFIG: &str = "hooks:\n  path: .git-hooks\n  pre-commit: {managed: declared}\n  pre-push: {managed: declared}\n";

#[test]
fn a_named_hook_directory_is_reported_until_install_points_the_clone_at_it() {
    let dir = fixture("hook-dir");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    commit_hook_dir(&dir);

    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 1, "{status}");
    assert_eq!(
        states(&status),
        ["pre-commit=not-pointed", "pre-push=not-pointed"]
    );
    assert_eq!(status["next_action"], "urzua hooks install", "{status}");
    let text = status.to_string();
    assert!(text.contains("not pointed at .git-hooks"), "{status}");
    assert!(!text.contains(".git/hooks"), "{status}");

    let checks = hook_checks(&dir);
    assert!(
        checks.iter().all(|c| c["state"] == "not-pointed"),
        "{checks:?}"
    );
    assert!(
        checks
            .iter()
            .all(|c| !c["message"].as_str().unwrap().contains(".git/hooks")),
        "{checks:?}"
    );
    let (_, doctor) = urzua(&dir, &["doctor"]);
    assert_eq!(doctor["next_action"], "urzua hooks install", "{doctor}");

    let (code, dry) = urzua(&dir, &["hooks", "install", "--dry-run"]);
    assert_eq!(code, 0, "{dry}");
    assert_eq!(states(&dry)[0], "core.hooksPath=would-point");
    let unset = common::isolated_git()
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(!unset.status.success());

    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(
        states(&report),
        [
            "core.hooksPath=pointed",
            "pre-commit=declared",
            "pre-push=declared"
        ]
    );
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 0, "{status}");
    assert_eq!(
        states(&status),
        ["pre-commit=declared", "pre-push=declared"]
    );

    let (code, report) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(code, 0, "{report}");
    assert!(
        states(&report).contains(&"core.hooksPath=unset".to_string()),
        "{report}"
    );
    let (_, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(
        states(&status),
        ["pre-commit=not-pointed", "pre-push=not-pointed"]
    );
}

fn urzua_env(dir: &Path, args: &[&str], env: &[(&str, &Path)]) -> (i32, serde_json::Value) {
    let mut cmd = common::urzua();
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.args(args).current_dir(dir).output().unwrap();
    let json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("urzua {args:?}: {}", String::from_utf8_lossy(&out.stderr)));
    (out.status.code().unwrap(), json)
}

fn local_hooks_path(dir: &Path) -> Option<String> {
    let out = common::isolated_git()
        .args(["config", "--local", "--get", "core.hooksPath"])
        .current_dir(dir)
        .output()
        .unwrap();
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Another `core.hooksPath`, set locally or globally: no next action, the
/// message names its scope and the command a person may choose, and nothing
/// suggests unsetting it.
fn assert_conflict_reported(dir: &Path, env: &[(&str, &Path)], scope: &str) {
    let (code, report) = urzua_env(dir, &["hooks", "install"], env);
    assert_eq!(code, 2, "{report}");
    let problem = report["problems"][0].as_str().unwrap();
    assert!(problem.contains(&format!("scope {scope}")), "{report}");
    assert!(
        problem.contains("git config --local core.hooksPath .git-hooks"),
        "{report}"
    );
    assert!(!report.to_string().contains("--unset"), "{report}");

    let (_, status) = urzua_env(dir, &["hooks", "status"], env);
    assert!(status["next_action"].is_null(), "{status}");
    assert!(
        status.to_string().contains(&format!("scope {scope}")),
        "{status}"
    );
    assert!(!status.to_string().contains("--unset"), "{status}");
    let (_, doctor) = urzua_env(dir, &["doctor"], env);
    for c in doctor["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "hook-wired")
    {
        assert!(c.get("fix").is_none(), "{c}");
        assert!(
            c["message"]
                .as_str()
                .unwrap()
                .contains(&format!("scope {scope}")),
            "{c}"
        );
    }
}

#[test]
fn a_local_hooks_path_set_elsewhere_is_named_and_left_to_a_person() {
    let dir = fixture("hook-dir-local");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    commit_hook_dir(&dir);
    common::git(&dir, &["config", "core.hooksPath", "elsewhere"]);
    assert_conflict_reported(&dir, &[], "local");
    urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(local_hooks_path(&dir).as_deref(), Some("elsewhere"));
}

#[test]
fn a_global_hooks_path_is_named_and_left_to_a_person() {
    let dir = fixture("hook-dir-global");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    commit_hook_dir(&dir);
    let home = common::TempDir::new("urzua-hooks-global-home");
    let global = home.join("gitconfig");
    std::fs::write(&global, "[core]\n\thooksPath = /somewhere/else\n").unwrap();
    assert_conflict_reported(&dir, &[("GIT_CONFIG_GLOBAL", &global)], "global");
    assert_eq!(local_hooks_path(&dir), None);
}

#[test]
fn a_hook_directory_that_resolves_outside_the_repository_is_refused_before_git_config() {
    let dir = fixture("hook-dir-escape");
    let outside = common::TempDir::new("urzua-hooks-escape-target");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, dir.join(".git-hooks")).unwrap();
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["problems"][0]
            .as_str()
            .unwrap()
            .contains("outside the repository"),
        "{report}"
    );
    assert_eq!(local_hooks_path(&dir), None);
}

#[test]
fn install_reports_a_declared_hook_missing_from_the_hook_directory() {
    let dir = fixture("hook-dir-missing");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    write_exec(
        &dir.join(".git-hooks/pre-commit"),
        "#!/bin/sh\nurzua hooks run pre-commit\n",
    );
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 1, "{report}");
    assert_ne!(report["status"], "ok", "{report}");
    assert!(
        states(&report).contains(&"pre-push=declared-missing".to_string()),
        "{report}"
    );
    let (_, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(
        states(&status),
        ["pre-commit=declared", "pre-push=declared-missing"]
    );
}

#[test]
fn uninstall_in_a_fresh_clone_leaves_the_committed_hooks() {
    let origin = fixture("hook-dir-origin");
    std::fs::write(
        origin.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    commit_hook_dir(&origin);
    common::git(&origin, &["add", "-A"]);
    common::git(&origin, &["commit", "-q", "--no-verify", "-m", "hooks"]);
    let parent = common::TempDir::new("urzua-hooks-clone");
    let clone = parent.join("c");
    common::git(&parent, &["clone", "-q", &origin.to_string_lossy(), "c"]);

    let (code, report) = urzua(&clone, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(local_hooks_path(&clone).as_deref(), Some(".git-hooks"));
    let (code, report) = urzua(&clone, &["hooks", "uninstall"]);
    assert_eq!(code, 0, "{report}");
    assert_eq!(
        states(&report),
        ["pre-commit=left", "pre-push=left", "core.hooksPath=unset"]
    );
    assert!(report.to_string().contains("leaves its files"), "{report}");
    for hook in ["pre-commit", "pre-push"] {
        assert!(clone.join(".git-hooks").join(hook).is_file());
    }
    let st = common::isolated_git()
        .args(["status", "--porcelain"])
        .current_dir(&clone)
        .output()
        .unwrap();
    assert!(
        st.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&st.stdout)
    );
    assert_eq!(local_hooks_path(&clone), None);
}

#[test]
fn a_declared_hook_with_no_hook_directory_does_not_ask_for_one_under_the_git_directory() {
    let dir = fixture("declared-default");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  pre-commit: {{managed: declared}}\n"),
    )
    .unwrap();
    let checks = hook_checks(&dir);
    let pre_commit = checks.iter().find(|c| c["layer"] == "pre-commit").unwrap();
    let m = pre_commit["message"].as_str().unwrap();
    assert!(m.contains("hooks.path"), "{m}");
    assert!(!m.contains("write the hook"), "{m}");
}

#[test]
fn a_hook_directory_outside_the_repository_is_a_config_error() {
    let dir = fixture("hook-dir-bad");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  path: ../x\n"),
    )
    .unwrap();
    let (code, _) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 2);
    for bad in [
        ".git/hooks",
        ".git",
        ".",
        "./x",
        "a//b",
        "/abs",
        "~/hk",
        "~",
        ".GIT/hooks",
        ".Git",
        "C:\\x",
        "a\\..\\..\\x",
        "a\nb",
        "sub/.git/hooks",
    ] {
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            format!(
                "{CONFIG}hooks:\n  path: {}\n",
                serde_json::to_string(bad).unwrap()
            ),
        )
        .unwrap();
        let (code, report) = urzua(&dir, &["hooks", "status"]);
        assert_eq!(code, 2, "{bad}: {report}");
    }
}

fn tree_status(dir: &Path) -> String {
    let out = common::isolated_git()
        .args(["status", "--porcelain", "--ignored"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_symlinked_hook_file_in_the_hook_directory_refuses_before_anything_is_written() {
    let dir = fixture("hook-dir-file-link");
    let outside = common::TempDir::new("urzua-hooks-file-link-target");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  path: .git-hooks\n"),
    )
    .unwrap();
    write_exec(&outside.join("pre-commit"), "#!/bin/sh\necho hi\n");
    std::fs::create_dir_all(dir.join(".git-hooks")).unwrap();
    std::os::unix::fs::symlink(
        outside.join("pre-commit"),
        dir.join(".git-hooks/pre-commit"),
    )
    .unwrap();
    let before = tree_status(&dir);

    let (_, status) = urzua(&dir, &["hooks", "status"]);
    assert!(status["next_action"].is_null(), "{status}");
    assert!(status.to_string().contains("symlink"), "{status}");

    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report["problems"][0]
            .as_str()
            .unwrap()
            .contains("nothing was written"),
        "{report}"
    );
    assert_eq!(local_hooks_path(&dir), None);
    assert_eq!(tree_status(&dir), before);
    assert!(!dir.join(".git-hooks/pre-push").exists());
}

#[test]
fn a_hook_directory_linked_into_the_git_directory_is_refused() {
    let dir = fixture("hook-dir-into-git");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}hooks:\n  path: hk\n"),
    )
    .unwrap();
    std::os::unix::fs::symlink(dir.join(".git/hooks"), dir.join("hk")).unwrap();
    let (_, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(
        states(&status),
        ["pre-commit=unsafe-dir", "pre-push=unsafe-dir"]
    );
    assert!(status["next_action"].is_null(), "{status}");
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 2, "{report}");
    assert!(
        report.to_string().contains("inside the git directory"),
        "{report}"
    );
    assert_eq!(local_hooks_path(&dir), None);
}

#[test]
fn a_hook_directory_replaced_by_a_link_outside_after_install_is_a_problem() {
    let dir = fixture("hook-dir-replaced");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    commit_hook_dir(&dir);
    let (code, report) = urzua(&dir, &["hooks", "install"]);
    assert_eq!(code, 0, "{report}");
    let outside = common::TempDir::new("urzua-hooks-replaced-target");
    for hook in ["pre-commit", "pre-push"] {
        std::fs::copy(dir.join(".git-hooks").join(hook), outside.join(hook)).unwrap();
    }
    std::fs::remove_dir_all(dir.join(".git-hooks")).unwrap();
    std::os::unix::fs::symlink(&outside, dir.join(".git-hooks")).unwrap();
    let (code, status) = urzua(&dir, &["hooks", "status"]);
    assert_eq!(code, 1, "{status}");
    assert_eq!(
        states(&status),
        ["pre-commit=unsafe-dir", "pre-push=unsafe-dir"]
    );
    let checks = hook_checks(&dir);
    assert!(checks.iter().all(|c| c["status"] != "ok"), "{checks:?}");
}

#[test]
fn a_hooks_path_from_an_included_file_is_named_and_left_by_uninstall() {
    let dir = fixture("hook-dir-include");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    commit_hook_dir(&dir);
    let inc = common::TempDir::new("urzua-hooks-include");
    std::fs::write(inc.join("extra"), "[core]\n\thooksPath = .git-hooks\n").unwrap();
    common::git(
        &dir,
        &[
            "config",
            "include.path",
            &inc.join("extra").to_string_lossy(),
        ],
    );
    let (code, report) = urzua(&dir, &["hooks", "uninstall"]);
    assert_eq!(code, 0, "{report}");
    assert!(
        states(&report).contains(&"core.hooksPath=left".to_string()),
        "{report}"
    );
    assert!(report.to_string().contains("extra"), "{report}");
}

#[test]
fn install_lists_a_missing_declared_hook_once() {
    let dir = fixture("hook-dir-once");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{CONFIG}{DIR_CONFIG}"),
    )
    .unwrap();
    write_exec(
        &dir.join(".git-hooks/pre-commit"),
        "#!/bin/sh\nurzua hooks run pre-commit\n",
    );
    let (_, report) = urzua(&dir, &["hooks", "install", "--dry-run"]);
    assert_eq!(
        states(&report),
        [
            "core.hooksPath=would-point",
            "pre-commit=declared",
            "pre-push=declared-missing"
        ]
    );
}
