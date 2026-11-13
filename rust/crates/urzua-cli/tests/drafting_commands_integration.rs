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
//! repository's records, configuration and commands (not its Rust sources or changesets,
//! which no assertion here reads), so the real `.urzua/config.yaml` is what runs.

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

/// Directories the fixture leaves out: the Rust workspace and the changesets are about
/// 900 files that every `check` in the fixture would read, and no test here looks at them.
const NOT_COPIED: [&str; 2] = ["rust/", ".changeset/"];

/// What stands in for the Rust workspace: the configuration names `rust` and two test
/// directories as back-pointer paths, and an entry that covers no tracked file is a finding
/// in its own right, so each of them keeps one (comment-only) file.
const STAND_INS: [&str; 2] = [
    "rust/crates/urzua-cli/tests/stand_in.rs",
    "rust/crates/urzua-core/tests/stand_in.rs",
];

/// A throwaway git repo holding a copy of this repository's working tree (tracked and
/// not-yet-tracked files, minus ignored ones and `NOT_COPIED`), committed as the baseline.
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
        if rel.is_empty() || NOT_COPIED.iter().any(|d| rel.starts_with(d)) || !src.is_file() {
            continue;
        }
        let dst = dir.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(&src, &dst).unwrap();
    }
    for rel in STAND_INS {
        let dst = dir.join(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::write(
            dst,
            "// Stands in for the Rust workspace in this fixture.\n",
        )
        .unwrap();
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
    run_step_with_bin(dir, script, title, record, env!("CARGO_BIN_EXE_urzua"))
}

/// A `urzua` that pins `check` to `OWN_CORPUS_AS_OF` and passes every other command
/// through. The finish step runs the hook, which checks the record with no way to pass
/// `--as-of`, and the fixture borrows this repository's configuration, whose waivers
/// lapse on a known date; the real clock would turn the test red then with no code change.
fn pinned_bin(dir: &Path) -> String {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(".urzua-pinned-bin.sh");
    let script = format!(
        "#!/usr/bin/env bash\nif [ \"$1\" = check ]; then shift; exec \"{bin}\" --as-of {as_of} check \"$@\"; fi\nexec \"{bin}\" \"$@\"\n",
        bin = env!("CARGO_BIN_EXE_urzua"),
        as_of = common::OWN_CORPUS_AS_OF
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_string_lossy().into_owned()
}

fn run_step_with_bin(dir: &Path, script: &str, title: &str, record: &str, bin: &str) -> Output {
    Command::new("bash")
        .args(["-ec", script])
        .env("TITLE", title)
        .env("RECORD", record)
        .env("URZUA", bin)
        .env("PATH", common::path_with_stub_gh())
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
            // A Done means section is a list of items in this repository's config
            // (ADR-96); every other section is prose.
            if line == "## Done means" {
                out.push_str("\n\n- A concrete criterion drafted from the conversation.\n\n");
            } else {
                out.push_str("\n\nA concrete sentence drafted from the conversation.\n\n");
            }
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

        let pinned = pinned_bin(&dir);
        let out = run_step_with_bin(&dir, &s[1], "", &record, &pinned);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "{command}: {stderr}\n{}",
            String::from_utf8_lossy(&out.stdout)
        );

        // The check examined the record (not a clean run over nothing) and was clean.
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["files_examined"], 1, "{command}: {report}");
        // A finding the repository has waived (a configuration finding is kept by every
        // scope) is reported but does not make the run unclean.
        let unwaived = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["waived"].is_null())
            .count();
        assert_eq!(unwaived, 0, "{command}: {report}");
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

fn sweep_examined(dir: &Path) -> u64 {
    let out = common::urzua()
        .args(["check"])
        .current_dir(dir)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    report["files_examined"].as_u64().unwrap()
}

/// The reason the commands stage before checking (ADR-56): where a repository
/// governs tracked files only, a sweep does not see a fresh scaffold until it is
/// staged. Switching this repository's `discovery.include_untracked` off is that
/// repository (ADR-92).
#[test]
fn without_include_untracked_a_sweep_examines_a_scaffold_only_once_it_is_staged() {
    let (dir, record, _) = scaffold("rfc.md", "Throwaway unstaged record");
    let config = dir.join(".urzua/config.yaml");
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(
        text.contains("include_untracked: true"),
        "this repository sets it"
    );
    std::fs::write(
        &config,
        text.replace("include_untracked: true", "include_untracked: false"),
    )
    .unwrap();
    let before = sweep_examined(&dir);
    git(&dir, &["add", "--", &record]);
    assert_eq!(sweep_examined(&dir), before + 1);
}

