//! `urzua doctor`'s `template-sections` check (`MILE-6`, `SPEC-15`): a type's
//! template has a heading for every section its configuration requires.
//!
//! Verifies: MILE-6

mod common;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run")
        .success());
}

fn doctor(dir: &Path) -> serde_json::Value {
    let out = common::urzua()
        .arg("doctor")
        .current_dir(dir)
        .output()
        .expect("urzua binary should run");
    serde_json::from_slice(&out.stdout).expect("JSON on stdout")
}

fn template_checks(report: &serde_json::Value) -> Vec<serde_json::Value> {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["check"] == "template-sections")
        .cloned()
        .collect()
}

const REQUIRED: &str = "      required: [{name: Context}, {name: Decision, aliases: [Outcome]}]\n";

/// A repository declaring `adr` with the required sections above (and `note`
/// with none), and the given templates.
fn repo(name: &str, required: &str, templates: &[(&str, &str)]) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-doctor-tpl-{name}"));
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    std::fs::create_dir_all(dir.join("docs/adr")).unwrap();
    std::fs::create_dir_all(dir.join("docs/note")).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        format!(
            "schema_version: 2\nrules:\n  header.required-fields: error\n\
             record_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    required_fields: [Status]\n    \
             header_shape: yaml-frontmatter\n    sections:\n      from: headings\n{required}  \
             note:\n    dir: docs/note\n    prefix: NOTE\n    required_fields: [Status]\n    \
             header_shape: yaml-frontmatter\n    sections: {{from: headings}}\n"
        ),
    )
    .unwrap();
    for (file, text) in templates {
        std::fs::write(dir.join(".urzua/templates").join(file), text).unwrap();
    }
    dir
}

#[test]
fn a_template_missing_a_required_section_is_reported_by_name() {
    let dir = repo(
        "missing",
        REQUIRED,
        &[(
            "adr.md",
            "---\nStatus: Proposed\n---\n# N — Title\n\n## Context\n\nx\n",
        )],
    );
    let checks = template_checks(&doctor(&dir));
    assert_eq!(
        checks.len(),
        1,
        "only the type with required sections: {checks:?}"
    );
    assert_eq!(checks[0]["status"], "warn", "{}", checks[0]);
    let message = checks[0]["message"].as_str().unwrap();
    assert!(message.contains("## Decision"), "{message}");
    assert!(!message.contains("## Context"), "{message}");
    assert!(message.contains("'adr'"), "{message}");
}

#[test]
fn a_matching_template_reports_clean_and_an_alias_satisfies_a_requirement() {
    let dir = repo(
        "clean",
        REQUIRED,
        &[(
            "adr.md",
            "---\nStatus: Proposed\n---\n# N — Title\n\n## Context\n\n## Outcome\n",
        )],
    );
    let checks = template_checks(&doctor(&dir));
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0]["status"], "ok", "{}", checks[0]);
}

#[test]
fn a_heading_inside_a_code_fence_is_not_a_heading() {
    let dir = repo(
        "fenced",
        REQUIRED,
        &[(
            "adr.md",
            "---\nStatus: Proposed\n---\n# N\n\n## Context\n\n```\n## Decision\n```\n",
        )],
    );
    let checks = template_checks(&doctor(&dir));
    assert_eq!(checks[0]["status"], "warn", "{}", checks[0]);
}

#[test]
fn a_type_with_no_template_has_nothing_to_compare_and_says_so() {
    let dir = repo("no-template", REQUIRED, &[]);
    let checks = template_checks(&doctor(&dir));
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0]["status"], "ok");
    assert!(checks[0]["message"]
        .as_str()
        .unwrap()
        .contains("no template"));
}

#[test]
fn a_type_declaring_no_required_section_is_not_listed() {
    let dir = repo("none", "", &[("adr.md", "# N\n")]);
    assert!(template_checks(&doctor(&dir)).is_empty());
}

#[test]
fn this_repositorys_own_templates_agree_with_its_own_configuration() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = common::TempDir::new("urzua-doctor-tpl-own");
    std::fs::create_dir_all(dir.join(".urzua/templates")).unwrap();
    git(&dir, &["init", "-q"]);
    std::fs::copy(
        root.join(".urzua/config.yaml"),
        dir.join(".urzua/config.yaml"),
    )
    .unwrap();
    for entry in std::fs::read_dir(root.join(".urzua/templates")).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(
            entry.path(),
            dir.join(".urzua/templates").join(entry.file_name()),
        )
        .unwrap();
    }
    let checks = template_checks(&doctor(&dir));
    assert!(
        checks.len() >= 5,
        "the own types declaring required sections: {checks:?}"
    );
    for c in &checks {
        assert_eq!(c["status"], "ok", "{c}");
    }
}

/// A repository whose `adr` type requires a `Decision` that may instead be a
/// Y-statement block in the preamble, with `text` as both its template and one
/// record, and `section.required` on.
fn marker_repo(name: &str, text: &str) -> common::TempDir {
    let dir = common::TempDir::new(&format!("urzua-doctor-tpl-{name}"));
    for d in [".urzua/templates", "docs/adr"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t@example.com"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(
        dir.join(".urzua/config.yaml"),
        "schema_version: 2\nrules:\n  section.required: error\nrecord_types:\n  adr:\n    dir: docs/adr\n    prefix: ADR\n    \
         required_fields: [Status]\n    header_shape: yaml-frontmatter\n    sections:\n      from: headings\n      \
         required: [{name: Decision, shape: y-statement, marker: 'Y-statement', preamble: true}]\n",
    )
    .unwrap();
    std::fs::write(dir.join(".urzua/templates/adr.md"), text).unwrap();
    std::fs::write(dir.join("docs/adr/ADR-1-x.md"), text).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "fixture"]);
    dir
}

/// `section.required` and `doctor` judge the same text the same way: a marker
/// in the preamble prose meets the requirement; one inside a fenced block or
/// inside the header does not.
#[test]
fn doctor_and_section_required_agree_on_a_marker_in_prose_a_fence_and_the_header() {
    let cases = [
        (
            "prose",
            "---\nStatus: Proposed\n---\n# 1 — T\n\nY-statement: in the context of x.\n",
            true,
        ),
        (
            "fence",
            "---\nStatus: Proposed\n---\n# 1 — T\n\n```\nY-statement: in the context of x.\n```\n",
            false,
        ),
        (
            "header",
            "---\nStatus: Proposed\nSubject: Y-statement\n---\n# 1 — T\n\nNothing.\n",
            false,
        ),
    ];
    for (name, text, met) in cases {
        let dir = marker_repo(name, text);
        let tpl = template_checks(&doctor(&dir));
        assert_eq!(tpl.len(), 1, "{name}");
        assert_eq!(tpl[0]["status"] == "ok", met, "{name} doctor: {}", tpl[0]);
        let out = common::urzua()
            .args(["check"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let missing = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule"] == "section.required");
        assert_eq!(missing, !met, "{name} section.required: {report}");
    }
}
