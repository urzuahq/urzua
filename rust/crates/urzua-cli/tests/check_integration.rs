//! CLI-golden integration tests: build a real, isolated git-repo fixture and
//! run the actual compiled binary against it. The fixture lives in its own
//! temp git repo -- entirely outside this project's own discovered record
//! set, so the containment SPEC-0002 requires is true by construction, not
//! assumed.

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

fn fixture_repo(name: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-cli-test-{name}"));
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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let parsed = assert_valid_json_object(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(parsed["scope"]["source"], "tracked-sweep");
}

#[test]
fn check_exits_0_on_a_clean_corpus() {
    let dir = fixture_repo("clean");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Created | **structural** |\n",
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
}

#[test]
fn check_exits_1_on_a_missing_required_field() {
    let dir = fixture_repo("findings");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: [\"Status\", \"Deciders\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deciders"), "stdout: {stdout}");
}

#[test]
fn check_exits_2_when_no_config_exists() {
    let dir = fixture_repo("noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn an_untracked_scratch_file_is_never_examined() {
    // The exact regression this project's own design calls out: a raw
    // filesystem walk would pick this up; git-tracked discovery must not.
    let dir = fixture_repo("scratch");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
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
}

#[test]
fn a_waiver_record_suppresses_blocking_but_the_finding_stays_listed() {
    let dir = fixture_repo("waiver");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/waiver")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\n\
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
        "---\nStatus: Accepted\nImplements: RFC-9999\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Created | **structural** |\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/waiver/0001-w.md"),
        "---\n\
         Rule: pointer.resolution\n\
         Scope: docs/adr/0001-x.md\n\
         Reason: RFC-9999 is tracked externally, not yet in this corpus\n\
         ---\n\
         # 0001 — Waive the dangling RFC-9999 reference on ADR-0001\n\
         \n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Created | **structural** |\n",
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
}

#[test]
fn doctor_reports_missing_config_as_exit_2() {
    let dir = fixture_repo("doctor-noconfig");
    std::fs::write(dir.join("README.md"), "fixture").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["doctor"]);
    assert_eq!(output.status.code(), Some(2));
}

/// Round 30 F4: `doctor` was healthy (exit 0) for a config enabling no rule,
/// where `check` exits 2 not-run.
#[test]
fn doctor_errors_when_no_rule_is_enabled() {
    let dir = fixture_repo("doctor-no-rules");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    commit_all(&dir);
    let output = run_urzua(&dir, &["doctor"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(stdout.contains("rules-enabled"), "stdout: {stdout}");
}

/// Round 30 F1/F2/F5: doctor and check share config load, so an invalid
/// record-type value is reported by both rather than accepted.
#[test]
fn doctor_reports_an_invalid_record_type_prefix_or_dir() {
    for (i, ty) in [
        "  adr:\n    dir: \"docs/adr\"\n    prefix: \"../..\"\n",
        "  adr:\n    dir: \"docs/adr\"\n    prefix: \"P\"\n  w:\n    dir: \"docs/w\"\n    prefix: \"P\"\n",
        "  adr:\n    dir: \"\"\n",
    ]
    .iter()
    .enumerate()
    {
        let dir = fixture_repo(&format!("doctor-bad-type-{i}"));
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            format!("schema_version: 2\nrules: {{field.quality: error}}\nrecord_types:\n{ty}"),
        )
        .unwrap();
        commit_all(&dir);
        let out = run_urzua(&dir, &["doctor"]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(1), "stdout: {stdout}");
        assert!(stdout.contains("config does not parse"), "stdout: {stdout}");
        assert_ne!(run_urzua(&dir, &["check"]).status.code(), Some(0));
    }
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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n\
         \x20 rfc:\n    dir: \"docs/rfc\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/rfc/0001-y.md"),
        "---\nStatus: Draft\n---\n# 0001 — Y\n",
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
}

#[test]
fn audit_exits_0_on_reciprocated_supersession() {
    let dir = fixture_repo("audit-clean");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields:\n      - \"Supersedes / Superseded-by\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nSupersedes / Superseded-by: ADR-0002\n---\n# 0001 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Superseded\nSupersedes / Superseded-by: ADR-0001\n---\n# 0002 — Y\n",
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
}

#[test]
fn audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing() {
    let dir = fixture_repo("audit-violation");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields:\n      - \"Supersedes / Superseded-by\"\n",
    )
    .unwrap();
    // ADR-0001 claims to supersede ADR-0002, but ADR-0002 never points back --
    // exactly the reciprocity violation `audit` exists to catch.
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\nSupersedes / Superseded-by: ADR-0002\n---\n# 0001 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-2-y.md"),
        "---\nStatus: Superseded\n---\n# 0002 — Y\n",
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
}

/// `BUG-42`: `audit` called the cross-record rules directly, so a repository
/// declaring one `off` or `warn` was still blocked by it at the hardcoded
/// level. The gate belongs to the rule registry, so `audit` must honour the
/// declared level exactly as `check` does.
#[test]
fn audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally() {
    let violation =
        "---\nStatus: Accepted\nSupersedes / Superseded-by: ADR-0002\n---\n# 0001 — X\n";
    let target = "---\nStatus: Superseded\n---\n# 0002 — Y\n";
    let run = |name: &str, level: &str| {
        let dir = fixture_repo(name);
        std::fs::create_dir_all(dir.join(".urzua")).unwrap();
        std::fs::write(
            dir.join(".urzua/config.yaml"),
            format!("schema_version: 2\nrules: {{pointer.resolution: warn, relation.supersession-reciprocity: {level}}}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields:\n      - \"Supersedes / Superseded-by\"\n"),
        )
        .unwrap();
        std::fs::write(dir.join("docs/adr/ADR-1-x.md"), violation).unwrap();
        std::fs::write(dir.join("docs/adr/ADR-2-y.md"), target).unwrap();
        commit_all(&dir);
        let output = run_urzua(&dir, &["audit"]);
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        )
    };

    let (code, stdout) = run("audit-gate-off", "off");
    assert_eq!(
        code,
        Some(0),
        "a rule declared off must not block: {stdout}"
    );
    let has_finding = |stdout: &str| {
        assert_valid_json_object(stdout)["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "relation.supersession-reciprocity")
    };
    assert!(
        !has_finding(&stdout),
        "a rule declared off reports nothing: {stdout}"
    );

    let (code, stdout) = run("audit-gate-warn", "warn");
    assert_eq!(
        code,
        Some(0),
        "a rule declared warn must not block: {stdout}"
    );
    assert!(
        has_finding(&stdout),
        "a rule declared warn still reports: {stdout}"
    );
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
        "schema_version: 2\nrules: {identity.collision: error, relation.supersession-reciprocity: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/DOC-1-a.md"),
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-b.md"), "---\nStatus: Draft\n---\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["audit"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(stdout.contains("identity.collision"), "stdout: {stdout}");
}

