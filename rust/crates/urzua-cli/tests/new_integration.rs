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
        dir.join(".urzua/config.toml"),
        "schema_version = 1\n\n[record_types.adr]\ndir = \"docs/adr\"\nrequired_fields = [\"Status\"]\nheader_shape = \"yaml-frontmatter\"\n",
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
        dir.join(".urzua/config.toml"),
        "schema_version = 1\n\n[record_types.adr]\ndir = \"docs/adr\"\nrequired_fields = [\"Status\"]\n",
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
