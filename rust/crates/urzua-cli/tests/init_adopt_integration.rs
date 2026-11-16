//! `urzua init` on a repository shaped like a real one: changesets, a docs
//! site, source and CI directories beside the records. The config it writes
//! must load, and it must not propose a type for every directory of markdown.

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {args:?} failed");
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    common::urzua()
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn json(out: &std::process::Output) -> serde_json::Value {
    let stdout = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON: {e}\n{stdout}"))
}

/// A repository with the directories a real project has beside its records.
fn realistic_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-cli-test-{name}"));
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);

    for n in ["0001-first", "0002-second", "0003-third"] {
        write(
            &dir,
            &format!("docs/adr/{n}.md"),
            "---\nStatus: Accepted\n---\n# Title\n",
        );
    }
    for n in ["0001-one", "0002-two"] {
        write(
            &dir,
            &format!("docs/rfc/{n}.md"),
            "---\nStatus: Draft\n---\n# Title\n",
        );
    }
    // Specs named by slug: headers with a status, no record number.
    for n in ["alpha", "beta"] {
        write(
            &dir,
            &format!("docs/specs/{n}.md"),
            "---\nStatus: Draft\nOwner: someone\n---\n# Title\n",
        );
    }
    // Changesets: hidden directory, front matter on every file.
    write(&dir, ".changeset/config.json", "{}\n");
    for n in ["brave-lions", "quiet-owls", "tidy-foxes"] {
        write(
            &dir,
            &format!(".changeset/{n}.md"),
            "---\n\"pkg\": patch\n---\n\nFix a thing.\n",
        );
    }
    // A docs site: many directories of pages with front matter and no status.
    for section in ["guides", "reference", "blog", "skills", "examples"] {
        for n in ["intro", "setup", "usage"] {
            write(
                &dir,
                &format!("docs/site/{section}/{n}.md"),
                "---\ntitle: A page\ndescription: Words\n---\n# Page\n",
            );
        }
    }
    // Numbered pages with no header at all.
    for n in ["0001-intro", "0002-setup"] {
        write(&dir, &format!("docs/pages/{n}.md"), "# Page\n\nProse.\n");
    }
    write(&dir, "src/lib.rs", "pub fn f() {}\n");
    write(&dir, "src/notes.md", "# notes\n");
    write(&dir, ".github/workflows/ci.yml", "name: ci\non: push\n");
    write(
        &dir,
        ".github/ISSUE_TEMPLATE/bug.md",
        "---\nname: Bug\nabout: x\n---\n",
    );
    write(
        &dir,
        ".github/ISSUE_TEMPLATE/feature.md",
        "---\nname: Feature\nabout: y\n---\n",
    );
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// N1: init wrote a record type named `.changeset` with an illegal prefix, and
/// proposed a type for every directory of pages.
#[test]
fn init_on_a_realistic_repo_writes_a_config_that_loads_with_a_sensible_type_count() {
    let dir = realistic_repo("init-realistic");

    let out = run_urzua(&dir, &["init"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let report = json(&out);
    let mut dirs: Vec<String> = report["proposed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["dir"].as_str().unwrap().to_string())
        .collect();
    dirs.sort();
    assert_eq!(dirs, ["docs/adr", "docs/rfc", "docs/specs"], "{report}");

    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    assert!(!config.contains(".changeset"), "{config}");

    let check = run_urzua(&dir, &["check"]);
    let checked = json(&check);
    assert_ne!(
        checked["status"], "not-run",
        "the config init wrote must load: {checked}"
    );
    assert!(
        matches!(check.status.code(), Some(0) | Some(1)),
        "{checked}"
    );

    let doctor = run_urzua(&dir, &["doctor"]);
    let doctor = String::from_utf8_lossy(&doctor.stdout);
    assert!(
        !doctor.contains("\"config-parses\"") || !doctor.contains("error"),
        "{doctor}"
    );
}

/// N1: what is left out is named, not dropped silently.
#[test]
fn init_names_the_directories_it_did_not_propose() {
    let dir = realistic_repo("init-names-skipped");
    let report = json(&run_urzua(&dir, &["init", "--dry-run"]));
    let notices = report["notices"].to_string();
    assert!(
        notices.contains("docs/pages"),
        "headerless pages named: {notices}"
    );
    assert!(
        notices.contains("docs/site/guides"),
        "status-less pages named: {notices}"
    );
    assert!(
        !notices.contains(".changeset"),
        "hidden directories are not even mentioned: {notices}"
    );
}

/// The config a `--dry-run` proposes is byte-identical to the file the real run
/// writes; the reports differ only in `dry_run`, `written` and the echoed content.
#[test]
fn init_dry_run_proposes_the_config_the_real_run_writes() {
    let dir = realistic_repo("init-dry-run-matches-write");
    let dry = json(&run_urzua(&dir, &["init", "--dry-run"]));
    assert_eq!(dry["dry_run"], true);
    assert!(!dir.join(".urzua/config.yaml").exists());
    let proposed = dry["config_yaml"].as_str().unwrap().to_string();

    let real = json(&run_urzua(&dir, &["init"]));
    assert_eq!(real["dry_run"], false);
    assert_eq!(real["written"], true);
    assert_eq!(
        std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap(),
        proposed
    );
    assert_eq!(dry["proposed"], real["proposed"]);
}

/// N1: `--dry-run` shows the proposal even when a config already exists, and
/// still writes nothing.
#[test]
fn init_dry_run_works_when_a_config_already_exists() {
    let dir = realistic_repo("init-dry-run-existing");
    assert_eq!(run_urzua(&dir, &["init"]).status.code(), Some(0));
    let before = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();

    let out = run_urzua(&dir, &["init", "--dry-run"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let report = json(&out);
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["written"], false);
    assert!(report["config_yaml"]
        .as_str()
        .unwrap()
        .contains("record_types"));
    assert_eq!(
        std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap(),
        before
    );

    // Without --dry-run an existing config is still never overwritten.
    assert_eq!(run_urzua(&dir, &["init"]).status.code(), Some(2));
}

/// N1: a directory name that cannot be a prefix still yields a config that loads.
#[test]
fn init_proposes_loadable_names_for_awkward_directory_names() {
    let dir = common::TempDir::new("urzua-cli-test-init-awkward");
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    for d in ["2024", "my_notes", "v1.2"] {
        write(
            &dir,
            &format!("docs/{d}/0001-a.md"),
            "---\nStatus: Draft\n---\n# A\n",
        );
    }
    // Two directories whose records share a filename prefix.
    write(&dir, "docs/adr/0001-a.md", "---\nStatus: Draft\n---\n# A\n");
    write(
        &dir,
        "docs/adr/old/0002-b.md",
        "---\nStatus: Draft\n---\n# B\n",
    );
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);

    let out = run_urzua(&dir, &["init"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let checked = json(&run_urzua(&dir, &["check"]));
    assert_ne!(checked["status"], "not-run", "{checked}");
}

/// SPEC-5: `init` takes the global `--config`, and writes there.
#[test]
fn init_writes_the_file_named_by_config() {
    let dir = realistic_repo("init-config");
    let target = dir.join("conf/alt.yaml");
    let out = run_urzua(&dir, &["--config", target.to_str().unwrap(), "init"]);
    let parsed = json(&out);
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    assert_eq!(parsed["written"], true, "{parsed}");
    assert!(target.exists(), "{parsed}");
    assert!(!dir.join(".urzua/config.yaml").exists(), "{parsed}");

    // The named file is the one checked for "already exists".
    let again = run_urzua(&dir, &["--config", target.to_str().unwrap(), "init"]);
    assert_eq!(again.status.code(), Some(2));
}

/// A repository with prefixed records, a README beside them and one record
/// still written in the removed blockquote form.
fn readme_and_old_form_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-cli-test-{name}"));
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    for n in ["ADR-1-first", "ADR-2-second"] {
        write(
            &dir,
            &format!("docs/adr/{n}.md"),
            "---\nStatus: Accepted\n---\n# Title\n",
        );
    }
    write(
        &dir,
        "docs/adr/ADR-3-old.md",
        "# ADR-3\n\n> **Status:** Accepted\n",
    );
    write(&dir, "docs/adr/README.md", "# Decisions\n");
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

#[test]
fn init_excludes_a_readme_in_a_record_directory_observed_failing() {
    let dir = readme_and_old_form_repo("init-readme");
    let report = json(&run_urzua(&dir, &["init"]));
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    assert!(config.contains("exclude_files"), "{config}");
    assert!(config.contains("README.md"), "{config}");
    assert_eq!(report["proposed"][0]["exclude_files"][0], "README.md");

    let checked = json(&run_urzua(&dir, &["check"]));
    assert!(
        !checked.to_string().contains("README.md"),
        "check says nothing about the README: {checked}"
    );
}

#[test]
fn init_without_a_readme_writes_no_exclude_files() {
    let dir = realistic_repo("init-no-readme");
    run_urzua(&dir, &["init"]);
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    assert!(!config.contains("exclude_files"), "{config}");
}

/// Adopter S3: the header warning must state what `check` does. With the
/// `required_fields: []` init writes, `check` is silent about an old-form
/// header, so the warning says so and names the config that makes it report: the declaration, and the rule that reads it
/// (`header.required-fields` is withheld until `required_fields` exists, `BUG-73`).
#[test]
fn the_old_form_warning_says_check_is_silent_until_required_fields_is_declared() {
    let dir = readme_and_old_form_repo("init-old-form-warning");
    let report = json(&run_urzua(&dir, &["init", "--dry-run"]));
    let notices = report["notices"].to_string();
    assert!(
        !notices.contains("will be reported as unreadable"),
        "the promise check does not keep is gone: {notices}"
    );
    assert!(notices.contains("required_fields"), "{notices}");

    // The sentence is true: silent as written, loud once declared.
    run_urzua(&dir, &["init"]);
    let silent = json(&run_urzua(&dir, &["check"]));
    assert!(
        !silent.to_string().contains("ADR-3-old"),
        "check is silent about the old form as init writes the config: {silent}"
    );
    let path = dir.join(".urzua/config.yaml");
    let config = std::fs::read_to_string(&path)
        .unwrap()
        .replace("required_fields: []", "required_fields: [Status]")
        .replace("rules:\n", "rules:\n  header.required-fields: warn\n");
    std::fs::write(&path, config).unwrap();
    let loud = json(&run_urzua(&dir, &["check"]));
    assert!(
        loud.to_string().contains("ADR-3-old"),
        "declaring required_fields makes check report it: {loud}"
    );
}

/// MILE-114: greenfield mode must not change adopt. On a repository that
/// already has records, plain `init` writes exactly the config recorded in
/// `fixtures/init_adopt_realistic.config.yaml` (generated by the build before
/// greenfield mode existed) and nothing else.
#[test]
fn adopt_config_is_byte_identical_to_the_pre_greenfield_output() {
    let dir = realistic_repo("init-adopt-golden");
    let out = run_urzua(&dir, &["init"]);
    assert_eq!(out.status.code(), Some(0));
    let golden_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/init_adopt_realistic.config.yaml");
    let written = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    if std::env::var_os("UPDATE_ADOPT_GOLDEN").is_some() {
        std::fs::create_dir_all(golden_path.parent().unwrap()).unwrap();
        std::fs::write(&golden_path, &written).unwrap();
    }
    assert_eq!(written, std::fs::read_to_string(&golden_path).unwrap());
    assert!(
        !dir.join(".urzua/templates").exists(),
        "adopt writes no starter templates"
    );
}

/// `MILE-104`: `section.duplicate`, `section.level-skipped` and `section.order`
/// each read a `sections.*` key adopt never writes, so proposing them would
/// have them report `ran` over zero records (`BUG-73`). Adopt withholds them,
/// names each with the key it needs, and `check` shows them as not enabled.
#[test]
fn adopt_withholds_the_section_structure_rules_and_names_the_key_each_needs() {
    let dir = realistic_repo("init-adopt-section-structure");
    let out = run_urzua(&dir, &["init"]);
    assert_eq!(out.status.code(), Some(0));
    let report = json(&out);
    let written = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    let notice = report["notices"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["message"].as_str())
        .find(|m| m.contains("not proposed") && m.contains("field.quality"))
        .expect("the withheld-rule notice")
        .to_string();
    for (rule, key) in [
        ("section.duplicate", "sections.unique_headings"),
        ("section.level-skipped", "sections.no_skipped_levels"),
        ("section.order", "sections.order"),
        ("section.required", "sections.required"),
        ("section.shape", "sections.required"),
    ] {
        assert!(!written.contains(rule), "{rule} is not written: {written}");
        assert!(notice.contains(rule) && notice.contains(key), "{notice}");
    }
    let check = json(&run_urzua(&dir, &["check"]));
    for e in check["rules_executed"].as_array().unwrap() {
        if matches!(
            e["rule"].as_str(),
            Some(
                "section.duplicate"
                    | "section.level-skipped"
                    | "section.order"
                    | "section.required"
                    | "section.shape"
            )
        ) {
            assert_eq!(e["status"], "not-enabled", "{e}");
        }
    }
}

/// A waiver type suppresses findings, so `init` does not turn it on for a
/// corpus that has a directory of waivers: it proposes `harness.outside_writes`
/// (which enables nothing alone) and withholds the waiver declaration, naming
/// the keys that make `waive --from-report` work (the adopter switchover, items
/// 3 and 13).
#[test]
fn init_withholds_the_waiver_declaration_and_names_its_keys() {
    let dir = common::TempDir::new("urzua-cli-test-waiver-withheld");
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    write(
        &dir,
        "docs/adr/ADR-0001-a.md",
        "---\nStatus: Accepted\n---\n# A\n",
    );
    write(
        &dir,
        "docs/waiver/WAIVER-0001-a.md",
        "---\nRule: x.y\nScope: a\nReason: r\n---\n# W\n",
    );
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    let out = run_urzua(&dir, &["init", "--dry-run"]);
    let report = json(&out);
    let yaml = report["config_yaml"].as_str().unwrap();
    assert!(yaml.contains("outside_writes: shared"), "{yaml}");
    assert!(
        !yaml.contains("role:"),
        "no waiver role is inferred: {yaml}"
    );
    assert!(!yaml.contains("waiver.matches-nothing"), "{yaml}");
    let notices = report["notices"].to_string();
    assert!(notices.contains("role: suppression"), "{notices}");
    assert!(
        notices.contains("effective_requires: [Reason, Expires]"),
        "{notices}"
    );
}