#[test]
fn init_then_check_names_a_pre_existing_blockquote_record_as_an_unsupported_form() {
    // MILE-85: the blockquote shape is no longer read. Adopt mode proposes
    // yaml-frontmatter, says how many records are in a removed form, and
    // `check` names the form instead of reporting a record with no header.
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
    let init_stdout = String::from_utf8_lossy(&init_output.stdout);
    assert!(
        init_stdout.contains("removed blockquote form"),
        "init must name the removed form: {init_stdout}"
    );
    assert!(dir.join(".urzua/config.yaml").exists());

    let path = dir.join(".urzua/config.yaml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("required_fields: []", "required_fields: [Status]"),
    )
    .unwrap();
    let check_output = run_urzua(&dir, &["check", "docs/"]);
    let stdout = String::from_utf8_lossy(&check_output.stdout);
    assert!(
        stdout.contains("written as a blockquote") && stdout.contains("no longer reads"),
        "check must name the unsupported form: {stdout}"
    );

    // Idempotence: a second init must refuse, not clobber.
    let second_init = run_urzua(&dir, &["init"]);
    assert_eq!(second_init.status.code(), Some(2));
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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
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
        "schema_version: 2\nrules: {}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/DOC-1-a.md"),
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-b.md"), "---\nStatus: Draft\n---\n").unwrap();
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
}

/// `BUG-131`'s primary scope: `check` and `audit` build the collision-aware
/// index for the six rules that resolve a reference against it, but never
/// disclosed a collision when `identity.collision` itself is disabled -- so
/// a real collision silently resolved first-seen-wins with no way to know,
/// exactly the gap `graph`'s sibling copy of this bug already fixed.
#[test]
fn check_discloses_an_identity_collision_as_a_notice_when_the_rule_is_disabled_observed_failing() {
    let dir = fixture_repo("check-collision-notice");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/DOC-1-a.md"),
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-b.md"), "---\nStatus: Draft\n---\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    let notices = parsed["notices"].as_array().unwrap();
    assert!(
        notices.iter().any(|n| n["subject"] == "identity.collision"
            && n["message"].as_str().unwrap().contains("DOC-1")),
        "identity.collision is not-enabled here, but the collision is still real: {stdout}"
    );
}

#[test]
fn audit_discloses_an_identity_collision_as_a_notice_when_the_rule_is_disabled_observed_failing() {
    let dir = fixture_repo("audit-collision-notice");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/rfc")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"DOC\"\n    required_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/DOC-1-a.md"),
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-b.md"), "---\nStatus: Draft\n---\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["audit"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed = assert_valid_json_object(&stdout);
    let notices = parsed["notices"].as_array().unwrap();
    assert!(
        notices.iter().any(|n| n["subject"] == "identity.collision"
            && n["message"].as_str().unwrap().contains("DOC-1")),
        "identity.collision is not-enabled here, but the collision is still real: {stdout}"
    );
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
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
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
}

#[test]
fn init_real_write_omits_config_yaml_from_the_report() {
    let dir = fixture_repo("init-write");
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
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
}

/// BUG-132: `x/y` and `z/y` share a base name and disambiguate to `x-y` and
/// `z-y` -- but an unrelated `x-y` directory already proposes that exact
/// name, and nothing after disambiguation checks for it. `render_config_yaml`
/// silently overwrites one type's entry with the other's, so the config
/// still loads and `init` reports `ok` while an entire directory's records
/// go ungoverned from the first run.
#[test]
fn a_post_disambiguation_name_collision_is_refused_not_silently_dropped_observed_failing() {
    let dir = fixture_repo("init-disambiguation-collision");
    std::fs::create_dir_all(dir.join("x/y")).unwrap();
    std::fs::create_dir_all(dir.join("z/y")).unwrap();
    std::fs::create_dir_all(dir.join("x-y")).unwrap();
    std::fs::write(
        dir.join("x/y/0001-a.md"),
        "---\nStatus: Accepted\n---\n# 0001 — A\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("z/y/0001-b.md"),
        "---\nStatus: Accepted\n---\n# 0001 — B\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("x-y/0001-c.md"),
        "---\nStatus: Accepted\n---\n# 0001 — C\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["init", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(2),
        "a genuine post-disambiguation collision must refuse, not report ok over a dropped type: {stdout}"
    );
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(parsed["status"], "not-run", "stdout: {stdout}");
    assert!(
        !dir.join(".urzua/config.yaml").exists(),
        "must not write a config that would silently drop a proposed type"
    );
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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
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
        "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n",
    )
    .unwrap();
    // A real header-shaped region (a frontmatter block) so the backfill
    // attempts a write, then made read-only so that write fails.
    let record_path = dir.join("docs/adr/0001-x.md");
    std::fs::write(&record_path, "---\nStatus: Accepted\n---\n# 0001 — X\n").unwrap();
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
        "---\nStatus: Accepted\n---\n# 0001 — X\n",
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
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus:\n---\n# 0001 — X\n",
    )
    .unwrap();
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
        "---\nStatus: Accepted\nRealized-by: code:src.rs\n---\n# 0001 — X\n",
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
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let claim_rule = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "claim.status-agreement")
        .unwrap_or_else(|| panic!("claim.status-agreement must appear: {stdout}"));
    assert_eq!(
        claim_rule["population"]["examined"], 1,
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

fn one_adr_repo(name: &str, rules: &str, body: &str) -> common::TempDir {
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
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
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
    // `MILE-120`: a bare-number filename is something wrong, and `audit` now
    // says so with the expected name (a warning, so the exit stays 0).
    let audit_out = run_urzua(&dir, &["audit"]);
    let audit_stdout = String::from_utf8_lossy(&audit_out.stdout).to_string();
    assert!(
        audit_stdout.contains("\"status\": \"findings-present\"")
            && audit_stdout.contains("0001-x.md -> ADR-0001-x.md"),
        "the bare filename is reported with its expected name: {audit_stdout}"
    );
    assert_eq!(
        audit_out.status.code(),
        Some(0),
        "nothing is actually wrong with this corpus: {audit_stdout}"
    );
}

/// `BUG-73`: the config `init` writes must not enable a rule that reads a
/// declaration `init` left empty -- it would report `ran` over zero records --
/// and `init` says which declaration turns each one on.
#[test]
fn init_does_not_propose_rules_that_would_examine_nothing_on_the_corpus_it_adopted() {
    let dir = fixture_repo("init-withholds-inert-rules");
    std::fs::write(
        dir.join("docs/adr/ADR-0001-x.md"),
        "---\nStatus: Accepted\n---\n# 1 — X\n",
    )
    .unwrap();
    commit_all(&dir);

    let init = run_urzua(&dir, &["init"]);
    let init_stdout = String::from_utf8_lossy(&init.stdout).to_string();
    assert!(init.status.success(), "{init_stdout}");
    for needle in [
        "field.quality",
        "required_fields",
        "known_fields",
        "pointer_fields",
    ] {
        assert!(
            init_stdout.contains(needle),
            "the notice must name {needle}: {init_stdout}"
        );
    }
    commit_all(&dir);

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    for rule in [
        "field.quality",
        "field.pending",
        "header.field-set-consistency",
        "header.pointer-field-clean",
    ] {
        let execution = parsed["rules_executed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule"] == rule)
            .unwrap_or_else(|| panic!("{rule} missing: {stdout}"));
        assert_eq!(
            execution["status"], "not-enabled",
            "{rule} must not be enabled by init: {stdout}"
        );
    }
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
fn a_record_with_no_revision_log_is_a_finding_when_its_type_declares_one_required() {
    // BUG-50: "absence is indistinguishable from compliance". SPEC-1 lost 18
    // revision rows by losing one marker line and the rule stayed green. A type
    // declaring `revision_log: {required: true}` now reports a record
    // with no log, and the finding offers the ADR-73 `edit-section` `add` action.
    let dir = one_adr_repo(
        "revision-log-absent",
        "{revision-log.change-class-required: error}",
        "---\nStatus: Accepted\n---\n# 1 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Created | **structural** |\n",
    );
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{config}    revision_log: {{required: true}}\n"),
    )
    .unwrap();
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
        pop["examined"], 2,
        "the record with no log is judged: {stdout}"
    );
    let findings: Vec<&serde_json::Value> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "revision-log.change-class-required")
        .collect();
    assert_eq!(findings.len(), 1, "{stdout}");
    assert!(findings[0]["file"]
        .as_str()
        .unwrap()
        .ends_with("ADR-2-y.md"));
    let action = &findings[0]["suggested_action"];
    assert_eq!(action["kind"], "edit-section", "{stdout}");
    assert_eq!(action["op"], "add", "{stdout}");
    assert_eq!(action["target"]["section"], "**Revision log**", "{stdout}");
}

