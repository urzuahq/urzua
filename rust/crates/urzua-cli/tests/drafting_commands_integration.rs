//! ADR-70 / MILE-177: walk each drafting command in `.claude/commands/` end to end.
//!
//! A markdown command cannot be executed, so the test makes its executable part
//! deterministic instead of simulating an agent: every command marks its shell steps
//! as ```` ```bash step ```` blocks, and this test extracts those blocks, in order, from
//! the command file itself and runs them with the real binary. The one step that is
//! judgment (filling in content from the conversation) is the only part the test
//! supplies, standing in for the agent. Because the commands are parsed rather than
//! mirrored, editing a command's steps changes what is tested, and a command that
//! stops calling `urzua new` or `urzua check` fails here. The fixture is a copy of this
//! repository's own working tree, so the real `.urzua/config.yaml` is what runs.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git should run");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A throwaway git repo holding a copy of this repository's working tree (tracked and
/// not-yet-tracked files, minus ignored ones), committed as the baseline.
fn fixture(name: &str) -> common::TempDir {
    let root = repo_root();
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = common::TempDir::new(&format!("urzua-draft-{name}-{n}"));
    let listing = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(&root)
        .output()
        .unwrap()
        .stdout;
    for rel in String::from_utf8_lossy(&listing).split('\0') {
        let src = root.join(rel);
        if rel.is_empty() || !src.is_file() {
            continue;
        }
        let dst = dir.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(&src, &dst).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "baseline"]);
    dir
}

/// The ```` ```bash step ```` blocks of a command file, in order.
fn steps(command: &str) -> Vec<String> {
    let text = std::fs::read_to_string(repo_root().join(".claude/commands").join(command))
        .unwrap_or_else(|e| panic!("{command}: {e}"));
    let mut out = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in text.lines() {
        let t = line.trim();
        match (&mut current, t) {
            (None, "```bash step") => current = Some(Vec::new()),
            (Some(block), "```") => {
                out.push(block.join("\n"));
                current = None;
            }
            (Some(block), _) => block.push(t.to_string()),
            _ => {}
        }
    }
    out
}

fn run_step(dir: &Path, script: &str, title: &str, record: &str) -> Output {
    Command::new("bash")
        .args(["-ec", script])
        .env("TITLE", title)
        .env("RECORD", record)
        .env("URZUA", env!("CARGO_BIN_EXE_urzua"))
        .current_dir(dir)
        .output()
        .expect("bash should run")
}

/// Stands in for the agent's content step: every `null` header field gets a value, and
/// the body's guidance text is replaced by a sentence. Values are test data, chosen to
/// resolve in this repository's corpus.
fn fill(record: &Path) {
    let text = std::fs::read_to_string(record).unwrap();
    let mut out = String::new();
    let mut fence = 0;
    let mut in_body_guidance = false;
    for line in text.lines() {
        if line == "---" {
            fence += 1;
        }
        if fence < 2 {
            let value = |key: &str| -> Option<&'static str> {
                match key {
                    "Deciders" => Some("tester"),
                    "Phase" => Some("'0'"),
                    "Track" => Some("governance-process"),
                    "Implements" => Some("ADR-70"),
                    "Derives-from" => Some("RFC-7"),
                    "Embodiment" | "Supersedes / Superseded-by" | "Blocked-on" => None,
                    _ => None,
                }
            };
            if let Some((key, "null")) = line.split_once(": ") {
                if let Some(v) = value(key) {
                    out.push_str(&format!("{key}: {v}\n"));
                    continue;
                }
            }
        } else if line.starts_with("## ") {
            in_body_guidance = true;
            out.push_str(line);
            out.push_str("\n\nA concrete sentence drafted from the conversation.\n\n");
            continue;
        } else if in_body_guidance
            && !line.starts_with("> ")
            && !line.starts_with('|')
            && (line.is_empty() || !line.starts_with('#'))
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    std::fs::write(record, out).unwrap();
}

fn json_path(stdout: &[u8]) -> String {
    let v: serde_json::Value = serde_json::from_slice(stdout).expect("step 1 prints JSON");
    v["path"].as_str().expect("path").to_string()
}

fn status_line(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .find(|l| l.starts_with("Status:"))
        .unwrap()
        .to_string()
}

/// Run a command's scaffold step; returns (fixture, record path).
fn scaffold(command: &str, title: &str) -> (common::TempDir, String, Vec<String>) {
    let dir = fixture(&command.trim_end_matches(".md").replace('/', "-"));
    let s = steps(command);
    assert_eq!(
        s.len(),
        2,
        "{command}: expected a scaffold and a finish step"
    );
    let out = run_step(&dir, &s[0], title, "");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (dir, json_path(&out.stdout), s)
}

#[test]
fn each_command_drafts_a_record_that_check_examined_and_passes() {
    for (command, expect_indexed) in [("rfc.md", false), ("adr.md", true), ("milestone.md", false)]
    {
        let (dir, record, s) = scaffold(command, &format!("Throwaway {command} record"));
        let scaffolded_status = status_line(&dir.join(&record));
        fill(&dir.join(&record));

        let out = run_step(&dir, &s[1], "", &record);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "{command}: {stderr}\n{}",
            String::from_utf8_lossy(&out.stdout)
        );

        // The check examined the record (not a clean run over nothing) and was clean.
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["files_examined"], 1, "{command}: {report}");
        assert_eq!(
            report["findings"].as_array().unwrap().len(),
            0,
            "{command}: {report}"
        );
        assert!(stderr.contains("examined 1 record"), "{command}: {stderr}");

        // The record, and the regenerated index when the type declares one, are staged.
        let staged = git(&dir, &["diff", "--cached", "--name-only"]);
        assert!(staged.lines().any(|l| l == record), "{command}: {staged}");
        assert_eq!(
            staged.lines().any(|l| l.ends_with("README.md")),
            expect_indexed,
            "{command}: {staged}"
        );
        // Never decide: the status urzua new wrote is untouched.
        assert_eq!(
            status_line(&dir.join(&record)),
            scaffolded_status,
            "{command}"
        );
    }
}

