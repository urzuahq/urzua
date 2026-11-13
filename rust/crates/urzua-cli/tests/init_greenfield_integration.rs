//! `urzua init --types` (SPEC-5, MILE-114): greenfield mode, built-in profiles
//! written out as visible config, starter templates, and the report of record-
//! shaped files nothing governs.
//!
//! Verifies: SPEC-5

mod common;

use std::io::Write;
use std::path::Path;

fn repo(label: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-greenfield-{label}"));
    common::git(&dir, &["init", "-q"]);
    common::git(&dir, &["config", "user.email", "test@example.com"]);
    common::git(&dir, &["config", "user.name", "test"]);
    write(dir.path(), "README.md", "# project\n");
    common::git(&dir, &["add", "-A"]);
    common::git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn run(dir: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value, String) {
    let out = common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let value = serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON: {e}\n{stdout}"));
    (out.status.code(), value, stdout)
}

fn config(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(".urzua/config.yaml")).expect("config written")
}

#[test]
fn types_writes_a_config_and_templates_for_each_selected_profile() {
    let dir = repo("write");
    let (code, report, _) = run(dir.path(), &["init", "--types", "adr,rfc,spec,prd"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["status"], "ok");
    assert_eq!(report["written"], true);
    let names: Vec<&str> = report["proposed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["adr", "rfc", "spec", "prd"]);
    assert_eq!(report["proposed"][0]["dir"], "docs/adr");
    assert_eq!(report["proposed"][0]["prefix"], "ADR");
    assert_eq!(report["templates"].as_array().unwrap().len(), 4);
    for t in ["adr", "rfc", "spec", "prd"] {
        assert!(dir
            .path()
            .join(format!(".urzua/templates/{t}.md"))
            .is_file());
    }
}

/// The profile is written out as visible lines, never referenced by name.
#[test]
fn the_profiles_are_materialized_as_visible_rules_in_the_config() {
    let dir = repo("visible");
    run(dir.path(), &["init", "--types", "adr,rfc,spec,prd"]);
    let parsed = urzua_core::config::parse(&config(dir.path())).expect("config loads");
    for (name, field) in [
        ("adr", "Deciders"),
        ("rfc", "Author"),
        ("spec", "Subject"),
        ("prd", "Owner"),
    ] {
        let ty = parsed.record_types.get(name).expect(name);
        assert!(ty.required_fields.iter().any(|f| f == field), "{name}");
        assert!(
            ty.required_fields.iter().any(|f| f == "Stable-Id"),
            "{name}"
        );
        let statuses = ty.status_decl().expect("statuses written out");
        assert!(!statuses.values.is_empty(), "{name}");
    }
    let text = config(dir.path());
    assert!(!text.contains("profile:"), "no by-name reference: {text}");
    // The lifecycle is written out as visible config.
    let value: yaml_serde::Value = yaml_serde::from_str(&text).unwrap();
    let transitions = |t: &str| value["record_types"][t]["statuses"]["transitions"].clone();
    let list = |v: &yaml_serde::Value, from: &str| -> Vec<String> {
        v[from]
            .as_sequence()
            .map(|s| s.iter().map(|x| x.as_str().unwrap().to_string()).collect())
            .unwrap_or_default()
    };
    let adr = transitions("adr");
    assert_eq!(list(&adr, "Proposed"), ["Accepted", "Rejected"]);
    assert_eq!(list(&adr, "Accepted"), ["Superseded"]);
    let prd = transitions("prd");
    assert_eq!(list(&prd, "Draft"), ["Review"]);
    assert_eq!(list(&prd, "Review"), ["Approved", "Rejected"]);
    assert_eq!(list(&prd, "Approved"), ["Superseded"]);
    assert_eq!(
        list(&transitions("rfc"), "Discussion"),
        ["Accepted", "Rejected"]
    );
    assert_eq!(list(&transitions("spec"), "Accepted"), ["Shipped"]);
}

/// MILE-114 Done: the config greenfield init writes passes `check` (over the
/// records `new` scaffolds from its profiles; `check` over an empty corpus is
/// `not-run`), and every shipped type's `new` writes the Stable-Id it requires.
#[test]
fn the_written_config_passes_check_and_new_writes_a_stable_id_for_every_type() {
    let dir = repo("check");
    run(dir.path(), &["init", "--types", "adr,rfc,spec,prd"]);
    for t in ["adr", "rfc", "spec", "prd"] {
        let new = common::urzua()
            .args(["new", t, "First thing", "--by", "tester"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert_eq!(
            new.status.code(),
            Some(0),
            "{t}: {}",
            String::from_utf8_lossy(&new.stdout)
        );
        let text = std::fs::read_dir(dir.path().join(format!("docs/{t}")))
            .unwrap()
            .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap())
            .next()
            .unwrap();
        assert!(text.contains("Stable-Id: "), "{t}: {text}");
    }
    common::git(&dir, &["add", "-A"]);
    let check = common::urzua()
        .arg("check")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(
        check.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&check.stdout)
    );
}

#[test]
fn dir_sets_the_layout_root() {
    let dir = repo("dir");
    let (code, report, _) = run(dir.path(), &["init", "--types", "adr", "--dir", "handbook"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["proposed"][0]["dir"], "handbook/adr");
    assert!(config(dir.path()).contains("handbook/adr"));
}

#[test]
fn an_unknown_type_is_refused_by_name_and_nothing_is_written() {
    let dir = repo("unknown");
    let (code, report, _) = run(dir.path(), &["init", "--types", "adr,widget"]);
    assert_eq!(code, Some(2));
    let msg = report["error"].as_str().unwrap();
    assert!(msg.contains("widget") && msg.contains("prd"), "{msg}");
    assert!(!dir.path().join(".urzua").exists());
}

#[test]
fn a_type_listed_twice_is_refused() {
    let dir = repo("dup");
    let (code, report, _) = run(dir.path(), &["init", "--types", "adr,adr"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(!dir.path().join(".urzua").exists());
}

#[test]
fn a_dir_that_leaves_the_repository_is_refused() {
    let dir = repo("escape");
    for bad in ["../x", "/abs", ".hidden"] {
        let (code, _, _) = run(dir.path(), &["init", "--types", "adr", "--dir", bad]);
        assert_eq!(code, Some(2), "{bad}");
    }
    assert!(!dir.path().join(".urzua").exists());
}

#[test]
fn dry_run_matches_the_real_run_byte_for_byte_and_writes_nothing() {
    let dir = repo("dry");
    let (code, dry, _) = run(dir.path(), &["init", "--types", "adr,rfc", "--dry-run"]);
    assert_eq!(code, Some(0));
    assert_eq!(dry["written"], false);
    assert!(!dir.path().join(".urzua").exists());
    run(dir.path(), &["init", "--types", "adr,rfc"]);
    assert_eq!(dry["config_yaml"].as_str().unwrap(), config(dir.path()));
}

/// SPEC-5 success criterion: a re-run that would write the same config is a
/// no-op exiting 0; one that would write a different config is refused.
#[test]
fn a_rerun_with_the_same_selection_is_a_no_op_and_a_different_one_is_refused() {
    let dir = repo("rerun");
    run(dir.path(), &["init", "--types", "adr,rfc"]);
    let before = config(dir.path());
    write(dir.path(), ".urzua/templates/adr.md", "# mine\n");
    let (code, report, _) = run(dir.path(), &["init", "--types", "adr,rfc"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["written"], false);
    assert_eq!(config(dir.path()), before);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".urzua/templates/adr.md")).unwrap(),
        "# mine\n",
        "a hand-edited template is never overwritten"
    );
    let (code, _, _) = run(dir.path(), &["init", "--types", "adr,rfc,spec"]);
    assert_eq!(code, Some(2));
}

/// SPEC-5 success criterion: unclassified files are reported, not errored.
#[test]
fn record_shaped_files_outside_the_selected_types_are_reported_not_errored() {
    let dir = repo("unclassified");
    write(
        dir.path(),
        "notes/0001-x.md",
        "---\nStatus: Draft\n---\n# x\n",
    );
    write(dir.path(), "0002-y.md", "---\nStatus: Draft\n---\n# y\n");
    write(
        dir.path(),
        "docs/adr/ADR-1-z.md",
        "---\nStatus: Proposed\n---\n# z\n",
    );
    common::git(&dir, &["add", "-A"]);
    let (code, report, _) = run(dir.path(), &["init", "--types", "adr"]);
    assert_eq!(code, Some(0), "{report}");
    let notices = report["notices"].to_string();
    assert!(notices.contains("notes/0001-x.md"), "{notices}");
    assert!(notices.contains("0002-y.md"), "{notices}");
    assert!(!notices.contains("ADR-1-z.md"), "{notices}");
    assert_eq!(report["proposed"][0]["record_count"], 1);
}

#[test]
fn greenfield_touches_only_the_urzua_directory() {
    let dir = repo("touched");
    run(dir.path(), &["init", "--types", "adr,rfc"]);
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let status = String::from_utf8_lossy(&out.stdout).to_string();
    for line in status.lines() {
        assert!(line.starts_with("?? .urzua/"), "{status}");
    }
}

#[test]
fn dir_without_types_is_refused_instead_of_ignored() {
    let dir = repo("dir-alone");
    write(
        dir.path(),
        "docs/adr/ADR-1-x.md",
        "---\nStatus: Accepted\n---\n# x\n",
    );
    common::git(&dir, &["add", "-A"]);
    let (code, report, _) = run(dir.path(), &["init", "--dir", "docs"]);
    assert_eq!(code, Some(2), "{report}");
    assert!(!dir.path().join(".urzua").exists());
}

/// Without `--types` and without a terminal, an empty repository gets the same
/// refusal adopt always gave: no prompt is attempted.
#[test]
fn with_no_types_and_no_terminal_an_empty_repository_is_refused_not_prompted() {
    let dir = repo("noprompt");
    let (code, report, _) = run(dir.path(), &["init"]);
    assert_eq!(code, Some(2));
    assert!(report["error"]
        .as_str()
        .unwrap()
        .contains("nothing to adopt"));
}

/// MILE-114: adopt reports a record-shaped file it cannot classify by path
/// (one at the repository root has no directory to name a type) as a notice.
#[test]
fn adopt_reports_a_root_level_record_it_could_not_classify() {
    let dir = repo("adopt-root");
    write(
        dir.path(),
        "docs/adr/ADR-1-x.md",
        "---\nStatus: Accepted\n---\n# x\n",
    );
    write(
        dir.path(),
        "0007-stray.md",
        "---\nStatus: Draft\n---\n# s\n",
    );
    common::git(&dir, &["add", "-A"]);
    let (code, report, _) = run(dir.path(), &["init"]);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["written"], true);
    assert!(report["notices"].to_string().contains("0007-stray.md"));
}

/// MILE-114: on a terminal, `init` in a repository with no record directory
/// shows a menu and reads the selection. `script` gives the binary a pseudo
/// terminal; its transcript holds the menu (standard error) and the JSON report.
#[test]
fn on_a_terminal_init_asks_which_types_to_govern() {
    let dir = repo("menu");
    let binary = common::urzua().get_program().to_os_string();
    let mut script = std::process::Command::new("script");
    script
        .args([
            "-qec",
            &(binary.to_string_lossy().to_string() + " init"),
            "/dev/null",
        ])
        .env("PATH", common::path_with_stub_gh())
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut child = script
        .spawn()
        .expect("`script` (util-linux) is needed for this test");
    // The temporary handle drops at the end of the statement, closing the input.
    child
        .stdin
        .take()
        .unwrap()
        .write_all("adr, 3\n".as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let transcript = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        transcript.contains("Choose the record types"),
        "{transcript}"
    );
    // The report is the transcript from its opening brace (0x7b) onward.
    let open = char::from(0x7b_u8);
    let json = &transcript[transcript.find(open).expect("a JSON report")..];
    let report: serde_json::Value = serde_json::from_str(json.trim()).unwrap();
    let names: Vec<&str> = report["proposed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["adr", "spec"], "{transcript}");
    assert!(dir.path().join(".urzua/config.yaml").is_file());
}