#[test]
fn a_record_with_no_revision_log_stays_unexamined_when_its_type_declares_required_false() {
    let dir = one_adr_repo(
        "revision-log-not-required",
        "{revision-log.change-class-required: error}",
        "---\nStatus: Accepted\n---\n# 1 — X\n\nNo log here.\n",
    );
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!("{config}    revision_log: {{required: false}}\n"),
    )
    .unwrap();
    commit_all(&dir);

    let stdout = String::from_utf8_lossy(&run_urzua(&dir, &["check"]).stdout).to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rule = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "revision-log.change-class-required")
        .unwrap();
    assert_eq!(rule["population"]["eligible"], 1, "{stdout}");
    assert_eq!(rule["population"]["examined"], 0, "{stdout}");
    assert!(
        parsed["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["rule"] != "revision-log.change-class-required"),
        "{stdout}"
    );
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
        "{header.field-set-consistency: warn}",
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
        .find(|r| r["rule"] == "header.field-set-consistency")
        .and_then(|r| r.get("population"))
        .expect("the rule must carry a population");
    assert_eq!(
        pop["eligible"], 0,
        "no type declares known_fields: {stdout}"
    );
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
        "---\nStatus: Accepted\n---\n# 1 — X\n",
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
            format!("---\nStatus: Accepted\n---\n# {n} — X\n"),
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
        "---\nStatus: Accepted\n---\n# 1 — X\n",
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
        "---\nStatus: Accepted\n---\n# 1 — X\n",
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
        eprintln!(
            "skipped: permission bits did not block access in this environment (running as root?)"
        );
        return;
    }

    let out = run_urzua(&dir, &["check"]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

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
        "---\nPhase: Implemented\nEvidence: code:src.rs\n---\n# 0001 — X\n",
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
        "---\nEmbodiment: Implemented\nRealized-by: code:shared.rs\n---\n# 0001 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/0002-y.md"),
        "---\nEmbodiment: Implemented\nRealized-by: code:shared.rs\n---\n# 0002 — Y\n",
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
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
    // A live sibling, left in place: proves the filter drops only the staged
    // deletion, not every candidate below `docs/adr/archive` (a positive
    // control CodeRabbit's review of round 21 asked for).
    std::fs::write(
        dir.join("docs/adr/archive/0002-y.md"),
        "---\nStatus: Accepted\n---\n",
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

/// Originally a found-in-review bug with two record types (`adr`/`rfc`)
/// sharing a prefix so their records collided on one identifier. Config load
/// now refuses a shared prefix (Round 30), so the same first-seen-wins
/// guarantee is exercised with two same-type records; the history below
/// describes the original cross-type form. `load_records` used to iterate
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
fn a_same_type_identifier_collision_resolves_deterministically() {
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
         \x20 mile:\n    dir: \"docs/mile\"\n    prefix: \"M\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Status\", \"Derives-from\"]\n    pointer_fields: [\"Derives-from\"]\n    narrative_fields: []\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/DOC-1-a.md"),
        "---\nStatus: Accepted\n---\n",
    )
    .unwrap();
    std::fs::write(dir.join("docs/adr/DOC-1-b.md"), "---\nStatus: Draft\n---\n").unwrap();
    std::fs::write(
        dir.join("docs/mile/M-1-c.md"),
        "---\nStatus: Open\nDerives-from: DOC-1\n---\n",
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

/// MILE-85: an adopted corpus in a removed shape is proposed the only shape
/// with a reader, and `init` says how many records it will not read.
#[test]
fn init_on_a_bold_list_corpus_proposes_yaml_frontmatter_and_names_the_removed_form() {
    let dir = fixture_repo("init-bold-list");
    for n in 1..=3 {
        std::fs::write(
            dir.join(format!("docs/adr/{n:04}-record-{n}.md")),
            format!(
                "# {n} — Record {n}\n\n- **Status:** Accepted\n- **Date:** 2026-01-0{n}\n\nBody.\n"
            ),
        )
        .unwrap();
    }
    commit_all(&dir);

    let init = run_urzua(&dir, &["init"]);
    let init_stdout = String::from_utf8_lossy(&init.stdout);
    assert_eq!(init.status.code(), Some(0), "stdout: {init_stdout}");
    let parsed = assert_valid_json_object(&init_stdout);
    assert_eq!(
        parsed["proposed"][0]["header_shape"], "yaml-frontmatter",
        "stdout: {init_stdout}"
    );
    let notices = parsed["notices"].as_array().expect("a notice names it");
    assert!(
        notices.iter().any(|n| {
            let m = n["message"].as_str().unwrap_or("");
            m.contains("docs/adr") && m.contains("3 in the removed bold-list form")
        }),
        "stdout: {init_stdout}"
    );
    let config = std::fs::read_to_string(dir.join(".urzua/config.yaml")).unwrap();
    assert!(
        config.contains("header_shape: yaml-frontmatter"),
        "{config}"
    );
}

/// MILE-85: records in both removed forms are each counted in the notice.
#[test]
fn init_on_a_mixed_removed_shape_type_counts_each_form() {
    let dir = fixture_repo("init-mixed-shape");
    for n in 1..=2 {
        std::fs::write(
            dir.join(format!("docs/adr/{n:04}-bold-{n}.md")),
            "# T\n\n- **Status:** Accepted\n\nBody.\n",
        )
        .unwrap();
    }
    std::fs::write(
        dir.join("docs/adr/0003-quote.md"),
        "# T\n\n> Status: Accepted\n\nBody.\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["init", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(
        parsed["proposed"][0]["header_shape"], "yaml-frontmatter",
        "stdout: {stdout}"
    );
    let notices = parsed["notices"]
        .as_array()
        .expect("a split needs a notice");
    assert!(
        notices.iter().any(|n| {
            let m = n["message"].as_str().unwrap_or("");
            m.contains("docs/adr")
                && m.contains("2 in the removed bold-list form")
                && m.contains("1 in the removed blockquote form")
        }),
        "stdout: {stdout}"
    );
}

/// BUG-151: records that parse under no shape keep the destination shape,
/// with a notice rather than a silent default.
#[test]
fn init_on_records_with_no_readable_header_keeps_yaml_frontmatter_and_says_so() {
    let dir = fixture_repo("init-no-header");
    std::fs::write(
        dir.join("docs/adr/0001-x.md"),
        "# 0001 — X\n\nProse only.\n",
    )
    .unwrap();
    commit_all(&dir);

    let out = run_urzua(&dir, &["init", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed = assert_valid_json_object(&stdout);
    assert_eq!(
        parsed["proposed"][0]["header_shape"], "yaml-frontmatter",
        "stdout: {stdout}"
    );
    assert!(
        parsed["notices"]
            .as_array()
            .map(|a| a
                .iter()
                .any(|n| n["message"].as_str().unwrap_or("").contains("docs/adr")))
            .unwrap_or(false),
        "stdout: {stdout}"
    );
}

/// BUG-151 review: valid frontmatter at the start of the document is the
/// header even when body bullets carry more fields.
#[test]
fn init_prefers_leading_frontmatter_over_more_numerous_body_bullets() {
    let dir = fixture_repo("init-frontmatter-vs-bullets");
    for n in 1..=2 {
        std::fs::write(
            dir.join(format!("docs/adr/{n:04}-record-{n}.md")),
            format!(
                "---\nStatus: Accepted\n---\n# {n} — Record {n}\n\n- **Owner:** someone\n- **Area:** a thing\n\nBody.\n"
            ),
        )
        .unwrap();
    }
    commit_all(&dir);

    let init = run_urzua(&dir, &["init"]);
    let init_stdout = String::from_utf8_lossy(&init.stdout);
    let parsed = assert_valid_json_object(&init_stdout);
    assert_eq!(
        parsed["proposed"][0]["header_shape"], "yaml-frontmatter",
        "stdout: {init_stdout}"
    );

    let path = dir.join(".urzua/config.yaml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("required_fields: []", "required_fields: [Status]"),
    )
    .unwrap();
    let check = run_urzua(&dir, &["check", "docs/"]);
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(
        !stdout.contains("missing required header field"),
        "stdout: {stdout}"
    );
}

const EDIT_CONFIG: &str = "schema_version: 2\nrules: {header.required-fields: error, header.field-set-consistency: warn, header.pointer-field-clean: warn, type.no-declared-spec: warn, config.pointer-declaration-missing: error, config.pointer-field-not-known: error, config.pointer-narrative-overlap: error, pointer.resolution: error, field.quality: error, field.pending: warn, filename.title-consistency: error, relation.supersession-reciprocity: error, revision-log.change-class-required: error, embodiment.consistency: warn, embodiment.locator-promotion-candidate: warn}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n";

fn edit_fixture(name: &str, record: &str) -> common::TempDir {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(dir.join(".urzua/config.yaml"), EDIT_CONFIG).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-first.md"), record).unwrap();
    commit_all(&dir);
    dir
}

/// A CRLF record with no trailing newline: `fix --apply` must change the one
/// field line and add one revision-log row, nothing else -- not rewrite the
/// whole file to LF with a trailing newline.
#[test]
fn fix_apply_preserves_crlf_and_the_absent_trailing_newline() {
    let original = "---\r\nEmbodiment: Not started\r\nRealized-by: code:src/a.rs\r\n---\r\n# ADR-1: First\r\n\r\n> **Revision log**\r\n>\r\n> | Date | Change | Class |\r\n> |---|---|---|\r\n> | 2026-01-01 | Initial. | **structural** |";
    let dir = edit_fixture("fix-crlf", original);

    let output = run_urzua(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let after = std::fs::read_to_string(dir.join("docs/adr/ADR-1-first.md")).unwrap();
    let before_lines: Vec<&str> = original.split("\r\n").collect();
    let after_lines: Vec<&str> = after.split("\r\n").collect();
    assert_eq!(after_lines.len(), before_lines.len() + 1, "{after:?}");
    assert!(
        !after.replace("\r\n", "").contains('\n'),
        "bare LF: {after:?}"
    );
    assert!(!after.ends_with('\n'), "trailing newline added: {after:?}");
    let changed = after_lines
        .iter()
        .filter(|l| !before_lines.contains(l))
        .count();
    assert_eq!(changed, 2, "only the field line and the new row: {after:?}");
}

/// A plain (non-blockquote) revision-log table: the appended row must be a
/// plain table row too, or it renders outside the table.
#[test]
fn fix_apply_appends_inside_a_plain_revision_log_table() {
    let original = "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: First\n\n**Revision log**\n\n| Date | Change | Class |\n|---|---|---|\n| 2026-01-01 | Initial. | **structural** |\n";
    let dir = edit_fixture("fix-plain-table", original);

    let output = run_urzua(&dir, &["fix", "--apply", "--force", "--by", "me"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let after = std::fs::read_to_string(dir.join("docs/adr/ADR-1-first.md")).unwrap();
    let last = after.lines().last().unwrap();
    assert!(
        last.starts_with("| ") && last.contains("Tool-authored"),
        "new row must be a plain table row: {after:?}"
    );
}

/// `migrate ids --apply` on a CRLF record with no trailing newline inserts
/// exactly one line and leaves every other byte alone.
#[test]
fn migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline() {
    let original = "---\r\nStatus: Draft\r\nDate: 2026-01-01\r\n---\r\n# ADR-1: First\r\n\r\nbody\r\nno trailing newline";
    let dir = edit_fixture("migrate-crlf", original);

    let output = run_urzua(&dir, &["migrate", "ids", "--apply"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let after = std::fs::read_to_string(dir.join("docs/adr/ADR-1-first.md")).unwrap();
    let kept: Vec<&str> = after
        .split("\r\n")
        .filter(|l| !l.contains("Stable-Id:"))
        .collect();
    assert_eq!(kept.join("\r\n"), original, "{after:?}");
    assert_eq!(after.matches("Stable-Id:").count(), 1, "{after:?}");
}

/// An `--ids` entry that selects nothing is a reported failure (exit 1)
/// naming the entry, not a clean `ok` with nothing written.
#[test]
fn fix_apply_ids_that_match_no_repair_is_a_failure_naming_the_selector() {
    let original = "---\nEmbodiment: Not started\nRealized-by: code:src/a.rs\n---\n# ADR-1: First\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n";
    let dir = edit_fixture("fix-ids-unmatched", original);

    for selector in ["NOPE", "ADR-1"] {
        let output = run_urzua(&dir, &["fix", "--apply", "--ids", selector, "--by", "me"]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = assert_valid_json_object(&stdout);
        assert_eq!(parsed["status"], "partial-failure", "{selector}: {stdout}");
        assert_eq!(output.status.code(), Some(1), "{selector}: {stdout}");
        assert_eq!(parsed["failed"][0]["record"], selector, "{stdout}");
    }
    let after = std::fs::read_to_string(dir.join("docs/adr/ADR-1-first.md")).unwrap();
    assert_eq!(after, original, "an unmatched selector must write nothing");

    // The documented path form still applies, with no failure.
    let output = run_urzua(
        &dir,
        &[
            "fix",
            "--apply",
            "--ids",
            "docs/adr/ADR-1-first.md",
            "--by",
            "me",
        ],
    );
    let parsed = assert_valid_json_object(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(parsed["status"], "applied");
    assert_eq!(output.status.code(), Some(0));
}

/// `MILE-119`: every rule that builds a per-record or per-slot population must
/// classify a `header_shape: none` type (`ADR-50`) as out of scope -- it has no
/// header, so "unreadable" would misreport a deliberate declaration as a
/// malformed file. The rule set is `RULE_TABLE`, not a hand-picked list, so a
/// rule added later is covered without anyone remembering to extend this test.
///
/// A rule is exempt only if it cannot read a header at all, and its table entry
/// (`RuleDef::header_none`) says why. Everything else must show `eligible > 0` (a rule with no candidate
/// proves nothing), `examined == 0` and `unreadable == 0`, and every candidate
/// out of scope.
#[test]
fn every_header_reading_rule_reports_a_header_none_type_as_out_of_scope() {
    let dir = fixture_repo("header-none-all-rules");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/note")).unwrap();
    let rules: Vec<String> = urzua_core::rules::ALL_RULES
        .iter()
        .map(|id| match *id {
            // These rules refuse to load without their own option.
            "claim.status-agreement" => format!(
                "{id}: {{level: error, claim_paths: [\"docs/claims\"], closed_statuses: [\"Done\"]}}"
            ),
            "narrative-field.stale" => {
                format!("{id}: {{level: error, terminal_statuses: [\"Done\"]}}")
            }
            "pointer.target-status" => format!("{id}: {{level: error, not_in: [\"Done\"]}}"),
            "boundary.published-path-reference" => {
                format!("{id}: {{level: error, published_paths: [\"docs/claims\"]}}")
            }
            "index.freshness" => format!(
                "{id}: {{level: error, indexes: [{{path: \"docs/claims/c.md\", type: note, format: \"{{id}}\"}}]}}"
            ),
            "diff.source-requires-record" => {
                format!("{id}: {{level: error, source_paths: [\"src\"]}}")
            }
            "diff.source-and-records-mixed" => {
                format!("{id}: {{level: error, application_paths: [\"src\"]}}")
            }
            "embodiment.back-pointer-resolution"
            | "embodiment.back-pointer-decision-accepted" => {
                format!("{id}: {{level: error, back_pointer_paths: [\"docs/claims\"]}}")
            }
            "date.field-valid" => format!("{id}: {{level: error, date_fields: [\"Date\"]}}"),
            "date.field-order" => {
                format!("{id}: {{level: error, date_order: [{{earlier: Date, later: Accepted-on}}]}}")
            }
            "citation.path-exists" => {
                format!("{id}: {{level: error, reference_sections: [\"References\"]}}")
            }
            "citation.issue-exists" => {
                format!("{id}: {{level: error, reference_sections: [\"References\"], issue_refs: {{snapshot: \".urzua/known-refs.json\", repository: \"o/r\", forms: [number]}}}}")
            }
            _ => format!("{id}: error"),
        })
        .collect();
    std::fs::create_dir_all(dir.join("docs/claims")).unwrap();
    std::fs::write(dir.join("docs/claims/c.md"), "nothing claimed\n").unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {{{}}}\n\nrelation_pairs:\n  - {{name: follow, forward: Follows, inverse: Followed-by, reciprocity: required, cross_type: false, acyclic: true}}\n\nrecord_types:\n  note:\n    dir: \"docs/note\"\n    prefix: \"NOTE\"\n    header_shape: none\n    required_fields: [\"Status\"]\n    known_fields: [\"Parent\", \"Ref\", \"Date\", \"Accepted-on\", \"Embodiment\", \"Realized-by\", \"Supersedes / Superseded-by\", \"Follows\", \"Followed-by\"]\n    pointer_fields: [\"Parent\"]\n    narrative_fields: [\"Ref\"]\n    revision_log: {{version_bump: {{field: Version, classes: [substantive]}}}}\n    statuses: {{values: [Open, Done], success: [Done], initial: [Open], transitions: {{Open: [Done]}}, delivery: {{delivered_tiers: [Implemented], unevidenced_tiers: [\"Not started\"], not_done: [Open], set_status: Done}}, require: [{{when: [Done], fields: [Ref]}}], forbid: [{{when: [Open], fields: [Parent]}}]}}\n",
            rules.join(", ")
        ),
    )
    .unwrap();
    std::fs::write(dir.join("docs/note/NOTE-1-x.md"), "# 1 — X\n\nA body.\n").unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "check must print JSON ({e}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let executed = parsed["rules_executed"].as_array().unwrap();

    let mut failures = Vec::new();
    for def in urzua_core::rules::RULE_TABLE {
        let id = &def.id;
        if matches!(def.header_none, urzua_core::rules::HeaderNone::Exempt(_)) {
            continue;
        }
        let Some(entry) = executed.iter().find(|r| r["rule"] == *id) else {
            failures.push(format!("{id}: absent from rules_executed"));
            continue;
        };
        let pop = &entry["population"];
        let n = |k: &str| pop[k].as_u64();
        let (eligible, examined, oos, unreadable) = (
            n("eligible"),
            n("examined"),
            n("out_of_scope"),
            n("unreadable"),
        );
        if !(eligible.is_some_and(|e| e > 0)
            && examined == Some(0)
            && unreadable == Some(0)
            && oos == eligible)
        {
            failures.push(format!("{id}: population {pop}"));
        }
    }
    assert!(
        failures.is_empty(),
        "rules that do not classify a header_shape: none type as out of scope:\n{}",
        failures.join("\n")
    );
}

/// BUG-172: `pointer.target-status` applied one `not_in` list to every declared
/// pointer field, so a lineage field (`Derives-from`) pointing at a Superseded
/// record warned the same as a dependency field (`Implements`). The `fields`
/// option restricts which declared fields are judged.
fn target_status_fields_fixture(name: &str, option: &str) -> common::TempDir {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/mile")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\n\
             rules:\n  pointer.target-status:\n    level: warn\n    not_in: [\"Superseded\"]\n{option}\n\
             record_types:\n\
             \x20 adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Status\"]\n\
             \x20 mile:\n    dir: \"docs/mile\"\n    prefix: \"M\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Status\", \"Derives-from\", \"Implements\"]\n    pointer_fields: [\"Derives-from\", \"Implements\"]\n    narrative_fields: []\n",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-a.md"),
        "---\nStatus: Superseded\n---\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/mile/M-1-b.md"),
        "---\nStatus: Open\nDerives-from: ADR-1\nImplements: ADR-1\n---\n",
    )
    .unwrap();
    commit_all(&dir);
    dir
}

fn target_status_messages(dir: &Path) -> Vec<String> {
    let out = run_urzua(dir, &["check"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stderr)));
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "pointer.target-status")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn target_status_fields_restricts_which_declared_fields_warn_observed_failing() {
    let dir = target_status_fields_fixture(
        "target-status-fields-restricted",
        "    fields: [\"Implements\"]",
    );
    let messages = target_status_messages(&dir);
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].starts_with("Implements:"), "{messages:?}");
}

#[test]
fn target_status_without_fields_warns_on_every_declared_field() {
    let dir = target_status_fields_fixture("target-status-fields-omitted", "");
    let messages = target_status_messages(&dir);
    assert_eq!(messages.len(), 2, "{messages:?}");
}

#[test]
fn target_status_fields_naming_an_undeclared_field_is_a_config_error_observed_failing() {
    let dir = target_status_fields_fixture(
        "target-status-fields-undeclared",
        "    fields: [\"Implments\"]",
    );
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("Implments"), "{text}");
}

fn filename_pattern_repo(name: &str, files: &[&str], rules: &str) -> common::TempDir {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n"
        ),
    )
    .unwrap();
    for f in files {
        std::fs::write(
            dir.join("docs/adr").join(f),
            "---\nStatus: Accepted\n---\n# 1 - X\n",
        )
        .unwrap();
    }
    commit_all(&dir);
    dir
}