#[test]
fn a_decision_status_written_into_the_draft_is_refused_and_nothing_is_staged() {
    for command in ["rfc.md", "adr.md", "milestone.md"] {
        let (dir, record, s) = scaffold(command, "Throwaway refusal record");
        fill(&dir.join(&record));
        let path = dir.join(&record);
        let scaffolded = status_line(&path);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replacen(&scaffolded, "Status: Accepted", 1)).unwrap();

        let out = run_step(&dir, &s[1], "", &record);
        assert!(!out.status.success(), "{command} must refuse");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("refusing"),
            "{command}"
        );
        assert_eq!(
            git(&dir, &["diff", "--cached", "--name-only"]).trim(),
            "",
            "{command}"
        );
    }
}

#[test]
fn a_field_urzua_new_did_not_write_is_refused() {
    let (dir, record, s) = scaffold("adr.md", "Throwaway extra field record");
    fill(&dir.join(&record));
    let path = dir.join(&record);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replacen("\n---\n#", "\nAccepted-on: 2026-11-04\n---\n#", 1),
    )
    .unwrap();
    let out = run_step(&dir, &s[1], "", &record);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("refusing"));
}

/// The reason the commands stage before checking (ADR-56): a sweep governs tracked
/// files only, so a fresh scaffold is invisible to it until it is staged.
#[test]
fn a_sweep_examines_a_scaffold_only_once_it_is_staged() {
    let (dir, record, _) = scaffold("rfc.md", "Throwaway unstaged record");
    let examined = |dir: &Path| -> u64 {
        let out = Command::new(env!("CARGO_BIN_EXE_urzua"))
            .args(["check"])
            .current_dir(dir)
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        report["files_examined"].as_u64().unwrap()
    };
    let before = examined(&dir);
    git(&dir, &["add", "--", &record]);
    assert_eq!(examined(&dir), before + 1);
}

#[test]
fn commands_carry_no_schema_knowledge_of_their_own() {
    let (dir, record, _) = scaffold("adr.md", "Throwaway vocabulary record");
    let scaffold_text = std::fs::read_to_string(dir.join(&record)).unwrap();
    // Every header key urzua new wrote, for the three types, plus the status vocabulary
    // that AGENTS.md reserves: none may be restated in a command or the shared script.
    let mut forbidden: Vec<String> = [
        "Accepted",
        "Rejected",
        "Proposed",
        "Planned",
        "Done",
        "Fixed",
        "Draft",
        "Superseded",
    ]
    .map(String::from)
    .to_vec();
    for t in ["adr", "rfc", "milestone"] {
        let d = fixture(&format!("vocab-{t}"));
        let out = run_step(
            &d,
            &format!("scripts/draft-record.sh new {t} Vocab"),
            "",
            "",
        );
        let p = d.join(json_path(&out.stdout));
        for line in std::fs::read_to_string(p).unwrap().lines().skip(1) {
            if line == "---" {
                break;
            }
            forbidden.push(line.split(':').next().unwrap().to_string());
        }
    }
    assert!(scaffold_text.contains("Status:"));
    for file in ["rfc.md", "adr.md", "milestone.md"] {
        let text =
            std::fs::read_to_string(repo_root().join(".claude/commands").join(file)).unwrap();
        for word in &forbidden {
            assert!(!text.contains(word.as_str()), "{file} restates `{word}`");
        }
    }
    let script = std::fs::read_to_string(repo_root().join("scripts/draft-record.sh")).unwrap();
    for word in ["Status", "Accepted", "Proposed", "Deciders", "Implements"] {
        assert!(!script.contains(word), "draft-record.sh restates `{word}`");
    }
}

/// The section the milestone type requires comes from the template `urzua new` fills, so
/// the drafting command carries it without restating it. A draft that loses the section
/// is refused by the check the command ends in, not accepted for having come from it.
#[test]
fn a_milestone_draft_carries_the_required_section_and_check_refuses_one_without_it() {
    let (dir, record, s) = scaffold("milestone.md", "Throwaway done means record");
    let path = dir.join(&record);
    let scaffolded = std::fs::read_to_string(&path).unwrap();
    assert!(
        scaffolded.lines().any(|l| l == "## Done means"),
        "the scaffold must carry the section the milestone type requires:\n{scaffolded}"
    );

    fill(&path);
    let kept = std::fs::read_to_string(&path).unwrap();
    assert!(kept.lines().any(|l| l == "## Done means"), "{kept}");
    let without: String = kept.replace(
        "## Done means\n\nA concrete sentence drafted from the conversation.\n\n",
        "",
    );
    assert_ne!(kept, without, "the fixture must have removed the section");
    std::fs::write(&path, without).unwrap();

    let out = run_step(&dir, &s[1], "", &record);
    assert!(
        !out.status.success(),
        "a milestone without the required section must not finish clean: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("section.required"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}