/// MILE-206, ADR-92: this repository includes untracked records, so a sweep
/// examines a fresh scaffold before it is staged, and staging it changes nothing.
#[test]
fn with_include_untracked_a_sweep_examines_a_scaffold_before_it_is_staged() {
    let (dir, record, _) = scaffold("rfc.md", "Throwaway unstaged record");
    let included = sweep_examined(&dir);
    // The same tree with the option off does not see the unstaged scaffold.
    let config = dir.join(".urzua/config.yaml");
    let text = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        text.replace("include_untracked: true", "include_untracked: false"),
    )
    .unwrap();
    assert_eq!(sweep_examined(&dir) + 1, included);
    std::fs::write(&config, text).unwrap();
    git(&dir, &["add", "--", &record]);
    assert_eq!(sweep_examined(&dir), included);
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
            &format!("bash .claude/hooks/urzua-record.sh new {t} Vocab"),
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
    let script =
        std::fs::read_to_string(repo_root().join(".claude/hooks/urzua-record.sh")).unwrap();
    for word in ["Status", "Accepted", "Proposed", "Deciders", "Implements"] {
        assert!(!script.contains(word), "urzua-record.sh restates `{word}`");
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
        "## Done means\n\n- A concrete criterion drafted from the conversation.\n\n",
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

/// MILE-204: this repository runs on the installed bundle. Every drafting command in
/// `.claude/commands/` is the one `urzua harness install` generates for its record type
/// (so `harness status` reports no drift and the lock names it), and the hand-wired
/// hook scripts the bundle replaced are gone.
#[test]
fn this_repository_runs_on_the_installed_bundle_with_no_drift_and_no_hook_scripts() {
    let root = repo_root();
    let out = common::urzua()
        .args(["harness", "status"])
        .current_dir(&root)
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(out.status.success(), "harness status drifted: {report}");
    assert_eq!(report["status"], "ok", "{report}");
    let lock = std::fs::read_to_string(root.join(".urzua/harness.lock")).unwrap();
    for ty in ["adr", "rfc", "milestone"] {
        assert!(
            lock.contains(&format!(".claude/commands/{ty}.md")),
            "{ty}: the drafting command is generated, so the lock names it"
        );
    }
    assert!(!root.join("scripts/hooks").exists());
    assert!(!root.join("scripts/draft-record.sh").exists());
}

/// MILE-200: the files that enforce the gate are protected paths in this repository's own
/// configuration. An agent's edit to the gate action, its scripts or the release
/// configuration is denied; a person's is not.
#[test]
fn an_agent_edit_to_the_files_that_enforce_the_gate_is_denied_in_this_repository() {
    use std::io::Write;
    use std::process::Stdio;
    let root = repo_root();
    let edit = |path: &str, actor: &str| -> (i32, serde_json::Value) {
        let request = serde_json::json!({
            "event": "edit", "tool": "Write", "paths": [path], "content": "x\n",
            "actor": actor, "cwd": root.to_string_lossy(),
        });
        let mut child = common::urzua()
            .args(["harness", "run", "edit"])
            .env_remove("URZUA_HOOK_OVERRIDE")
            .env_remove("URZUA_HOOK_REASON")
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(request.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        (
            out.status.code().unwrap(),
            serde_json::from_slice(&out.stdout).unwrap(),
        )
    };
    for path in [
        ".github/actions/urzua-gate/action.yml",
        ".github/actions/urzua-gate/actors.sh",
        ".github/scripts/release-invariants.sh",
        "knope.toml",
    ] {
        let (code, v) = edit(path, "agent");
        assert_eq!(code, 1, "{path}: {v}");
        assert_eq!(v["verdict"], "block", "{path}: {v}");
        let (code, v) = edit(path, "human");
        assert_eq!(code, 0, "{path}: {v}");
    }
}

/// A freshly scaffolded milestone reports no shape error on the section the template
/// wrote: the template scaffolds the list-item shape this repository's
/// `statuses.delivery.items` accepts. The two placeholder items are still unfilled, and
/// the type's declared `delivery.items.pending` vocabulary reports exactly those two
/// (and nothing else) until a real criterion replaces them.
#[test]
fn a_fresh_milestone_scaffold_reports_only_its_two_placeholder_items() {
    let (dir, record, _) = scaffold("milestone.md", "Throwaway shape probe record");
    let out = common::urzua()
        .args(["check", "--rules", "status.done-items-shape"])
        .arg(&record)
        .current_dir(dir.path())
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["files_examined"], 1, "{report}");
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2, "{report}");
    for finding in findings {
        let message = finding["message"].as_str().unwrap();
        assert!(
            message.contains("is an unfilled placeholder"),
            "only placeholder findings are expected on a scaffold: {report}"
        );
    }
}