/// `MILE-120`: a bare-number corpus is one finding per type carrying every
/// expected name, and the census says what was examined.
#[test]
fn a_bare_number_corpus_is_reported_once_per_type_with_rename_guidance() {
    let dir = filename_pattern_repo(
        "filename-bare",
        &["0001-a.md", "0002-b.md"],
        "{filename.identity-pattern: warn}",
    );
    let out = run_urzua(&dir, &["check"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let hits: Vec<_> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "filename.identity-pattern")
        .collect();
    assert_eq!(hits.len(), 1, "{parsed}");
    let m = hits[0]["message"].as_str().unwrap();
    assert!(m.contains("0001-a.md -> ADR-0001-a.md") && m.contains("0002-b.md -> ADR-0002-b.md"));
    let exec = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "filename.identity-pattern")
        .unwrap();
    assert_eq!(exec["population"]["examined"], 2);
}

#[test]
fn the_filename_pattern_rule_is_opt_in() {
    let dir = filename_pattern_repo(
        "filename-optin",
        &["0001-a.md", "ADR-2-b.md"],
        "{header.required-fields: warn}",
    );
    let out = run_urzua(&dir, &["check"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["rule"] != "filename.identity-pattern"));
}

#[test]
fn one_off_pattern_file_among_conforming_ones_is_reported_on_that_file() {
    let dir = filename_pattern_repo(
        "filename-one",
        &["ADR-1-a.md", "draft.md"],
        "{filename.identity-pattern: error}",
    );
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(1), "an error-level finding blocks");
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let f = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["rule"] == "filename.identity-pattern")
        .unwrap();
    assert_eq!(f["file"], "docs/adr/draft.md");
}

