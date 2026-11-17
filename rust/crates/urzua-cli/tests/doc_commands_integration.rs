//! `MILE-266`: every `urzua` command in a fenced shell block of README.md and
//! docs/reference/*.md runs against a fixture without a usage error, under a
//! `gh` that verifies a login. A command that needs the network or a real pull
//! request is allow-listed with its reason.
//!
//! Verifies: SPEC-2

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{commit_all, git};

/// Commands not run, each with why.
const ALLOWED: &[(&str, &str)] = &[];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// Each `urzua ...` line of a fenced `sh`, `bash`, `shell`, `console` or
/// unlabelled block, a leading `$ ` dropped and `\` continuations joined, with
/// the file it came from.
fn documented_commands() -> Vec<(String, String)> {
    let root = repo_root();
    let mut files = vec![root.join("README.md")];
    let mut reference: Vec<PathBuf> = std::fs::read_dir(root.join("docs/reference"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    reference.sort();
    files.extend(reference);
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        let lines: Vec<&str> = text.lines().collect();
        let mut fence: Option<String> = None;
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if let Some(rest) = trimmed.strip_prefix("```") {
                fence = match fence {
                    None => Some(rest.trim().to_string()),
                    Some(_) => None,
                };
                i += 1;
                continue;
            }
            let shell = fence
                .as_deref()
                .is_some_and(|l| ["", "sh", "bash", "shell", "console"].contains(&l));
            let line = trimmed.strip_prefix("$ ").unwrap_or(trimmed);
            if shell && line.starts_with("urzua ") {
                let mut command = line.to_string();
                while command.ends_with('\\') && i + 1 < lines.len() {
                    command.pop();
                    i += 1;
                    command.push_str(lines[i].trim());
                }
                out.push((name.clone(), command));
            }
            i += 1;
        }
    }
    out
}

fn bin_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("doc-commands-bin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let gh = dir.join("gh");
    if !gh.exists() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&gh, "#!/bin/sh\necho maintainer\n").unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let link = dir.join("urzua");
    if !link.exists() {
        let _ = std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_urzua"), &link);
    }
    dir
}

const CONFIG: &str = "schema_version: 2
rules:
  embodiment.consistency: warn
  header.required-fields: warn
  embodiment.claim-expired:
    level: warn
    anchor_field: Last-audited
    require_pin: [code]
    periods:
      - {type: adr, status: [Accepted], days: 30}
record_types:
  adr:
    dir: docs/adr
    prefix: ADR
    header_shape: yaml-frontmatter
    required_fields: [Status, Deciders]
    known_fields: [Embodiment, Realized-by, Last-audited]
    pointer_fields: []
    narrative_fields: []
    statuses:
      values: [Proposed, Accepted]
      success: [Accepted]
  waiver:
    dir: docs/waiver
    prefix: WAIVER
    role: suppression
    header_shape: yaml-frontmatter
    required_fields: [Rule, Scope, Reason]
    known_fields: [Expires, Category, Justified-by]
    effective_requires: [Reason, Expires]
";

fn adr(n: &str, status: &str, extra: &str) -> String {
    format!(
        "---\nStatus: {status}\n{extra}---\n# {n} - X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Filed. | **structural** |\n"
    )
}

/// A governed repository holding the records the pages name, with a drifted
/// claim, a stale pin and an `origin` to diff against.
fn governed(name: &str) -> (common::TempDir, common::TempDir) {
    let dir = common::TempDir::new(&format!("urzua-doc-{name}"));
    for d in ["docs/adr", "docs/waiver", "src", ".urzua"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q", "-b", "main"]);
    std::fs::write(dir.join(".urzua/config.yaml"), CONFIG).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
    let pinned =
        "code:src/lib.rs@sha256:0000000000000000000000000000000000000000000000000000000000000000";
    std::fs::write(
        dir.join("docs/adr/ADR-0001-use-postgres.md"),
        adr(
            "1",
            "Accepted",
            "Deciders: a\nEmbodiment: Specified\nRealized-by: code:src/lib.rs\n",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-42-example.md"),
        adr(
            "42",
            "Accepted",
            "Deciders: a\nEmbodiment: Specified\nRealized-by: code:src/lib.rs\n",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-3-x.md"),
        adr(
            "3",
            "Proposed",
            &format!("Embodiment: Implemented\nRealized-by: {pinned}\n"),
        ),
    )
    .unwrap();
    commit_all(&dir, 40);
    let origin = common::TempDir::new(&format!("urzua-doc-{name}-origin"));
    git(
        &origin,
        &["clone", "-q", "--bare", dir.to_str().unwrap(), "."],
    );
    git(&dir, &["remote", "add", "origin", origin.to_str().unwrap()]);
    git(&dir, &["fetch", "-q", "origin"]);
    (dir, origin)
}

fn empty(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-doc-{name}"));
    git(&dir, &["init", "-q"]);
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "init"]);
    dir
}

#[test]
fn every_documented_urzua_command_runs_against_a_fixture() {
    let commands = documented_commands();
    assert!(
        commands.len() > 20,
        "the extraction found the commands: {commands:?}"
    );
    let mut path = bin_dir().into_os_string();
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    let mut failures = Vec::new();
    for (i, (file, command)) in commands.iter().enumerate() {
        if let Some((_, why)) = ALLOWED.iter().find(|(c, _)| c == command) {
            assert!(!why.is_empty());
            continue;
        }
        // `init --types` starts a repository, `init` adopts one with records
        // and no configuration, `harness update` and `harness status` read an
        // installed harness; everything else works on a governed one.
        let (dir, _origin) = if command.starts_with("urzua init --types") {
            (empty(&format!("{i}")), None)
        } else if command.starts_with("urzua init") {
            let (dir, origin) = governed(&format!("{i}"));
            std::fs::remove_file(dir.join(".urzua/config.yaml")).unwrap();
            git(&dir, &["add", "-A"]);
            git(&dir, &["commit", "-q", "-m", "unconfigured"]);
            (dir, Some(origin))
        } else if command.starts_with("urzua harness update")
            || command.starts_with("urzua harness status")
        {
            let (dir, origin) = governed(&format!("{i}"));
            let out = common::urzua()
                .args(["harness", "install", "--adapter", "claude-code"])
                .current_dir(&dir)
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(0), "{out:?}");
            commit_all(&dir, 0);
            (dir, Some(origin))
        } else {
            let (dir, origin) = governed(&format!("{i}"));
            (dir, Some(origin))
        };
        let out = Command::new("sh")
            .arg("-c")
            .arg(command)
            .env("PATH", &path)
            .current_dir(&dir)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        let not_run = out.status.code() == Some(2)
            || serde_json::from_str::<serde_json::Value>(&text)
                .is_ok_and(|v| v["status"] == "not-run");
        if not_run {
            failures.push(format!(
                "{file}: `{command}`\n{text}{}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
