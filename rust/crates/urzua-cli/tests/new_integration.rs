//! CLI-golden integration tests for `urzua new`: build a real, isolated
//! git-repo fixture and run the actual compiled binary against it.

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

fn fixture_repo(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("urzua-cli-new-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "test"]);
    dir
}

fn commit_all(dir: &Path) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "fixture"]);
}

fn run_urzua(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_urzua"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("urzua binary should run")
}

/// BUG-3: a type configured as `yaml-frontmatter` must get YAML frontmatter
/// from `urzua new`, even when a blockquote-shaped template file exists for
/// it -- the configured shape wins unconditionally, never the template's own
/// (possibly stale) shape.
#[test]
fn new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present() {
    let dir = fixture_repo("yaml-with-template");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    // A template that still uses the old blockquote shape -- this is
    // exactly BUG-3's reproduction: config says yaml-frontmatter, but a
    // pre-existing template file disagrees.
    std::fs::write(
        dir.join(".urzua/templates/adr.md"),
        "# NNNN — Title\n\n> Status: Proposed\n\n## Context\n\nFill this in.\n\n## Decision\n\nFill this in.\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["new", "adr", "A real title"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let path: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON on stdout");
    let rel_path = path["path"].as_str().expect("path field");
    let content = std::fs::read_to_string(dir.join(rel_path)).unwrap();

    assert!(
        content.starts_with("---\n"),
        "expected YAML frontmatter, got: {content}"
    );
    assert!(
        !content.contains("> Status:"),
        "blockquote header leaked through despite yaml-frontmatter config: {content}"
    );
    // The template's body scaffolding still comes through underneath the
    // synthesized header -- fixing the header shape shouldn't cost the
    // template's sections.
    assert!(content.contains("## Context"));
    assert!(content.contains("## Decision"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn new_still_uses_the_template_verbatim_for_the_default_blockquote_shape() {
    let dir = fixture_repo("blockquote-default");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".urzua/templates/adr.md"),
        "# NNNN — Title\n\n> Status: Proposed\n\n## Context\n\nFill this in.\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["new", "adr", "A real title"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let path: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON on stdout");
    let rel_path = path["path"].as_str().expect("path field");
    let content = std::fs::read_to_string(dir.join(rel_path)).unwrap();

    assert!(content.contains("> Status: Proposed"));
    assert!(!content.starts_with("---\n"));

    std::fs::remove_dir_all(&dir).ok();
}

const BUG_CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error}\n\nrecord_types:\n  bug:\n    dir: \"docs/bug\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n";

/// MILE-89: a number already used on another local branch must be skipped,
/// with the claiming branch named on stderr.
#[test]
fn new_skips_a_number_claimed_on_another_branch_and_names_it() {
    let dir = fixture_repo("claimed-elsewhere");
    std::fs::create_dir_all(dir.join("docs/bug")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), BUG_CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/bug/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — x\n",
    )
    .unwrap();
    commit_all(&dir);
    git(&dir, &["checkout", "-q", "-b", "other-pr"]);
    std::fs::write(
        dir.join("docs/bug/BUG-2-x.md"),
        "---\nStatus: Open\n---\n# 2 — x\n",
    )
    .unwrap();
    commit_all(&dir);
    git(&dir, &["checkout", "-q", "-"]);
    assert!(!dir.join("docs/bug/BUG-2-x.md").exists());

    let output = run_urzua(&dir, &["new", "bug", "Fresh"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["display_number"], 3, "stderr: {stderr}");
    assert!(stderr.contains("other-pr"), "stderr: {stderr}");

    std::fs::remove_dir_all(&dir).ok();
}

/// When git cannot read refs (here: a `.git` directory that is not a real
/// repository, which `find_repo_root` still accepts), numbering falls back to
/// the working tree and says so rather than silently skipping the check.
#[test]
fn new_falls_back_to_the_working_tree_and_says_so_when_git_cannot_read_refs() {
    let dir = std::env::temp_dir().join(format!("urzua-cli-new-nogit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/bug")).unwrap();
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), BUG_CONFIG).unwrap();
    std::fs::write(
        dir.join("docs/bug/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — x\n",
    )
    .unwrap();

    let output = run_urzua(&dir, &["new", "bug", "Fresh", "--by", "t"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["display_number"], 2);
    assert!(stderr.contains("working tree only"), "stderr: {stderr}");
    std::fs::remove_dir_all(&dir).ok();
}

/// A title that slugifies to nothing (empty, whitespace, or only non-ASCII)
/// used to create `ADR-N-.md`, a name the numbering scan does not recognise,
/// so the next `new` reissued N. It is now refused before anything is
/// written, and the number is not consumed.
#[test]
fn new_refuses_a_title_with_no_slug_and_writes_nothing() {
    let dir = fixture_repo("empty-slug");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    commit_all(&dir);

    for title in ["", "   ", "日本語"] {
        let output = run_urzua(&dir, &["new", "adr", title, "--by", "me"]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(2), "{title:?}: {stdout}");
        assert!(stdout.contains("no ASCII letters or digits"), "{stdout}");
        let written: Vec<_> = std::fs::read_dir(dir.join("docs/adr")).unwrap().collect();
        assert!(written.is_empty(), "{title:?} wrote {written:?}");
    }

    // A good title afterwards still takes number 1.
    let output = run_urzua(&dir, &["new", "adr", "Real title", "--by", "me"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(dir.join("docs/adr/ADR-1-real-title.md").exists());

    std::fs::remove_dir_all(&dir).ok();
}
