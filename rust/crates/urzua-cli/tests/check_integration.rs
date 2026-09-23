//! CLI-golden integration tests: build a real, isolated git-repo fixture and
//! run the actual compiled binary against it. The fixture lives in its own
//! temp git repo -- entirely outside this project's own discovered record
//! set, so the containment SPEC-0002 requires is true by construction, not
//! assumed.

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
    let dir = std::env::temp_dir().join(format!("urzua-cli-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
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

#[test]
fn scope_source_is_the_declared_contract_value_not_a_debug_rendering() {
    // SPEC-0002 declares this value. Emitting `format!("{:?}", ..)` of an
    // internal enum made an ordinary rename a silent contract change
    // (BUG-0025), so assert the real stdout rather than the type.
    let dir = fixture_repo("scope-source");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let parsed = assert_valid_json_object(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(parsed["scope"]["source"], "tracked-sweep");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_exits_0_on_a_clean_corpus() {
    let dir = fixture_repo("clean");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_exits_1_on_a_missing_required_field() {
    let dir = fixture_repo("findings");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Deciders\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deciders"), "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_exits_2_when_no_config_exists() {
    let dir = fixture_repo("noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(output.status.code(), Some(2));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_untracked_scratch_file_is_never_examined() {
    // The exact regression this project's own design calls out: a raw
    // filesystem walk would pick this up; git-tracked discovery must not.
    let dir = fixture_repo("scratch");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    std::fs::write(dir.join("docs/adr/0002-untracked.md"), "not a real record").unwrap();

    let output = run_urzua(&dir, &["check", "docs/"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"files_examined\": 1"),
        "expected exactly 1 file examined (the untracked file must be excluded): {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_waiver_record_suppresses_blocking_but_the_finding_stays_listed() {
    let dir = fixture_repo("waiver");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n\
         \x20   known_fields: [\"Implements\"]\n    pointer_fields: [\"Implements\"]\n    narrative_fields: []\n\
         \x20 waiver:\n    dir: \"docs/waiver\"\n    required_fields: []\n\
         \x20   pointer_fields: []\n    narrative_fields: []\n",
    )
    .unwrap();
    // A dangling pointer -- pointer.resolution's one error case, and not
    // redundant with any other rule, so waiving it is the only thing
    // standing between this fixture and a clean exit.
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n> Implements: RFC-9999\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waiver/0001-w.md"),
        "# 0001 — Waive the dangling RFC-9999 reference on ADR-0001\n\n\
         > Rule: pointer.resolution\n\
         > Scope: docs/adr/0001-x.md\n\
         > Reason: RFC-9999 is tracked externally, not yet in this corpus\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a waived finding must not block: {stdout}"
    );
    assert!(
        stdout.contains("\"waived\""),
        "the waived finding must still be listed, not omitted: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_reports_missing_config_as_exit_2() {
    let dir = fixture_repo("doctor-noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(2));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_emits_json_not_plain_text_lines() {
    // BUG-4, observed failing before the fix: `doctor` printed `[OK]`/
    // `[WARN]`/`[ERROR]` lines, the one command that hadn't caught up with
    // ADR-23's "stdout is always JSON" contract every other command follows.
    let dir = fixture_repo("doctor-json");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("[OK]") && !stdout.contains("[WARN]") && !stdout.contains("[ERROR]"),
        "doctor must not emit bracketed plain-text lines: {stdout}"
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("doctor stdout is not JSON: {e}\n{stdout}"));
    // "warn", not "ok" -- this fixture has no .github/workflows/ci.yml, so
    // the ci-wired check correctly reports a warning; exit code stays 0
    // since a warning never blocks (only an error does).
    assert_eq!(parsed["status"], "warn");
    assert_eq!(
        output.status.code(),
        Some(0),
        "a warning must not block: {stdout}"
    );
    assert!(
        parsed["checks"].as_array().unwrap().len() >= 3,
        "expected config-exists, config-parses, and at least one record-type-dir check: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_scopes_to_the_requested_path_not_the_whole_corpus() {
    // BUG-0001, observed failing before the fix: `check`'s path argument was
    // used only to locate the repo root, never to filter which files were
    // actually examined -- `check docs/adr/` and `check docs/` returned
    // identical results. This plants two record types and asserts a
    // narrower path excludes the other one.
    let dir = fixture_repo("scope");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n\
         \x20 rfc:\n    dir: \"docs/rfc\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/rfc/0001-y.md"),
        "# 0001 — Y\n\n> Status: Draft\n",
    )
    .unwrap();
    commit_all(&dir);

    let scoped = run_urzua(&dir, &["check", "docs/adr/"]);
    let scoped_stdout = String::from_utf8_lossy(&scoped.stdout);
    assert!(
        scoped_stdout.contains("\"files_examined\": 1"),
        "scoping to docs/adr/ must exclude docs/rfc/: {scoped_stdout}"
    );

    let whole = run_urzua(&dir, &["check", "docs/"]);
    let whole_stdout = String::from_utf8_lossy(&whole.stdout);
    assert!(
        whole_stdout.contains("\"files_examined\": 2"),
        "an unscoped check must still see both files: {whole_stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn audit_exits_0_on_reciprocated_supersession() {
    let dir = fixture_repo("audit-clean");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields:\n      - \"Supersedes / Superseded-by\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n> Supersedes / Superseded-by: ADR-0002\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "# 0002 — Y\n\n> Status: Superseded\n> Supersedes / Superseded-by: ADR-0001\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["audit"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing() {
    let dir = fixture_repo("audit-violation");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields:\n      - \"Supersedes / Superseded-by\"\n",
    )
    .unwrap();
    // ADR-0001 claims to supersede ADR-0002, but ADR-0002 never points back --
    // exactly the reciprocity violation `audit` exists to catch.
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n> Supersedes / Superseded-by: ADR-0002\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "# 0002 — Y\n\n> Status: Superseded\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["audit"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(
        stdout.contains("relation.supersession-reciprocity"),
        "stdout: {stdout}"
    );
    // audit's rule set is narrower than check's -- the cross-record rules,
    // never the per-record ones (ADR-0030).
    assert!(
        !stdout.contains("header.required-fields"),
        "stdout: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A found-in-review bug: `audit` used to build its record index with the
/// lossy `build_normalized_index` and never ran `identity.collision`, so two
/// records sharing an identifier were silently resolved against whichever
/// one `HashMap` insertion happened to keep, with no finding at all --
/// unlike `check`, which reports the collision as an error.
#[test]
fn audit_reports_an_identity_collision_check_would_also_report_observed_failing() {
    let dir = fixture_repo("audit-collision");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {identity.collision: error, relation.supersession-reciprocity: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: []\n  rfc:\n    dir: \"docs/rfc\"\n    prefix: \"DOC\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-a.md"), "> Status: Accepted\n").unwrap();
    std::fs::write(dir.join("docs/rfc/DOC-1-b.md"), "> Status: Draft\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["audit"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(stdout.contains("identity.collision"), "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn init_then_check_flags_a_pre_existing_blockquote_record() {
    // ADR-33: adopt mode proposes `header_shape = "yaml-frontmatter"`
    // regardless of what shape the existing corpus happens to use -- so a
    // record already written in the deprecated `blockquote` shape (as this
    // one is) no longer parses cleanly against the freshly-adopted config,
    // and `check` correctly reports the mismatch rather than silently
    // reading it as though no header requirement applied.
    let dir = fixture_repo("bootstrap");
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let init_output = run_urzua(&dir, &["init"]);
    assert_eq!(
        init_output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );
    assert!(dir.join(".urzua/config.yaml").exists());

    let check_output = run_urzua(&dir, &["check", "docs/"]);
    let stdout = String::from_utf8_lossy(&check_output.stdout);
    // Exit 0, not 1: `init` proposes every rule at `warn`, so an adopted
    // corpus is told what is irregular without being blocked on day one
    // (MILE-51). The finding itself must still be present -- the assertion
    // below is the real one, and the exit code is secondary to it.
    assert_eq!(check_output.status.code(), Some(0), "stdout: {stdout}");
    assert!(
        stdout.contains("no header-shaped region found"),
        "stdout: {stdout}"
    );

    // Idempotence: a second init must refuse, not clobber.
    let second_init = run_urzua(&dir, &["init"]);
    assert_eq!(second_init.status.code(), Some(2));

    std::fs::remove_dir_all(&dir).ok();
}

fn assert_valid_json_object(stdout: &str) -> serde_json::Value {
    let parsed: serde_json::Value = serde_json::from_str(stdout)
        .unwrap_or_else(|e| panic!("stdout was not valid JSON: {e}\nstdout: {stdout}"));
    assert!(parsed.is_object(), "stdout: {stdout}");
    parsed
}

/// Before ADR-0046/`Report`/`emit()`, `fix`'s "could not run" path (no
/// config) printed nothing to stdout at all -- only an `eprintln!`. Every
/// fatal path now emits real, parseable JSON, unconditionally.
#[test]
fn fix_could_not_run_emits_json() {
    let dir = fixture_repo("fix-noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["fix"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["status"], "not-run", "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

/// Regression for a real exit-code change CodeRabbit's own adversarial pass
/// found: the pre-refactor code kept `fix --apply`'s exit code independent
/// of `examined` (`0` if nothing failed, `1` otherwise) -- an empty corpus
/// exited `0`. An early draft of `build_fix_report` unconditionally treated
/// `examined == 0` as `FixStatus::NotRun` (exit `2`) regardless of mode,
/// silently changing apply mode's exit code on an empty/undiscovered corpus.
#[test]
fn fix_apply_on_an_empty_corpus_still_exits_0() {
    let dir = fixture_repo("fix-apply-empty");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["fix", "--apply", "--force"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "fix --apply on an empty corpus must still exit 0, matching pre-refactor behavior: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn explain_could_not_run_emits_json() {
    let dir = fixture_repo("explain-noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["explain", "README.md"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["status"], "not-run", "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn graph_could_not_run_emits_json() {
    let dir = fixture_repo("graph-noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["graph"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["status"], "not-run", "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

/// A found-in-review gap: `graph` used to build its index with the lossy
/// `build_normalized_index` and never disclosed a collision at all, unlike
/// `check`/`audit`. An edge naming a colliding identifier could silently
/// point at the arbitrary winner with no way to know.
#[test]
fn graph_discloses_an_identity_collision_as_a_notice_observed_failing() {
    let dir = fixture_repo("graph-collision");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: []\n  rfc:\n    dir: \"docs/rfc\"\n    prefix: \"DOC\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-a.md"), "> Status: Accepted\n").unwrap();
    std::fs::write(dir.join("docs/rfc/DOC-1-b.md"), "> Status: Draft\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["graph"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    let notices = parsed["notices"].as_array().unwrap();
    assert!(
        notices.iter().any(|n| n["subject"] == "identity.collision"
            && n["message"].as_str().unwrap().contains("DOC-1")),
        "stdout: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// `doctor`'s missing-config path builds its own richer `DoctorReport`
/// (with the `checks` array intact) rather than the shared `CouldNotRun` --
/// still real, parseable JSON, with the 2/1/0 exit-code split preserved.
#[test]
fn doctor_missing_config_emits_parseable_json_with_checks() {
    let dir = fixture_repo("doctor-noconfig-json");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert!(parsed["checks"].is_array(), "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

/// `--help` is the one stated plain-text exception (ADR-0046, matching
/// `cargo`'s own convention) -- never JSON-wrapped, exit 0.
#[test]
fn help_exits_0_with_plain_text_not_json() {
    let dir = fixture_repo("help");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "--help must not be JSON-wrapped: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A genuine bad-flag parse error goes through `try_parse()` -> `emit()`
/// like any other fatal command error -- not a clap-owned panic/exit with
/// unstructured stderr text.
#[test]
fn a_bad_flag_emits_json_not_a_panic() {
    let dir = fixture_repo("bad-flag");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["--this-flag-does-not-exist"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["status"], "not-run", "stdout: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

/// `urzua init`'s real report shape (BUG-20): `proposed`/`written`/
/// `config_yaml`, not plain-text prose. `--dry-run` must include
/// `config_yaml` (nothing else to read the preview from); a real write must
/// not (`new`'s own established convention -- the caller reads the file).
#[test]
fn init_dry_run_reports_the_proposed_config_as_structured_json() {
    let dir = fixture_repo("init-dry-run");
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["init", "--dry-run"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["dry_run"], true, "stdout: {stdout}");
    assert_eq!(parsed["written"], false, "stdout: {stdout}");
    assert_eq!(parsed["proposed"][0]["name"], "adr", "stdout: {stdout}");
    assert!(
        parsed["config_yaml"]
            .as_str()
            .unwrap()
            .contains("record_types:\n  adr:"),
        "stdout: {stdout}"
    );
    assert!(
        !dir.join(".urzua/config.yaml").exists(),
        "--dry-run must not write"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn init_real_write_omits_config_yaml_from_the_report() {
    let dir = fixture_repo("init-write");
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["init"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["written"], true, "stdout: {stdout}");
    assert!(
        parsed.get("config_yaml").is_none(),
        "a real write must not echo the content back, matching `new`'s convention: {stdout}"
    );
    assert!(dir.join(".urzua/config.yaml").exists());

    std::fs::remove_dir_all(&dir).ok();
}

/// `urzua migrate ids`'s real report shape (BUG-20): `missing`/`results`
/// with a real per-file outcome, not `[OK]`/`[SKIPPED]`/`[FAILED]` prose
/// lines.
#[test]
fn migrate_ids_apply_reports_per_file_outcomes_as_structured_json() {
    let dir = fixture_repo("migrate-ids");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    // No header-shaped region -- deliberately exercises the `skipped`
    // outcome, the one path with a real per-file `error` message.
    std::fs::write(dir.join("docs/adr/0001-x.md"), "# 0001 — X\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["migrate", "ids", "--apply"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["apply"], true, "stdout: {stdout}");
    assert_eq!(
        parsed["results"][0]["outcome"], "skipped",
        "stdout: {stdout}"
    );
    assert!(
        parsed["results"][0]["error"]
            .as_str()
            .unwrap()
            .contains("no header-shaped region"),
        "stdout: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// A real write failure exits 1, matching `fix --apply`'s own
/// `PartialFailure` signal -- unlike `Skipped`, which stays exit 0.
/// Unix-only: the failure is induced via a read-only file permission.
#[cfg(unix)]
#[test]
fn migrate_ids_exits_1_on_a_real_write_failure() {
    let dir = fixture_repo("migrate-ids-write-fail");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.layout-consistency: warn, header.field-set-consistency: warn, header.deprecated-shape: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    // A real header-shaped region (a blockquote line) so the backfill
    // attempts a write, then made read-only so that write fails.
    let record_path = dir.join("docs/adr/0001-x.md");
    std::fs::write(&record_path, "# 0001 — X\n\n> Status: Accepted\n").unwrap();
    commit_all(&dir);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&record_path, std::fs::Permissions::from_mode(0o444)).unwrap();
    }

    let output = run_urzua(&dir, &["migrate", "ids", "--apply"]);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&record_path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(
        parsed["results"][0]["outcome"], "failed",
        "stdout: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// ADR-7: "off" and "ran clean" must stay distinguishable. A rule a repository
/// did not turn on is reported as not-enabled with zero records examined --
/// never omitted, which would read as though it had run and found nothing.
#[test]
fn an_undeclared_rule_is_reported_as_not_enabled_never_omitted() {
    let dir = fixture_repo("not-enabled");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {field.quality: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let executed = parsed["rules_executed"].as_array().unwrap();

    let quality = executed
        .iter()
        .find(|r| r["rule"] == "field.quality")
        .expect("a declared rule must appear");
    assert_eq!(quality["status"], "ran");

    let required = executed
        .iter()
        .find(|r| r["rule"] == "header.required-fields")
        .expect("an undeclared rule must still appear, marked not-enabled");
    assert_eq!(required["status"], "not-enabled");
    assert!(
        required.get("population").is_none(),
        "a rule that did not run has no population to report: {required}"
    );

    // Every rule this build ships is accounted for, so a reader can tell
    // "not configured" from "does not exist". Asserted against ALL_RULES
    // rather than a literal, so a rule added there and not wired into
    // `check.rs` names itself instead of failing as a count mismatch.
    let reported: std::collections::HashSet<&str> = executed
        .iter()
        .map(|r| r["rule"].as_str().unwrap())
        .collect();
    for id in urzua_core::rules::ALL_RULES {
        assert!(
            reported.contains(id),
            "rule {id} is in ALL_RULES but never ran"
        );
    }
    assert_eq!(
        executed.len(),
        urzua_core::rules::ALL_RULES.len(),
        "{executed:?}"
    );
}

/// The declared level replaces whatever severity the rule body chose -- that is
/// the whole of MILE-80. `field.quality` emits Error on a blank field; a
/// repository declaring it `warn` must not be blocked by it.
#[test]
fn the_declared_level_overrides_the_rule_body_s_own_severity() {
    let dir = fixture_repo("level-override");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {field.quality: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/0001-x.md"), "# 0001 — X\n\n> Status:\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let findings = parsed["findings"].as_array().unwrap();
    assert!(!findings.is_empty(), "the rule must still fire: {parsed}");
    for f in findings {
        assert_eq!(f["severity"], "warning", "{f}");
    }
    assert_eq!(parsed["blocking"], false);
    assert_eq!(output.status.code(), Some(0));
}

/// The rule's correctness lives in the CLI closure, not the pure function: a
/// stub `present` in a unit test proves nothing about how existence is decided.
/// BUG-22 shipped inert for exactly this reason -- the pure half was tested and
/// the wiring was not.
#[test]
fn a_locator_naming_a_staged_deletion_is_reported_observed_failing() {
    let dir = fixture_repo("staged-deletion");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {embodiment.locator-exists: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Realized-by\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("src.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Status: Accepted\n> Realized-by: code:src.rs\n",
    )
    .unwrap();
    commit_all(&dir);

    // Tracked and on disk: silent.
    let out = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let hits = |p: &serde_json::Value| {
        p["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["rule"] == "embodiment.locator-exists")
            .count()
    };
    assert_eq!(hits(&parsed), 0, "{parsed}");

    // `git rm` drops it from ls-files but leaves it in diff --cached, and
    // discovery unions the two -- so a tracked-set check alone stays silent
    // here, which is the case the rule exists for.
    git(&dir, &["rm", "-q", "src.rs"]);
    let out = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(hits(&parsed), 1, "{parsed}");
    assert_eq!(parsed["blocking"], true);
}

#[test]
fn a_path_scope_narrows_what_is_reported_on_not_what_a_pointer_resolves_against() {
    let dir = fixture_repo("scope-pointer");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {pointer.resolution: error}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n    known_fields: [\"Derives-from\"]\n    pointer_fields: [\"Derives-from\"]\n\
         \x20 rfc:\n    dir: \"docs/rfc\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nDerives-from: RFC-1\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/rfc/RFC-1-y.md"), "---\n---\n# 1 — Y\n").unwrap();
    commit_all(&dir);

    let scoped = run_urzua(&dir, &["check", "docs/adr/"]);
    let stdout = String::from_utf8_lossy(&scoped.stdout);
    assert!(
        !stdout.contains("does not resolve"),
        "RFC-1 exists outside the scope; scoping must not make it dangle: {stdout}"
    );
    assert!(
        stdout.contains("\"files_examined\": 1"),
        "only the in-scope record is reported on: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed["records_read_by_any_rule"], 1,
        "the assertion above is vacuous unless a rule actually read the record: {stdout}"
    );
}

#[test]
fn a_claim_path_prefix_is_read_to_any_depth() {
    let dir = fixture_repo("claim-nested");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("changes/2026-09")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("changes/2026-09/0042-fix.md"),
        "---\ndefault: patch\n---\n\nFixes BUG-1.\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("claim.status-agreement"),
        "rule must appear: {stdout}"
    );
    assert!(
        !stdout.contains("\"rule\": \"claim.status-agreement\",\n      \"records_examined\": 0"),
        "a nested claim layout must not report a clean run over zero claims: {stdout}"
    );
    assert!(
        stdout.contains("BUG-1"),
        "the nested changeset claims a bug that is still Open: {stdout}"
    );
}

/// `BUG-121`: `BUG-67`'s path-prefix scope filter is correct for a rule
/// reporting on a record, but `claim.status-agreement`'s findings name a
/// claim file under `claim_paths`, which no record-type `dir` scope covers
/// -- so a scoped invocation (`check docs/adr`, not unscoped `check`)
/// silently dropped a real false-claim finding, the same shape `BUG-86`
/// named for this repository's own `make records` before that fix changed
/// the *caller* to stop scoping rather than fixing the filter itself.
#[test]
fn a_scoped_invocation_still_reports_a_claim_finding_observed_failing() {
    let dir = fixture_repo("claim-outside-scope");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("changes")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("changes/0001-x.md"),
        "---\ndefault: patch\n---\n\nCloses ADR-1.\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check", "docs/adr"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("claim.status-agreement"),
        "a scoped invocation must still report the claim finding: {stdout}"
    );
    assert!(
        stdout.contains("ADR-1"),
        "the finding must still name the falsely-claimed record: {stdout}"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["blocking"], true, "{parsed}");
}

#[test]
fn a_record_below_a_declared_dir_but_not_in_it_is_reported_not_dropped() {
    let dir = fixture_repo("unowned");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr/archive")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {type.record-outside-declared-dir: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/archive/ADR-9-old.md"),
        "---\nStatus: Superseded\n---\n# 9 — Old\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"files_examined\": 1"),
        "RFC-35: the subdirectory record is not owned by the type: {stdout}"
    );
    assert!(
        stdout.contains("docs/adr/archive/ADR-9-old.md"),
        "and the tool must say so rather than drop it silently: {stdout}"
    );
    assert!(
        stdout.contains("no type \\nowns it") || stdout.contains("no type"),
        "with a message naming the cause: {stdout}"
    );
}

#[test]
fn a_scope_excludes_a_finding_about_a_file_that_is_not_a_record() {
    let dir = fixture_repo("scope-nonrecord");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs/archive")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {type.record-outside-declared-dir: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/archive/BUG-1-old.md"),
        "---\nStatus: Open\n---\n# 1 — Old\n",
    )
    .unwrap();
    commit_all(&dir);

    let whole = String::from_utf8_lossy(&run_urzua(&dir, &["check"]).stdout).to_string();
    assert!(
        whole.contains("docs/bugs/archive/BUG-1-old.md"),
        "unscoped, the unowned file is reported: {whole}"
    );

    let scoped =
        String::from_utf8_lossy(&run_urzua(&dir, &["check", "docs/adr/"]).stdout).to_string();
    assert!(
        !scoped.contains("docs/bugs/archive/BUG-1-old.md"),
        "scoping to docs/adr/ must exclude it, though it is not a record: {scoped}"
    );
}

#[test]
fn a_scope_keeps_a_finding_about_a_file_git_does_not_track() {
    let dir = fixture_repo("scope-untracked");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("changes")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);
    // Deliberately uncommitted: this is the pre-commit case the rule exists for.
    std::fs::write(dir.join("changes/0001-f.md"), "Fixes BUG-1.\n").unwrap();

    let whole = run_urzua(&dir, &["check"]);
    let dotted = run_urzua(&dir, &["check", "."]);
    assert_eq!(
        whole.status.code(),
        Some(1),
        "unscoped, the open record is blocking: {}",
        String::from_utf8_lossy(&whole.stdout)
    );
    assert_eq!(
        dotted.status.code(),
        Some(1),
        "`check .` must agree with `check` over the same corpus: {}",
        String::from_utf8_lossy(&dotted.stdout)
    );
}

#[test]
fn a_claim_paths_entry_naming_a_file_does_not_load() {
    let dir = fixture_repo("claim-file");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"NOTES.md\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("NOTES.md"), "Fixes BUG-1.\n").unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_ne!(
        out.status.code(),
        Some(0),
        "a claim_paths entry naming a file must not report a clean run: {stdout}"
    );
}

#[test]
fn a_symlink_cycle_inside_a_claim_path_terminates_and_reads_each_claim_once() {
    let dir = fixture_repo("claim-symlink");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("changes")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: warn\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("changes/0001-f.md"), "Fixes BUG-1.\n").unwrap();
    std::os::unix::fs::symlink("..", dir.join("changes/loop")).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    // `changes/loop -> ..` points at an ancestor of the declared prefix, which
    // would sweep the whole repository and report on files never declared as
    // claims (BUG-88). Refused, and the message says why.
    assert_eq!(out.status.code(), Some(2), "must not run: {stdout}");
    assert!(
        stdout.contains("ancestor of the declared prefix"),
        "and must name the reason: {stdout}"
    );
}

#[test]
fn a_staged_deletion_is_not_reported_as_owned_by_no_type() {
    let dir = fixture_repo("staged-deletion-ownership");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {type.record-outside-declared-dir: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Accepted\n---\n# 2 — Y\n",
    )
    .unwrap();
    commit_all(&dir);
    std::process::Command::new("git")
        .args(["rm", "-q", "docs/adr/ADR-2-y.md"])
        .current_dir(&dir)
        .status()
        .unwrap();

    let stdout = String::from_utf8_lossy(&run_urzua(&dir, &["check"]).stdout).to_string();
    assert!(
        !stdout.contains("ADR-2-y.md"),
        "it sits directly in the declared dir and no longer exists: {stdout}"
    );
    // Positive control: without it this test passes just as well when the rule
    // never ran at all, which is the shape ADR-55 forbids.
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let population = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "type.record-outside-declared-dir")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_ne!(
        population["examined"], 0,
        "the rule must have examined the declared dir\'s files: {stdout}"
    );
}

fn one_adr_repo(name: &str, rules: &str, body: &str) -> std::path::PathBuf {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\n\
             record_types:\n\
             \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-x.md"), body).unwrap();
    commit_all(&dir);
    dir
}

#[test]
fn a_run_in_which_no_rule_examined_anything_is_not_ok() {
    // A record that would fail header.required-fields, with no rule declared
    // to look at it. Reporting `ok` makes a config that lost its rules block
    // indistinguishable from a clean corpus (ADR-55).
    let dir = one_adr_repo("no-rules", "{}", "---\nTitle: X\n---\n# 1 — X\n");
    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("\"status\": \"not-run\""),
        "no rule examined anything, so nothing was established: {stdout}"
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "and it must not exit 0: {stdout}"
    );
}

/// `BUG-99`: `init` on a prefixless corpus proposes no identity-dependent
/// rules, which happens to be `audit`'s entire rule set -- so the adopter's
/// documented first two commands (`init` then `audit`) exit 2 immediately.
/// Not a bug in `audit`'s exit-code policy (`BUG-77`, unchanged and still
/// enforced below): `init` now discloses it up front instead of letting the
/// adopter discover it from `audit`'s own exit code.
#[test]
fn init_warns_when_its_proposed_config_leaves_audit_with_nothing_declared() {
    let dir = fixture_repo("init-prefixless-warns-about-audit");
    std::fs::write(dir.join("docs/adr/0001-x.md"), "> Status: Accepted\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["init"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "init itself must still succeed: {stdout}"
    );
    assert!(
        stdout.contains("rule-applicability"),
        "a prefixless corpus must warn that audit's rule set is entirely undeclared: {stdout}"
    );
    assert!(
        stdout.contains("audit"),
        "the notice must name the affected command: {stdout}"
    );

    // `identity.collision` doesn't need a declared prefix to run
    // meaningfully (unlike `pointer.resolution`/`relation.supersession-
    // reciprocity`, which `identity_dependent` correctly drops here) --
    // `audit` now has one rule that legitimately runs on this corpus, so
    // `BUG-99`'s "exits non-zero for no real reason" symptom is gone: a
    // thin-config corpus with nothing actually wrong is `ok`, per `ADR-53`.
    let audit_out = run_urzua(&dir, &["audit"]);
    let audit_stdout = String::from_utf8_lossy(&audit_out.stdout).to_string();
    assert!(
        audit_stdout.contains("\"status\": \"ok\""),
        "identity.collision runs regardless of prefix and finds nothing wrong: {audit_stdout}"
    );
    assert_eq!(
        audit_out.status.code(),
        Some(0),
        "nothing is actually wrong with this corpus: {audit_stdout}"
    );
}

#[test]
fn audit_with_neither_of_its_rules_declared_is_not_ok() {
    let dir = one_adr_repo(
        "audit-no-rules",
        "{header.required-fields: error}",
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    );
    let out = run_urzua(&dir, &["audit"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("\"status\": \"not-run\""),
        "audit executed no rule: {stdout}"
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "audit must not exit 0 when no rule examined anything: {stdout}"
    );
}

#[test]
fn an_unreadable_tracked_record_stops_the_run_rather_than_shrinking_the_corpus() {
    let dir = one_adr_repo(
        "unreadable",
        "{header.required-fields: error}",
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    );
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Accepted\n---\n# 2 — Y\n",
    )
    .unwrap();
    commit_all(&dir);
    // Invalid UTF-8 is the realistic trigger; permissions are flakier in CI.
    std::fs::write(dir.join("docs/adr/ADR-2-y.md"), [0xff, 0xfe, 0x00, 0x9f]).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("could not read"),
        "an unreadable record must be named, not dropped: {stdout}"
    );
    assert_ne!(out.status.code(), Some(0), "and must not exit 0: {stdout}");
}

#[test]
fn two_records_claiming_one_identifier_are_reported() {
    let dir = fixture_repo("identity-collision");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {identity.collision: error}\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-y.md"),
        "---\nStatus: Fixed\n---\n# 1 — Y\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("is claimed by 2 records"),
        "the index keeps one and the other stops existing: {stdout}"
    );
    assert_eq!(out.status.code(), Some(1), "blocking: {stdout}");
}

#[test]
fn a_symlinked_claim_file_is_read_not_skipped() {
    let dir = fixture_repo("claim-symlinked-file");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("store")).unwrap();
    std::fs::create_dir_all(dir.join("changes")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("store/0001-f.md"), "Fixes BUG-1.\n").unwrap();
    std::os::unix::fs::symlink("../store/0001-f.md", dir.join("changes/0001-f.md")).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("claims to close BUG-1"),
        "a symlinked claim is still a claim: {stdout}"
    );
    assert_eq!(out.status.code(), Some(1), "BUG-1 is Open: {stdout}");
}

#[test]
fn a_claim_paths_root_symlinked_to_a_real_directory_is_usable() {
    let dir = fixture_repo("claim-root-symlink");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("store")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("store/0001-f.md"), "Fixes BUG-1.\n").unwrap();
    std::os::unix::fs::symlink("store", dir.join("changes")).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        !stdout.contains("not a readable directory") && !stdout.contains("does not resolve"),
        "it is a readable directory inside the repository: {stdout}"
    );
    assert!(
        stdout.contains("claims to close BUG-1"),
        "and its claims must be read: {stdout}"
    );
}

/// `BUG-117`: a symlink resolving to exactly the declared prefix root (not a
/// genuine ancestor) used to abort the whole run, even though `visited`
/// already dedupes it silently -- no unbounded walk would occur.
#[test]
fn a_symlink_resolving_to_the_prefix_root_itself_is_deduped_not_rejected() {
    let dir = fixture_repo("claim-self-referential-symlink");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("changes")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("changes/0001-f.md"), "Fixes BUG-1.\n").unwrap();
    // Resolves to `changes` itself, not an ancestor of it.
    std::os::unix::fs::symlink(".", dir.join("changes/self")).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        !stdout.contains("resolves to an ancestor"),
        "a link to the prefix root itself is not an ancestor: {stdout}"
    );
    assert!(
        stdout.contains("claims to close BUG-1"),
        "the real claim file is still read: {stdout}"
    );
}

#[test]
fn a_config_scoped_rule_alone_discloses_that_it_read_no_record() {
    // type.no-declared-spec counts configured types, not records, and reported
    // them as `records_examined` -- so "some rule examined something" was
    // satisfied by a rule that had opened no file (BUG-81).
    //
    // The count is unit-bearing, so it cannot be mistaken for records, and the
    // verdict doesn't read it: declaring only this rule is a thin config, which
    // ADR-53 makes the adopter's call. What must not happen is the run reading
    // as a clean corpus, and `records_read_by_any_rule: 0` prevents that.
    let dir = fixture_repo("config-scope-only");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {type.no-declared-spec: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n    spec: \"SPEC-1\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nTitle: X\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pop = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "type.no-declared-spec")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_eq!(
        pop["unit"], "record-type",
        "it counts declarations, not records: {stdout}"
    );
    assert_eq!(
        parsed["records_read_by_any_rule"], 0,
        "a declaration is not a record read: {stdout}"
    );
}

#[test]
fn an_unstaged_deletion_of_a_tracked_record_stops_the_run() {
    let dir = one_adr_repo(
        "unstaged-deletion",
        "{header.required-fields: error}",
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    );
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Accepted\n---\n# 2 — Y\n",
    )
    .unwrap();
    commit_all(&dir);

    // Deleted from the worktree, not staged: git still tracks it.
    std::fs::remove_file(dir.join("docs/adr/ADR-2-y.md")).unwrap();
    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("ADR-2-y.md"),
        "the missing record must be named: {stdout}"
    );
    assert_ne!(out.status.code(), Some(0), "and must not exit 0: {stdout}");

    // Staged, it is a deletion the corpus has accounted for.
    std::process::Command::new("git")
        .args(["rm", "-q", "--cached", "docs/adr/ADR-2-y.md"])
        .current_dir(&dir)
        .status()
        .unwrap();
    let staged = run_urzua(&dir, &["check"]);
    assert_eq!(
        staged.status.code(),
        Some(0),
        "a staged deletion is legitimately absent: {}",
        String::from_utf8_lossy(&staged.stdout)
    );
}

#[test]
fn a_path_scoped_rule_alone_discloses_that_it_read_no_record() {
    // type.record-outside-declared-dir reads tracked path names and never opens
    // a file, so it cannot establish anything about a record's contents -- yet
    // it reported those names as `records_examined` (BUG-83).
    //
    // The unit says `path`, and `records_read_by_any_rule: 0` says no record
    // was judged, over a corpus whose one record is unparseable garbage.
    let dir = fixture_repo("path-scope-only");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {type.record-outside-declared-dir: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Id\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "garbage not a header at all\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pop = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "type.record-outside-declared-dir")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_eq!(
        pop["unit"], "path",
        "it judges filenames, not records: {stdout}"
    );
    assert_eq!(
        parsed["records_read_by_any_rule"], 0,
        "the one record was never opened: {stdout}"
    );
}

#[test]
fn audit_is_ok_on_a_clean_corpus_with_no_relationships() {
    // Both rules run and legitimately have nothing to judge. init generates
    // this config, so requiring a non-zero examined count made every fresh
    // adopter's first audit fail CI (BUG-84).
    let dir = one_adr_repo(
        "audit-no-relationships",
        "{pointer.resolution: warn, relation.supersession-reciprocity: warn}",
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    );
    let out = run_urzua(&dir, &["audit"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("\"status\": \"ok\""),
        "a corpus with no relationships is clean, not unestablished: {stdout}"
    );
    assert_eq!(out.status.code(), Some(0), "exit 0: {stdout}");
}

#[test]
fn a_directory_symlink_inside_claim_paths_is_treated_like_the_root() {
    let dir = fixture_repo("claim-nested-symlink");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("shared")).unwrap();
    std::fs::create_dir_all(dir.join("claims/real")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"claims\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(dir.join("shared/0001-f.md"), "Fixes BUG-1.\n").unwrap();
    std::os::unix::fs::symlink("../../shared", dir.join("claims/real/linked")).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        !stdout.contains("\"status\": \"not-run\""),
        "an in-repo directory symlink is accepted as a root, so it is accepted nested: {stdout}"
    );
    assert!(
        stdout.contains("claims to close BUG-1"),
        "and its claims are read: {stdout}"
    );
}

#[test]
fn ci_wired_finds_the_invocation_in_any_workflow_not_just_ci_yml() {
    // The check's own subject is "is the checker actually wired in?". Reading
    // one workflow by name made it report a falsehood about a repository where
    // it is wired in -- this repo's invocation lives in checks.yml, and
    // doctor read ci.yml.
    let dir = fixture_repo("ci-wired-scan");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    // A workflow that is not named ci.yml, and a ci.yml that does not invoke it.
    std::fs::write(
        dir.join(".github/workflows/checks.yml"),
        "name: checks\njobs:\n  records:\n    steps:\n      - run: urzua check\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".github/workflows/ci.yml"),
        "name: ci\njobs:\n  build:\n    steps:\n      - run: cargo build\n",
    )
    .unwrap();
    commit_all(&dir);

    let stdout = String::from_utf8_lossy(&run_urzua(&dir, &["doctor"]).stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let ci = parsed["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "ci-wired")
        .expect("ci-wired check must be present");
    assert_eq!(
        ci["status"], "ok",
        "the invocation is in checks.yml, so the checker IS wired in: {stdout}"
    );
}

#[test]
fn ci_wired_reports_an_error_when_the_workflows_cannot_be_read() {
    // "No workflow invokes the checker" and "the workflows could not be read"
    // are different answers. Collapsing them reports confidently about a
    // directory never opened -- the defect BUG-91 fixed, one level down.
    let dir = fixture_repo("ci-wired-unreadable");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".github/workflows/checks.yml"),
        "name: checks\njobs:\n  records:\n    steps:\n      - run: urzua check\n",
    )
    .unwrap();
    commit_all(&dir);

    use std::os::unix::fs::PermissionsExt;
    let workflows = dir.join(".github/workflows");
    std::fs::set_permissions(&workflows, std::fs::Permissions::from_mode(0o000)).unwrap();
    let out = run_urzua(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    std::fs::set_permissions(&workflows, std::fs::Permissions::from_mode(0o755)).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let ci = parsed["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "ci-wired")
        .expect("ci-wired check must be present");
    assert_eq!(
        ci["status"], "error",
        "an unreadable workflows directory is not the same as an unwired checker: {stdout}"
    );
    assert_ne!(out.status.code(), Some(0), "and must not exit 0: {stdout}");
}

#[test]
fn a_record_with_no_revision_log_is_absent_not_outside_the_population() {
    // BUG-50: "absence is indistinguishable from compliance". SPEC-1 lost 18
    // revision rows by losing one marker line and the rule stayed green. The
    // report performs the subtraction the rule alone could not.
    let dir = one_adr_repo(
        "revision-log-absent",
        "{revision-log.change-class-required: error}",
        "---\nStatus: Accepted\n---\n# 1 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Created | **structural** |\n",
    );
    // A second record with no revision log at all.
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Accepted\n---\n# 2 — Y\n\nNo log here.\n",
    )
    .unwrap();
    commit_all(&dir);

    let stdout = String::from_utf8_lossy(&run_urzua(&dir, &["check"]).stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pop = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "revision-log.change-class-required")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_eq!(pop["eligible"], 2, "both records are in scope: {stdout}");
    assert_eq!(
        pop["examined"], 1,
        "only one carries a log, and the other's absence must be visible: {stdout}"
    );
}

#[test]
fn a_type_declaring_no_layout_puts_its_records_outside_the_population() {
    // Distinct from absence: the rule does not apply at all, which is the
    // cold-start state and must read as eligible 0 rather than as a rule
    // whose matcher is broken.
    let dir = one_adr_repo(
        "layout-not-declared",
        "{header.layout-consistency: warn}",
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    );
    let stdout = String::from_utf8_lossy(&run_urzua(&dir, &["check"]).stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pop = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "header.layout-consistency")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_eq!(pop["eligible"], 0, "no type declares a layout: {stdout}");
    assert_eq!(pop["examined"], 0, "{stdout}");
}

#[test]
fn a_field_rule_counts_declared_slots_not_records() {
    // BUG-40: field.quality reported 1023 against a 309-record corpus and the
    // number was never wrong -- it was 100% of declared field slots under a
    // name claiming records. The slot list is the population, so the
    // denominator cannot be mislabelled.
    let dir = fixture_repo("field-slot-population");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {field.quality: error}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Date\", \"Author\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    for (n, slug) in [(1, "a"), (2, "b")] {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-{slug}.md")),
            "---\nStatus: Accepted\nDate: 2026-01-01\nAuthor: someone\n---\n# x\n",
        )
        .unwrap();
    }
    commit_all(&dir);

    let stdout = String::from_utf8_lossy(&run_urzua(&dir, &["check"]).stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pop = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "field.quality")
        .and_then(|r| r.get("population"))
        .expect("field.quality must carry a population");
    assert_eq!(pop["unit"], "field", "not records: {stdout}");
    assert_eq!(
        pop["eligible"], 6,
        "2 records x 3 declared required fields, not 2: {stdout}"
    );
    assert_eq!(
        parsed["files_examined"], 2,
        "the corpus is 2 records: {stdout}"
    );
}

#[test]
fn a_record_whose_name_git_quotes_is_still_examined() {
    // git C-quotes any path with a non-ASCII byte under the default
    // core.quotepath, so the name arrived wrapped in quotes and its parent
    // never matched the declared dir (BUG-87).
    let dir = one_adr_repo(
        "quoted-path",
        "{header.required-fields: error}",
        "---\nStatus: Accepted\n---\n# 1 \u{2014} X\n",
    );
    std::fs::write(
        dir.join("docs/adr/ADR-2-caf\u{e9}.md"),
        "---\nTitle: no status\n---\n# 2 \u{2014} Caf\u{e9}\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("\"files_examined\": 2"),
        "the accented record belongs to the corpus: {stdout}"
    );
    assert!(
        stdout.contains("Status"),
        "and its missing required field is reported: {stdout}"
    );
    assert_eq!(out.status.code(), Some(1), "blocking: {stdout}");
}

#[test]
fn a_symlink_cycle_that_stays_inside_the_prefix_terminates() {
    let dir = fixture_repo("claim-sibling-cycle");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bugs")).unwrap();
    std::fs::create_dir_all(dir.join("changes/a")).unwrap();
    std::fs::create_dir_all(dir.join("changes/b")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"changes\"]\n    closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 bug:\n    dir: \"docs/bugs\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bugs/BUG-1-x.md"),
        "---\nStatus: Open\n---\n# 1 \u{2014} X\n",
    )
    .unwrap();
    std::fs::write(dir.join("changes/a/0001-f.md"), "Fixes BUG-1.\n").unwrap();
    // Mutual sibling links: a cycle that never leaves the declared prefix, so
    // only the visited set can stop the walk.
    std::os::unix::fs::symlink("../b", dir.join("changes/a/tob")).unwrap();
    std::os::unix::fs::symlink("../a", dir.join("changes/b/toa")).unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let claims = stdout.matches("claims to close BUG-1").count();
    assert_eq!(
        claims, 1,
        "one claim file, read once, not once per hop: {stdout}"
    );
}

#[test]
fn a_rule_handed_nothing_discloses_that_it_certified_nothing() {
    // The census says `eligible: 0` -- the rule was handed nothing, and it says
    // so where a reader sees it (BUG-94). Enabling a rule before its scope
    // exists is the cold-start state, not a fault, so the verdict stays the
    // adopter's (ADR-53); what must not happen is two records reading as
    // examined when none were.
    let dir = one_adr_repo(
        "handed-nothing",
        "{header.layout-consistency: warn}",
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    );
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Accepted\n---\n# 2 — Y\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let pop = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "header.layout-consistency")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_eq!(pop["eligible"], 0, "no type declares a layout: {stdout}");
    assert_eq!(
        parsed["files_examined"], 2,
        "both records were read off disk: {stdout}"
    );
    assert_eq!(
        parsed["records_read_by_any_rule"], 0,
        "and neither was judged by any rule: {stdout}"
    );
}

#[test]
fn a_blocking_finding_is_never_reported_as_a_run_that_did_not_happen() {
    // A config finding survives every scope filter, so scoping to a path with
    // no records left `status: not-run` beside `blocking: true` -- a run that
    // established nothing and found a blocking error at the same time.
    let dir = fixture_repo("blocking-not-run");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/elsewhere")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {config.pointer-field-not-known: error}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Status\"]\n\
         \x20   pointer_fields: [\"Nonexistent\"]\n    narrative_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "# 1 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/elsewhere/.keep"), "x\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check", "docs/elsewhere"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["blocking"], true, "{stdout}");
    assert_ne!(
        parsed["status"], "not-run",
        "a blocking finding is something established: {stdout}"
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "blocking is exit 1, not 2: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn records_read_never_exceeds_files_examined() {
    // Rules run over the whole corpus so a reference resolves outside the scope
    // (BUG-67), but counting those here put the two numbers on different
    // denominators -- BUG-40, in the field added to end it.
    let dir = fixture_repo("scoped-denominator");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {identity.collision: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    for n in 1..=3 {
        std::fs::write(
            dir.join(format!("docs/adr/ADR-{n}-x.md")),
            format!("# {n} — X\n\n> Status: Accepted\n"),
        )
        .unwrap();
    }
    commit_all(&dir);

    let out = run_urzua(&dir, &["check", "docs/adr/ADR-1-x.md"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["files_examined"], 1, "{stdout}");
    assert_eq!(
        parsed["records_read_by_any_rule"], 1,
        "both numbers count the scope, or they are not comparable: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_type_declaring_only_required_fields_is_still_checked_for_whitespace() {
    // `known_fields_by_type` is absent for such a type, because
    // header.field-set-consistency skips it by design. A rule reading that map
    // would skip it too -- silently, which is the shape ADR-55 names.
    let dir = fixture_repo("required-only-whitespace");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {field.untrimmed-value: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n\
         \x20   header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: \"Accepted   \"\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let hits = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "field.untrimmed-value")
        .count();
    assert_eq!(hits, 1, "no known_fields must not mean no check: {stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_absent_claim_paths_directory_is_reported_not_fatal() {
    // Git keeps no empty directory, so a declared `.changeset` ceases to exist
    // the moment a release consumes the last fragment. Aborting took `check`
    // down with it. BUG-56 asked for the absence to be *visible*, which a
    // Notice achieves without moving the exit code (ADR-46).
    let dir = fixture_repo("absent-claim-paths");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\".changeset\"]\n\
         \x20   closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "# 1 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_ne!(
        parsed["status"], "not-run",
        "an absent dir is not a failure: {stdout}"
    );
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    let notices = parsed["notices"].as_array().expect("a notice names it");
    assert!(
        notices
            .iter()
            .any(|n| n["message"].as_str().unwrap_or("").contains(".changeset")),
        "the absence must still be visible (BUG-56): {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[cfg(unix)]
#[test]
fn a_claim_paths_entry_behind_a_permission_denied_ancestor_aborts() {
    // canonicalize() and exists() both fold a permission failure into the
    // same signal as genuine absence, so the BUG-56 fix's Notice-not-abort
    // path had to be checked against more than "the directory is missing" --
    // otherwise an unreadable directory reads as an empty one and the rule
    // silently examines nothing, which is BUG-56's own shape under a
    // permissions error instead of a typo.
    use std::os::unix::fs::PermissionsExt;

    let dir = fixture_repo("claim-paths-permission-denied");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("locked/claims")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n\
         \x20 claim.status-agreement:\n    level: error\n    claim_paths: [\"locked/claims\"]\n\
         \x20   closed_statuses: [\"Fixed\"]\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "# 1 — X\n\n> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    let locked = dir.join("locked");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

    // A sandbox running as root (or with an ACL bypassing the mode bits)
    // would make this fixture assert nothing real. Detect that rather than
    // let the test pass for the wrong reason.
    let bypassed = locked.join("claims").try_exists().is_ok();
    if bypassed {
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        eprintln!(
            "skipped: permission bits did not block access in this environment (running as root?)"
        );
        return;
    }

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(
        out.status.code(),
        Some(2),
        "a permission failure is not absence and must abort: {stdout}"
    );
    assert!(
        !stdout.contains("does not exist"),
        "a permission failure must not read as absence: {stdout}"
    );
    assert!(
        stdout.contains("could not"),
        "the abort must name what actually happened: {stdout}"
    );
}

/// `BUG-120`: `compute_drifted_records` matched the pre-`RFC-42` literal
/// `Realized-by` instead of resolving `RelationRole::EmbodimentLocator`, so
/// a type declaring `relation_fields.embodiment_locator` never had drift
/// detected for it -- `embodiment.consistency` could never see `Drift
/// detected` disagree with a stale stated value for such a type.
#[test]
fn drift_is_detected_against_a_custom_embodiment_locator_field_name_observed_failing() {
    let dir = fixture_repo("custom-locator-drift");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {embodiment.consistency: error}\n\n\
         record_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n\
         \x20   known_fields: [\"Phase\", \"Evidence\"]\n\
         \x20   relation_fields:\n      embodiment_state: \"Phase\"\n      embodiment_locator: \"Evidence\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("src.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Phase: Implemented\n> Evidence: code:src.rs\n",
    )
    .unwrap();
    commit_all(&dir);

    // Touch the evidence file in a later commit, without touching the
    // record -- the exact shape `commit_strictly_before` looks for.
    std::fs::write(dir.join("src.rs"), "fn main() { println!(\"x\"); }\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = parsed["findings"].as_array().unwrap();
    assert_eq!(
        findings
            .iter()
            .filter(|f| f["rule"] == "embodiment.consistency")
            .count(),
        1,
        "{parsed}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("Drift detected")),
        "{parsed}"
    );
}

/// `BUG-135`: `compute_drifted_records` memoizes `last_commit_for_path` and
/// `commit_strictly_before` per locator/commit pair rather than re-running
/// the underlying `git` subprocess for every citing record. Two records
/// citing the *same* locator must both still be detected as drifted --
/// the case a caching bug (stale entry, wrong cache key) would break first.
#[test]
fn two_records_citing_the_same_locator_are_both_detected_as_drifted() {
    let dir = fixture_repo("shared-locator-drift");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {embodiment.consistency: error}\n\n\
         record_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n\
         \x20   known_fields: [\"Embodiment\", \"Realized-by\"]\n",
    )
    .unwrap();
    std::fs::write(dir.join("shared.rs"), "fn shared() {}\n").unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\n> Embodiment: Implemented\n> Realized-by: code:shared.rs\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0002-y.md"),
        "# 0002 — Y\n\n> Embodiment: Implemented\n> Realized-by: code:shared.rs\n",
    )
    .unwrap();
    commit_all(&dir);

    // Touch the shared locator in a later commit, without touching either
    // record -- both records' `Realized-by` lines predate this commit.
    std::fs::write(dir.join("shared.rs"), "fn shared() { println!(\"x\"); }\n").unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let drift_findings: Vec<&str> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "embodiment.consistency")
        .filter(|f| f["message"].as_str().unwrap().contains("Drift detected"))
        .map(|f| f["file"].as_str().unwrap())
        .collect();
    assert_eq!(
        drift_findings.len(),
        2,
        "both citers of the shared locator must be independently detected as drifted: {parsed}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// `BUG-24`: SPEC-2's Discovery contract says an explicit argv path
/// overrides discovery and is used as given -- an untracked file named
/// directly must still be examined, not silently reported as
/// `files_examined: 0`.
#[test]
fn an_untracked_file_named_explicitly_is_examined_not_silently_dropped_observed_failing() {
    let dir = fixture_repo("argv-override-untracked");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    commit_all(&dir);

    // Written after the fixture's own commit, and never staged -- genuinely
    // untracked, the exact shape `urzua new` leaves behind since it doesn't
    // stage what it writes.
    std::fs::write(
        dir.join("docs/adr/ADR-2-untracked.md"),
        "---\n---\n# 2 — Untracked\n",
    )
    .unwrap();

    let out = run_urzua(&dir, &["check", "docs/adr/ADR-2-untracked.md"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["files_examined"], 1, "{parsed}");
    assert_eq!(parsed["scope"]["source"], "argv", "{parsed}");
    let findings = parsed["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["rule"] == "header.required-fields"),
        "the untracked record's missing Status must actually be checked: {parsed}"
    );
}

/// The override is file-only, not a directory walk: an untracked file
/// merely sitting under an explicitly-requested *directory* stays invisible,
/// the same guarantee `an_untracked_scratch_file_is_never_examined` already
/// gives the unscoped sweep (`ADR-6`'s "never a raw directory walk").
#[test]
fn an_untracked_file_below_an_explicitly_requested_directory_is_not_examined() {
    let dir = fixture_repo("argv-override-dir");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    std::fs::write(
        dir.join("docs/adr/ADR-3-untracked.md"),
        "---\n---\n# 3 — Untracked\n",
    )
    .unwrap();

    let out = run_urzua(&dir, &["check", "docs/adr"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["files_examined"], 1, "{parsed}");
    assert_eq!(
        parsed["scope"]["source"], "tracked-sweep",
        "no override applies to a directory argument: {parsed}"
    );
}

/// `BUG-126`: a record staged for deletion (`git rm`, not committed) that
/// sits below a declared type's `dir` without being directly in it must not
/// be reported as sitting outside it -- it's leaving, not misplaced.
#[test]
fn a_staged_deletion_below_a_declared_dir_is_not_reported_as_outside_it_observed_failing() {
    let dir = fixture_repo("staged-deletion-outside-dir");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr/archive")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {type.record-outside-declared-dir: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/archive/0001-x.md"),
        "> Status: Accepted\n",
    )
    .unwrap();
    // A live sibling, left in place: proves the filter drops only the staged
    // deletion, not every candidate below `docs/adr/archive` (a positive
    // control CodeRabbit's review of round 21 asked for).
    std::fs::write(
        dir.join("docs/adr/archive/0002-y.md"),
        "> Status: Accepted\n",
    )
    .unwrap();
    commit_all(&dir);

    git(&dir, &["rm", "-q", "docs/adr/archive/0001-x.md"]);

    let out = run_urzua(&dir, &["check"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = parsed["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .all(|f| f["rule"] != "type.record-outside-declared-dir"
                || f["file"] != "docs/adr/archive/0001-x.md"),
        "a file being deleted is not an ownership question: {parsed}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f["rule"] == "type.record-outside-declared-dir"
                && f["file"] == "docs/adr/archive/0002-y.md"),
        "a live sibling in the same spot must still be reported: {parsed}"
    );
}

// `BUG-127`: a reviewer flagged `resolve_argv_overrides`' `symlink_metadata`
// check as silently dropping a symlinked argv path. Investigated and found
// unreachable: `relative_scopes` canonicalizes (and thus dereferences) every
// argv path before `resolve_argv_overrides` ever sees it, so no test can
// observe a difference between following and not following the link at that
// call site. No regression test exists for this non-bug; see the `BUG-127`
// record for the full account.

/// A found-in-review bug: two record types (`adr`/`rfc`) sharing a prefix so
/// their records collide on one identifier. `load_records` used to iterate
/// `config.record_types` -- a `HashMap`, whose own order is randomized per
/// process -- feeding `build_index_reporting_collisions`' first-seen-wins
/// merge a different candidate order on every run. Empirically confirmed on
/// the pre-fix binary across 20 separate process invocations against this
/// exact fixture: 13 won by `adr`, 7 by `rfc`, on an unchanged corpus. A
/// single test process only ever sees one hash seed, so this can't be
/// observed flipping within one `cargo test` run the way a classic
/// before/after assertion would -- instead this asserts the fix's actual
/// guarantee, which is what makes the flip impossible: `adr` always wins,
/// because `Config::sorted_type_names` makes `records` order
/// type-name-sorted regardless of `record_types`' own hash-map order, and
/// `"adr" < "rfc"` lexicographically.
#[test]
fn a_cross_type_identifier_collision_resolves_deterministically_regardless_of_hashmap_order() {
    let dir = fixture_repo("hashmap-collision-determinism");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::create_dir_all(dir.join("docs/mile")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\n\
         rules:\n  identity.collision: warn\n  pointer.target-status:\n    level: error\n    not_in: [\"Accepted\"]\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Status\"]\n\
         \x20 rfc:\n    dir: \"docs/rfc\"\n    prefix: \"DOC\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Status\"]\n\
         \x20 mile:\n    dir: \"docs/mile\"\n    prefix: \"M\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Status\", \"Derives-from\"]\n    pointer_fields: [\"Derives-from\"]\n    narrative_fields: []\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-a.md"), "> Status: Accepted\n").unwrap();
    std::fs::write(dir.join("docs/rfc/DOC-1-b.md"), "> Status: Draft\n").unwrap();
    std::fs::write(
        dir.join("docs/mile/M-1-c.md"),
        "> Status: Open\n> Derives-from: DOC-1\n",
    )
    .unwrap();
    commit_all(&dir);

    for _ in 0..10 {
        let out = run_urzua(&dir, &["check"]);
        let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let findings = parsed["findings"].as_array().unwrap();
        assert!(
            findings
                .iter()
                .any(|f| f["rule"] == "pointer.target-status"),
            "adr's Status: Accepted must always win the collision, deterministically, every run: {parsed}"
        );
    }
}
