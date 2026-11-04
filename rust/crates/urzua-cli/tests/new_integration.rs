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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
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

const PADDED_RULES: &str = "{header.required-fields: error, field.quality: error, filename.identity-pattern: error, filename.title-consistency: error, header.field-set-consistency: error}";

/// A type with a four-digit padded identity pattern, a declared status
/// vocabulary and a closed field set (`known_fields`).
fn padded_repo(name: &str, known_fields: &str, pattern: &str) -> std::path::PathBuf {
    let dir = fixture_repo(name);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {PADDED_RULES}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\", \"Date\", \"Author\"]\n    known_fields: {known_fields}\n    header_shape: \"yaml-frontmatter\"\n    identity: {{from: filename, pattern: '{pattern}'}}\n    statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-0003-first.md"),
        "---\nStatus: Accepted\nDate: 2026-01-01\nAuthor: x\nStable-Id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n# ADR-3 - First\n",
    )
    .unwrap();
    commit_all(&dir);
    dir
}

const PADDED_PATTERN: &str = r"^(?P<prefix>ADR)-(?P<number>\d{4})-(?P<slug>.+)$";

fn new_report(dir: &Path, title: &str) -> serde_json::Value {
    let out = run_urzua(dir, &["new", "adr", title, "--by", "tester"]);
    assert!(
        out.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("JSON on stdout")
}

fn check_is_clean(dir: &Path) {
    commit_all(dir);
    let out = run_urzua(dir, &["check"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{parsed}");
    assert_eq!(parsed["findings"].as_array().unwrap().len(), 0, "{parsed}");
}

/// `MILE-151`/`BUG-182`: the padded filename, the prefixed H1 and a real status,
/// with no manual edit; `check` over the result is clean.
#[test]
fn new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing() {
    let dir = padded_repo("padded", "[\"Stable-Id\"]", PADDED_PATTERN);
    let report = new_report(&dir, "Second thing");
    assert_eq!(report["path"], "docs/adr/ADR-0004-second-thing.md");
    assert_eq!(report["display_number"], 4);
    let content = std::fs::read_to_string(dir.join("docs/adr/ADR-0004-second-thing.md")).unwrap();
    assert!(content.contains("\nStatus: Proposed\n"), "{content}");
    assert!(!content.contains("null"), "{content}");
    assert!(content.contains("\nStable-Id: "), "{content}");
    assert!(
        content.contains("\n# ADR-0004 — Second thing\n"),
        "the H1 id is padded exactly as the file name is: {content}"
    );
    check_is_clean(&dir);
    std::fs::remove_dir_all(&dir).ok();
}

/// `ADR-21`: every record type gets a `Stable-Id`. A closed field set that does
/// not list it is a configuration inconsistency, so `new` still writes it and
/// warns naming the inconsistency, never omitting it silently.
#[test]
fn new_always_writes_a_stable_id_and_warns_when_the_closed_field_set_omits_it_observed_failing() {
    let dir = padded_repo("closed-set", "[]", PADDED_PATTERN);
    let report = new_report(&dir, "Second thing");
    let id = report["stable_id"].as_str().expect("stable_id populated");
    assert!(!id.is_empty(), "{report}");
    let content = std::fs::read_to_string(dir.join("docs/adr/ADR-0004-second-thing.md")).unwrap();
    assert!(content.contains(&format!("Stable-Id: {id}")), "{content}");
    let notices = report["notices"]
        .as_array()
        .expect("a notice names the inconsistency");
    assert!(
        notices.iter().any(|n| n["severity"] == "warning"
            && n["message"].as_str().unwrap().contains("Stable-Id")
            && n["message"].as_str().unwrap().contains("known_fields")),
        "{report}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A pattern `new` cannot produce a conforming name for is refused before
/// anything is written, never answered with a name `check` would flag.
#[test]
fn new_refuses_when_the_declared_pattern_cannot_name_the_file() {
    let dir = padded_repo(
        "unproducible",
        "[\"Stable-Id\"]",
        r"^(?P<prefix>ADR)_(?P<number>\d+)_(?P<slug>.+)$",
    );
    let out = run_urzua(&dir, &["new", "adr", "Second thing", "--by", "tester"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("identity"));
    let names: Vec<_> = std::fs::read_dir(dir.join("docs/adr"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// Numbering reads the declared pattern too, so a corpus the default grammar
/// cannot read is still counted.
#[test]
fn new_numbers_from_the_declared_pattern() {
    let dir = padded_repo("numbering", "[\"Stable-Id\"]", PADDED_PATTERN);
    std::fs::write(
        dir.join("docs/adr/ADR-0012-twelve.md"),
        "---\nStatus: Accepted\nDate: 2026-01-01\nAuthor: x\n---\n# ADR-12 - Twelve\n",
    )
    .unwrap();
    commit_all(&dir);
    let report = new_report(&dir, "Next");
    assert_eq!(report["path"], "docs/adr/ADR-0013-next.md");
    std::fs::remove_dir_all(&dir).ok();
}

/// `BUG-188`: a type's template names header fields the type declares but does
/// not require; the new record keeps them, blank, and `check` stays clean. A
/// template field the type does not declare is not carried.
#[test]
fn new_keeps_the_templates_declared_non_required_header_fields_observed_failing() {
    let dir = padded_repo(
        "template-fields",
        "[\"Stable-Id\", \"Deciders\"]",
        PADDED_PATTERN,
    );
    std::fs::write(
        dir.join(".urzua/templates/adr.md"),
        "# NNNN — Title\n\n> Status: Proposed | Accepted\n> Deciders: name(s) (optional)\n> Mood: not declared by the type\n\n## Context\n\nx\n",
    )
    .unwrap();
    let report = new_report(&dir, "Second thing");
    let path = report["path"].as_str().unwrap().to_string();
    let content = std::fs::read_to_string(dir.join(&path)).unwrap();
    assert!(content.contains("\nDeciders: null\n"), "{content}");
    assert!(!content.contains("Mood"), "{content}");
    assert!(content.contains("## Context"), "{content}");
    check_is_clean(&dir);
    std::fs::remove_dir_all(&dir).ok();
}

/// MILE-176: the fields `new` fills with the date and the author are the ones
/// the type declares, not the literals `Date` and `Author`.
#[test]
fn new_fills_differently_named_date_and_author_fields_from_the_declaration() {
    let dir = fixture_repo("date-author-roles");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Created\", \"Owner\"]\n    header_shape: \"yaml-frontmatter\"\n    relation_fields:\n      date: \"Created\"\n      author: \"Owner\"\n",
    )
    .unwrap();
    commit_all(&dir);
    let output = run_urzua(&dir, &["new", "adr", "Named fields", "--by", "somebody"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let content = std::fs::read_to_string(dir.join(v["path"].as_str().unwrap())).unwrap();
    assert!(!content.contains("Created: null"), "{content}");
    assert!(!content.contains("Owner: null"), "{content}");
    assert!(content.contains("Owner:"), "{content}");
}

/// MILE-130: a type declaring `extra_dirs` is one numbering namespace, so
/// `new` numbers past the highest id in any of its directories, and past one
/// claimed in an extra directory on another branch.
#[test]
fn new_numbers_across_every_directory_of_a_type() {
    let dir = fixture_repo("namespace");
    std::fs::create_dir_all(dir.join("docs/adr-archive")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error}\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    extra_dirs: [\"docs/adr-archive\"]\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-3-a.md"),
        "---\nStatus: Accepted\n---\n# ADR-3 — A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr-archive/ADR-7-b.md"),
        "---\nStatus: Accepted\n---\n# ADR-7 — B\n",
    )
    .unwrap();
    commit_all(&dir);
    // Claimed in the extra directory on a branch this clone has.
    git(&dir, &["checkout", "-q", "-b", "other"]);
    std::fs::write(
        dir.join("docs/adr-archive/ADR-9-c.md"),
        "---\nStatus: Accepted\n---\n# ADR-9 — C\n",
    )
    .unwrap();
    commit_all(&dir);
    git(&dir, &["checkout", "-q", "-"]);

    let output = run_urzua(&dir, &["new", "adr", "Next"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let out: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(out["display_number"], 10);
    // Written to the primary directory.
    assert_eq!(out["path"], "docs/adr/ADR-10-next.md");
}