/// `MILE-132`: a declared status vocabulary is checked end to end through
/// `check`: an undeclared value, a field a status requires and a field a status
/// forbids, each reported on the offending record only.
#[test]
fn declared_statuses_are_enforced_end_to_end() {
    let dir = fixture_repo("status-vocabulary");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {status.undeclared-value: error, status.required-field-missing: error, status.forbidden-field-present: error}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    known_fields: [\"Deciders\", \"Accepted-on\"]\n    header_shape: \"yaml-frontmatter\"\n    statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n      require: [{when: [Accepted], fields: [Deciders]}]\n      forbid: [{when: [Proposed], fields: [Accepted-on]}]\n",
    )
    .unwrap();
    for (name, header) in [
        ("ADR-1-ok.md", "Status: Accepted\nDeciders: Ana\n"),
        ("ADR-2-no-deciders.md", "Status: Accepted\n"),
        ("ADR-3-typo.md", "Status: Acceptd\n"),
        (
            "ADR-4-early-date.md",
            "Status: Proposed\nAccepted-on: 2026-01-01\n",
        ),
    ] {
        std::fs::write(
            dir.join("docs/adr").join(name),
            format!("---\n{header}---\n# 1 - X\n"),
        )
        .unwrap();
    }
    commit_all(&dir);
    let out = run_urzua(&dir, &["check"]);
    assert_eq!(out.status.code(), Some(1), "error-level findings block");
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let hits = |rule: &str| -> Vec<String> {
        parsed["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["rule"] == rule)
            .map(|f| f["file"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(hits("status.undeclared-value"), ["docs/adr/ADR-3-typo.md"]);
    assert_eq!(
        hits("status.required-field-missing"),
        ["docs/adr/ADR-2-no-deciders.md"]
    );
    assert_eq!(
        hits("status.forbidden-field-present"),
        ["docs/adr/ADR-4-early-date.md"]
    );
    let exec = parsed["rules_executed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["rule"] == "status.undeclared-value")
        .unwrap();
    assert_eq!(exec["population"]["examined"], 4);
}

/// `MILE-122`: a declared relation pair is checked end to end through `check`,
/// across two record types, from config alone.
#[test]
fn check_reports_an_unechoed_claim_in_a_declared_cross_type_relation_pair() {
    let dir = fixture_repo("relation-pair-e2e");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::create_dir_all(dir.join("docs/bug")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {relation.pair-reciprocity: error}\n\nrelation_pairs:\n  - {name: resolution, forward: Resolves, inverse: Resolved-by, reciprocity: required}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    known_fields: [\"Resolves\"]\n  bug:\n    dir: \"docs/bug\"\n    required_fields: []\n    known_fields: [\"Resolved-by\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nResolves: BUG-7\n---\n# 1 — X\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/bug/BUG-7-y.md"),
        "---\nResolved-by: —\n---\n# 7 — Y\n",
    )
    .unwrap();
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    assert!(stdout.contains("relation.pair-reciprocity"), "{stdout}");
    assert!(stdout.contains("Resolved-by"), "{stdout}");

    std::fs::write(
        dir.join("docs/bug/BUG-7-y.md"),
        "---\nResolved-by: ADR-1\n---\n# 7 — Y\n",
    )
    .unwrap();
    commit_all(&dir);
    let output = run_urzua(&dir, &["check", "docs/"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// MILE-137: the acceptance date is required on Accepted and forbidden on
/// Proposed through the declared `statuses.require` / `statuses.forbid`, and
/// `date.field-valid` owns its well-formedness. One mechanism, no dedicated rule.
#[test]
fn the_acceptance_date_by_status_is_expressed_with_statuses_require_forbid_and_date_field_valid() {
    let dir = fixture_repo("acceptance-date-by-status");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {status.required-field-missing: error, status.forbidden-field-present: error, date.field-valid: {level: error, date_fields: [Accepted-on]}}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    header_shape: yaml-frontmatter\n    required_fields: [\"Status\"]\n    known_fields: [\"Accepted-on\"]\n    statuses: {values: [Proposed, Accepted], success: [Accepted], require: [{when: [Accepted], fields: [Accepted-on]}], forbid: [{when: [Proposed], fields: [Accepted-on]}]}\n",
    )
    .unwrap();
    let write = |name: &str, status: &str, extra: &str| {
        std::fs::write(
            dir.join(format!("docs/adr/{name}.md")),
            format!("---\nStatus: {status}\n{extra}---\n# X\n"),
        )
        .unwrap();
    };
    write("ADR-1-accepted-without-date", "Accepted", "");
    write(
        "ADR-2-accepted-with-date",
        "Accepted",
        "Accepted-on: 2026-01-01\n",
    );
    write(
        "ADR-3-proposed-with-date",
        "Proposed",
        "Accepted-on: 2026-01-01\n",
    );
    write("ADR-4-proposed-without-date", "Proposed", "");
    write(
        "ADR-5-accepted-bad-date",
        "Accepted",
        "Accepted-on: 2026-02-30\n",
    );
    commit_all(&dir);

    let output = run_urzua(&dir, &["check", "docs/"]);
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut hits: Vec<(String, String)> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["rule"].as_str().unwrap().to_string(),
                f["file"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    hits.sort();
    let hit = |rule: &str, file: &str| hits.iter().any(|(r, f)| r == rule && f.contains(file));
    assert!(hit("status.required-field-missing", "ADR-1-"), "{parsed}");
    assert!(hit("status.forbidden-field-present", "ADR-3-"), "{parsed}");
    assert!(hit("date.field-valid", "ADR-5-"), "{parsed}");
    assert_eq!(hits.len(), 3, "no other record may be flagged: {parsed}");
    assert_eq!(parsed["blocking"], true);
    assert_eq!(output.status.code(), Some(1));
}

fn excluded_files_repo(name: &str, exclude: &str, files: &[&str], rules: &str) -> common::TempDir {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules: {rules}\n\nrecord_types:\n  adr:\n    dir: \"docs/adr\"\n    prefix: \"ADR\"\n    required_fields: [\"Status\"]\n    header_shape: \"yaml-frontmatter\"\n{exclude}"
        ),
    )
    .unwrap();
    for f in files {
        let content = if *f == "README.md" {
            "# Architecture decisions\n\nAn index.\n".to_string()
        } else {
            "---\nStatus: Accepted\n---\n# ADR-1 - X\n".to_string()
        };
        std::fs::write(dir.join("docs/adr").join(f), content).unwrap();
    }
    commit_all(&dir);
    dir
}

fn check_json(dir: &Path) -> serde_json::Value {
    let out = run_urzua(dir, &["check"]);
    serde_json::from_slice(&out.stdout).expect("JSON on stdout")
}

fn findings_for<'a>(parsed: &'a serde_json::Value, file: &str) -> Vec<&'a serde_json::Value> {
    parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["file"] == file)
        .collect()
}

/// `MILE-150`/`BUG-183`: a declared exclusion takes a named non-record file out
/// of every rule's population and the type's file census.
#[test]
fn a_declared_non_record_file_is_in_no_rule_population_or_census_observed_failing() {
    let rules = "{header.required-fields: error, filename.identity-pattern: warn}";
    let dir = excluded_files_repo(
        "exclude-declared",
        "    exclude_files: [\"README.md\"]\n",
        &["ADR-1-a.md", "README.md"],
        rules,
    );
    let parsed = check_json(&dir);
    assert!(
        findings_for(&parsed, "docs/adr/README.md").is_empty(),
        "{parsed}"
    );
    for exec in parsed["rules_executed"].as_array().unwrap() {
        if exec["rule"] == "filename.identity-pattern" || exec["rule"] == "header.required-fields" {
            assert_eq!(exec["population"]["examined"], 1, "{exec}");
        }
    }
}

#[test]
fn an_undeclared_readme_is_still_reported_and_counted() {
    let rules = "{header.required-fields: error, filename.identity-pattern: warn}";
    let dir = excluded_files_repo(
        "exclude-undeclared",
        "",
        &["ADR-1-a.md", "README.md"],
        rules,
    );
    let parsed = check_json(&dir);
    assert!(
        !findings_for(&parsed, "docs/adr/README.md").is_empty(),
        "{parsed}"
    );
}

#[test]
fn an_undeclared_stray_file_is_reported_beside_a_declared_exclusion() {
    let rules = "{filename.identity-pattern: warn}";
    let dir = excluded_files_repo(
        "exclude-stray",
        "    exclude_files: [\"README.md\"]\n",
        &["ADR-1-a.md", "README.md", "notes.md"],
        rules,
    );
    let parsed = check_json(&dir);
    assert!(
        findings_for(&parsed, "docs/adr/README.md").is_empty(),
        "{parsed}"
    );
    assert_eq!(
        findings_for(&parsed, "docs/adr/notes.md").len(),
        1,
        "{parsed}"
    );
}

/// `MILE-150`: an exclusion that matches no file is a stale declaration, so it
/// is reported (no silent no-op); one that matches is not.
#[test]
fn an_exclude_files_entry_matching_no_file_is_reported_observed_failing() {
    let rules = "{type.dir-matches-nothing: warn}";
    let dir = excluded_files_repo(
        "exclude-stale",
        "    exclude_files: [\"README.md\", \"_template.md\"]\n",
        &["ADR-1-a.md", "README.md"],
        rules,
    );
    let parsed = check_json(&dir);
    let hits: Vec<_> = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "type.dir-matches-nothing")
        .collect();
    assert_eq!(hits.len(), 1, "{parsed}");
    assert_eq!(hits[0]["severity"], "warning", "{parsed}");
    assert_eq!(hits[0]["file"], ".urzua/config.yaml", "{parsed}");
    let m = hits[0]["message"].as_str().unwrap();
    assert!(m.contains("_template.md") && !m.contains("README"), "{m}");
}

/// A type whose only other file is the excluded one has no record to match, so
/// the rule must not report "none of the type's 1 file(s) matches".
#[test]
fn an_excluded_file_is_not_counted_in_the_all_off_pattern_total() {
    let rules = "{filename.identity-pattern: warn}";
    let dir = excluded_files_repo(
        "exclude-census",
        "    exclude_files: [\"README.md\"]\n",
        &["README.md", "0001-a.md"],
        rules,
    );
    let parsed = check_json(&dir);
    let all: Vec<_> = parsed["findings"].as_array().unwrap().iter().collect();
    assert_eq!(all.len(), 1, "{parsed}");
    assert_eq!(all[0]["file"], "docs/adr", "{parsed}");
    let m = all[0]["message"].as_str().unwrap();
    assert!(m.contains("1 file(s)") && !m.contains("README"), "{m}");
}

/// `BUG-185`: a config-level finding names the configuration as a
/// repository-relative path, like every other finding.
#[test]
fn a_config_level_finding_names_a_repository_relative_config_path_observed_failing() {
    let dir = excluded_files_repo(
        "config-relative",
        "",
        &["0001-a.md"],
        "{identity.collision: error, config.scope-matches-nothing: warn}",
    );
    let parsed = check_json(&dir);
    let hits = parsed["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule"] == "config.scope-matches-nothing")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 1, "{parsed}");
    assert_eq!(hits[0]["file"], ".urzua/config.yaml", "{parsed}");
}

// ---- MILE-179: doctor reports each local hook layer's wiring (ADR-71) ----

fn hook_fixture(name: &str) -> common::TempDir {
    let dir = fixture_repo(name);
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    dir
}

fn plant_hook(path: &Path, body: &str, executable: bool) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
    let mode = if executable { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// (state, status) of the hook check for one layer.
fn hook_state(dir: &Path, layer: &str) -> (String, String) {
    let out = run_urzua(dir, &["doctor"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let c = parsed["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "hook-wired" && c["layer"] == layer)
        .unwrap_or_else(|| panic!("no hook-wired check for {layer}: {parsed}"));
    (
        c["state"].as_str().unwrap().to_string(),
        c["status"].as_str().unwrap().to_string(),
    )
}

#[test]
fn doctor_reports_a_hook_that_is_not_installed() {
    let dir = hook_fixture("hook-absent");
    assert_eq!(
        hook_state(&dir, "pre-commit"),
        ("not-installed".into(), "warn".into())
    );
    assert_eq!(
        hook_state(&dir, "pre-push"),
        ("not-installed".into(), "warn".into())
    );
}

#[test]
fn doctor_reports_a_hook_that_runs_urzua() {
    let dir = hook_fixture("hook-running");
    plant_hook(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nexec urzua check --base HEAD\n",
        true,
    );
    assert_eq!(
        hook_state(&dir, "pre-commit"),
        ("installed-running-urzua".into(), "ok".into())
    );
}

#[test]
fn doctor_reports_a_hook_that_does_not_run_urzua() {
    let dir = hook_fixture("hook-other");
    plant_hook(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho hello\n",
        true,
    );
    assert_eq!(
        hook_state(&dir, "pre-commit"),
        ("installed-not-running-urzua".into(), "warn".into())
    );
}

#[test]
fn doctor_reports_a_non_executable_hook_git_would_ignore() {
    let dir = hook_fixture("hook-noexec");
    plant_hook(
        &dir.join(".git/hooks/pre-commit"),
        "#!/bin/sh\nurzua check\n",
        false,
    );
    assert_eq!(
        hook_state(&dir, "pre-commit"),
        ("installed-not-executable".into(), "warn".into())
    );
}

#[test]
fn doctor_follows_core_hooks_path() {
    let dir = hook_fixture("hook-hookspath");
    plant_hook(
        &dir.join(".githooks/pre-commit"),
        "#!/bin/sh\nurzua check --base HEAD\n",
        true,
    );
    // A hook in .git/hooks is not what git runs once hooksPath is set.
    plant_hook(&dir.join(".git/hooks/pre-push"), "urzua check\n", true);
    git(&dir, &["config", "core.hooksPath", ".githooks"]);
    assert_eq!(
        hook_state(&dir, "pre-commit"),
        ("installed-running-urzua".into(), "ok".into())
    );
    assert_eq!(
        hook_state(&dir, "pre-push"),
        ("not-installed".into(), "warn".into())
    );
}

// ---- MILE-179: the repository's own versioned hooks ----

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

/// Runs this repository's pre-commit hook in a fixture whose `rust/target`
/// holds `bin` (or nothing).
fn run_pre_commit(name: &str, bin: Option<&str>) -> std::process::Output {
    let dir = hook_fixture(name);
    if let Some(body) = bin {
        plant_hook(&dir.join("rust/target/release/urzua"), body, true);
    }
    Command::new("bash")
        .arg(repo_root().join(".git-hooks/pre-commit"))
        .current_dir(&dir)
        .output()
        .unwrap()
}

#[test]
fn pre_commit_hook_says_loudly_when_the_binary_is_absent() {
    let out = run_pre_commit("hook-nobin", None);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("NOT RUN"),
        "must say it did not run: {stderr}"
    );
    assert!(
        stderr.contains("CI"),
        "must say CI is the guarantee: {stderr}"
    );
    assert!(
        stderr.contains("make build"),
        "must say how to fix it: {stderr}"
    );
    // Warn, not fail: a fresh clone must still be able to make the commit
    // that builds (ADR-71 option D: hooks cannot be forced).
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn pre_commit_hook_stops_a_commit_on_a_blocking_finding_and_shows_it() {
    let out = run_pre_commit(
        "hook-block",
        Some("#!/bin/sh\necho '{\"findings\":[\"bad header\"]}'\nexit 1\n"),
    );
    assert_eq!(out.status.code(), Some(1));
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        all.contains("bad header"),
        "findings must be visible: {all}"
    );
}

#[test]
fn pre_commit_hook_reports_a_check_that_could_not_run() {
    let out = run_pre_commit("hook-notrun", Some("#!/bin/sh\necho oops >&2\nexit 2\n"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("NOT RUN"), "{stderr}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn pre_commit_hook_passes_silently_on_a_clean_check() {
    let out = run_pre_commit("hook-clean", Some("#!/bin/sh\necho '{}'\nexit 0\n"));
    assert_eq!(out.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("NOT RUN"));
}

/// The acceptance case: the real binary, a real hook, a planted bad header.
#[test]
fn pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary() {
    let dir = hook_fixture("hook-real");
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n  header.required-fields: error\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: [Status]\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nStatus: Accepted\n---\n# 1 -- x\n",
    )
    .unwrap();
    commit_all(&dir);
    std::fs::create_dir_all(dir.join("rust/target/release")).unwrap();
    std::fs::copy(
        env!("CARGO_BIN_EXE_urzua"),
        dir.join("rust/target/release/urzua"),
    )
    .unwrap();
    // The planted defect: the required Status field is removed.
    std::fs::write(
        dir.join("docs/adr/ADR-1-x.md"),
        "---\nDate: 2026-01-01\n---\n# 1 -- x\n",
    )
    .unwrap();
    let out = Command::new("bash")
        .arg(repo_root().join(".git-hooks/pre-commit"))
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn ci_wired_recognises_the_gate_action_as_an_invocation() {
    // MILE-180: a workflow that runs the checker through urzua's own reusable
    // action never spells `urzua check`; reporting it unwired would be a false
    // negative about a repository that is wired in.
    let dir = fixture_repo("ci-wired-action");
    std::fs::create_dir_all(dir.join(".urzua")).unwrap();
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules: {}\n\n\
         record_types:\n\
         \x20 adr:\n    dir: \"docs/adr\"\n    required_fields: []\n    header_shape: \"yaml-frontmatter\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(
        dir.join(".github/workflows/records.yml"),
        "name: records\njobs:\n  gate:\n    steps:\n      - uses: urzuahq/urzua/.github/actions/urzua-gate@abc\n",
    )
    .unwrap();
    let out = run_urzua(&dir, &["doctor"]);
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ci = parsed["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["check"] == "ci-wired")
        .unwrap();
    assert_eq!(ci["status"], "ok", "{ci}");
}
