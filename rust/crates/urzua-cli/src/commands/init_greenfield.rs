//! `urzua init` greenfield mode (SPEC-5, MILE-114): a repository with no
//! records yet chooses which types to govern. Each selected built-in profile is
//! written out into `config.yaml` as ordinary, visible rules -- never referenced
//! by name -- and a starter template per type into `.urzua/templates/`.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::header::HeaderShape;
use urzua_core::report::{CouldNotRun, Notice, NoticeSeverity, ReportStatus};
use yaml_serde::{Mapping, Value};

use crate::commands::init::{
    assemble_config, is_hidden_path, is_record_shaped, proposed_rules_mapping,
    withheld_rule_notices, InitContext, InitReport, ProposedRecordType, NOTICE_INIT_SCAN,
};
use crate::emit;

/// The default layout root: a type `adr` lives in `docs/adr`.
const DEFAULT_ROOT: &str = "docs";

/// One status a profile's lifecycle can leave from, and where it may go.
type Transition = (&'static str, &'static [&'static str]);

/// A built-in profile: the per-type half of RFC-1's core+profile model. Every
/// value here is written into the config verbatim, so an adopter who disagrees
/// edits a visible line.
pub(crate) struct Profile {
    pub(crate) name: &'static str,
    summary: &'static str,
    /// Required header fields after the status field (named by the status
    /// role's default, never a literal here), including the role the type is
    /// accountable to.
    required_fields: &'static [&'static str],
    /// Status vocabulary: the first value is the state `urzua new` writes.
    values: &'static [&'static str],
    success: &'static [&'static str],
    failure: &'static [&'static str],
    implementable: &'static [&'static str],
    transitions: &'static [Transition],
    /// The field `new` fills with the record's title, when not the H1 alone.
    title_field: Option<&'static str>,
    /// A type whose records carry a `Version` moved by a revision log.
    versioned: bool,
    template: &'static str,
}

pub(crate) const PROFILES: &[Profile] = &[
    Profile {
        name: "adr",
        summary: "architecture decision record: a choice already made",
        required_fields: &["Date", "Author", "Deciders", "Stable-Id"],
        values: &["Proposed", "Accepted", "Rejected", "Superseded"],
        success: &["Accepted"],
        failure: &["Rejected", "Superseded"],
        implementable: &["Accepted"],
        transitions: &[
            ("Proposed", &["Accepted", "Rejected"]),
            ("Accepted", &["Superseded"]),
        ],
        title_field: None,
        versioned: false,
        template: ADR_TEMPLATE,
    },
    Profile {
        name: "rfc",
        summary: "request for comments: a proposal still open for discussion",
        required_fields: &["Date", "Author", "Stable-Id"],
        values: &["Draft", "Discussion", "Accepted", "Rejected", "Superseded"],
        success: &["Accepted"],
        failure: &["Rejected", "Superseded"],
        implementable: &["Accepted"],
        transitions: &[
            ("Draft", &["Discussion", "Accepted", "Rejected"]),
            ("Discussion", &["Accepted", "Rejected"]),
            ("Accepted", &["Superseded"]),
        ],
        title_field: None,
        versioned: false,
        template: RFC_TEMPLATE,
    },
    Profile {
        name: "spec",
        summary: "specification: what a subject does, versioned",
        required_fields: &["Date", "Version", "Author", "Subject", "Stable-Id"],
        values: &["Draft", "Accepted", "Shipped"],
        success: &["Accepted", "Shipped"],
        failure: &[],
        implementable: &["Accepted", "Shipped"],
        transitions: &[("Draft", &["Accepted"]), ("Accepted", &["Shipped"])],
        title_field: Some("Subject"),
        versioned: true,
        template: SPEC_TEMPLATE,
    },
    // Provisional: RFC-1 leaves open whether a PRD fits the core+profile model.
    Profile {
        name: "prd",
        summary: "product requirements document, with a named owner",
        required_fields: &["Date", "Author", "Owner", "Stable-Id"],
        values: &["Draft", "Review", "Approved", "Rejected", "Superseded"],
        success: &["Approved"],
        failure: &["Rejected", "Superseded"],
        implementable: &["Approved"],
        transitions: &[
            ("Draft", &["Review"]),
            ("Review", &["Approved", "Rejected"]),
            ("Approved", &["Superseded"]),
        ],
        title_field: None,
        versioned: false,
        template: PRD_TEMPLATE,
    },
];

const ADR_TEMPLATE: &str = "---
Stable-Id: assigned by `urzua new`; never written by hand
Status: Proposed | Accepted | Rejected | Superseded
Date: YYYY-MM-DD
Author: name
Deciders: name(s)
---
# NNNN — Title

## Context

The situation forcing a decision: what is true, what constraints apply, what breaks if nothing
changes.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A | | |
| B | | |

## Decision

The choice, already made. A direction still open for comment is an RFC, not an ADR.

## Consequences

What becomes true, easier, harder or newly required, including the bad ones.
";

const RFC_TEMPLATE: &str = "---
Stable-Id: assigned by `urzua new`; never written by hand
Status: Draft | Discussion | Accepted | Rejected | Superseded
Date: YYYY-MM-DD
Author: name
---
# NNNN — Title

## Summary

The proposal in brief.

## Motivation

What problem this solves, with the evidence for it.

## Proposal

The design, concrete enough to review.

## Open questions

What is genuinely unresolved.
";

const SPEC_TEMPLATE: &str = "---
Stable-Id: assigned by `urzua new`; never written by hand
Status: Draft | Accepted | Shipped
Date: YYYY-MM-DD
Version: '0.1'
Author: name
Subject: one line on what this specification covers
---
# SPEC-N — Title

## Purpose

What this specification covers and why it exists.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
";

const PRD_TEMPLATE: &str = "---
Stable-Id: assigned by `urzua new`; never written by hand
Status: Draft | Review | Approved | Rejected | Superseded
Date: YYYY-MM-DD
Author: name
Owner: name
---
# NNNN — Title

## Problem

Who has it, and the evidence that it is real.

## Goals and non-goals

What success looks like, and what this deliberately does not attempt.

## Requirements

What the product must do, each testable.

## Open questions

What is still unresolved.
";

fn profile(name: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.name == name)
}

fn available() -> String {
    PROFILES
        .iter()
        .map(|p| p.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The profiles named by `--types`, in the order given. An unknown name or one
/// listed twice is refused: a declared input that selects nothing is reported.
fn select_profiles(types: &[String]) -> Result<Vec<&'static Profile>, String> {
    let mut chosen: Vec<&'static Profile> = Vec::new();
    for raw in types {
        let name = raw.trim();
        let Some(found) = profile(name) else {
            return Err(format!(
                "unknown record type '{name}' in --types; the built-in profiles are: {}",
                available()
            ));
        };
        if chosen.iter().any(|p| p.name == found.name) {
            return Err(format!("record type '{name}' is listed twice in --types"));
        }
        chosen.push(found);
    }
    if chosen.is_empty() {
        return Err(format!(
            "--types names no record type; the built-in profiles are: {}",
            available()
        ));
    }
    Ok(chosen)
}

/// The layout root from `--dir`: relative, inside the repository, not hidden.
fn layout_root(dir: Option<&Path>) -> Result<PathBuf, String> {
    use std::path::Component;
    let dir = dir.unwrap_or_else(|| Path::new(DEFAULT_ROOT));
    let mut root = PathBuf::new();
    for component in dir.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) if !part.to_string_lossy().starts_with('.') => {
                root.push(part)
            }
            _ => {
                return Err(format!(
                    "--dir {} must be a relative path inside the repository with no `..` and no hidden component",
                    dir.display()
                ))
            }
        }
    }
    Ok(root)
}

fn strings(items: &[&str]) -> Value {
    Value::Sequence(items.iter().map(|s| Value::from(*s)).collect())
}

fn statuses_mapping(p: &Profile) -> Mapping {
    let mut m = Mapping::new();
    m.insert("values".into(), strings(p.values));
    m.insert("success".into(), strings(p.success));
    if !p.failure.is_empty() {
        m.insert("failure".into(), strings(p.failure));
    }
    m.insert("implementable".into(), strings(p.implementable));
    m.insert("initial".into(), strings(&p.values[..1]));
    let mut transitions = Mapping::new();
    for (from, to) in p.transitions {
        transitions.insert(Value::from(*from), strings(to));
    }
    m.insert("transitions".into(), Value::Mapping(transitions));
    m
}

/// The revision log a versioned type keeps, and the version it starts at.
fn revision_log_mapping() -> Mapping {
    let mut bump = Mapping::new();
    bump.insert("field".into(), "Version".into());
    bump.insert("classes".into(), strings(&["substantive", "structural"]));
    bump.insert("initial".into(), "0.1".into());
    let mut log = Mapping::new();
    log.insert("marker".into(), "**Revision log**".into());
    log.insert("required".into(), Value::Bool(true));
    log.insert("classes".into(), strings(&["substantive", "structural"]));
    log.insert("class_column".into(), "Class".into());
    log.insert("date_column".into(), "Date".into());
    log.insert("tool_class".into(), "structural".into());
    log.insert("version_bump".into(), Value::Mapping(bump));
    log
}

fn type_mapping(p: &Profile, dir: &Path) -> Mapping {
    let mut entry = Mapping::new();
    entry.insert("dir".into(), dir.to_string_lossy().as_ref().into());
    entry.insert("prefix".into(), p.name.to_ascii_uppercase().into());
    let status = urzua_core::config::RelationRole::Status.default_field_name();
    let required: Vec<&str> = std::iter::once(status)
        .chain(p.required_fields.iter().copied())
        .collect();
    entry.insert("required_fields".into(), strings(&required));
    entry.insert("header_shape".into(), "yaml-frontmatter".into());
    // Declared, even empty, so a field a record adds beyond the required ones is
    // a finding and not an unchecked default (`config.known-fields-declaration-missing`).
    entry.insert("known_fields".into(), Value::Sequence(Vec::new()));
    let mut relation = Mapping::new();
    relation.insert("date".into(), "Date".into());
    relation.insert("author".into(), "Author".into());
    if let Some(title) = p.title_field {
        relation.insert("title".into(), title.into());
    }
    entry.insert("relation_fields".into(), Value::Mapping(relation));
    entry.insert("statuses".into(), Value::Mapping(statuses_mapping(p)));
    if p.versioned {
        entry.insert(
            "revision_log".into(),
            Value::Mapping(revision_log_mapping()),
        );
    }
    entry
}

/// Each selected type with the directory it is laid out in.
fn layouts(profiles: &[&'static Profile], root: &Path) -> Vec<(&'static Profile, PathBuf)> {
    profiles.iter().map(|p| (*p, root.join(p.name))).collect()
}

fn render(layouts: &[(&'static Profile, PathBuf)], proposed: &[ProposedRecordType]) -> String {
    let mut types = Mapping::new();
    for (p, dir) in layouts {
        types.insert(Value::from(p.name), Value::Mapping(type_mapping(p, dir)));
    }
    // The profiles write `required_fields` and `known_fields`, so the rules that
    // read them have something to examine and are proposed; the rest stay withheld.
    let rules = proposed_rules_mapping(proposed, &DECLARED);
    assemble_config(
        "# Generated by `urzua init --types` (greenfield mode). Each type below is a built-in\n# profile written out in full: edit any line.",
        types,
        rules,
    )
}

/// The declarations every profile writes.
const DECLARED: [urzua_core::rules::Declaration; 2] = [
    urzua_core::rules::Declaration::RequiredFields,
    urzua_core::rules::Declaration::KnownFields,
];

fn proposed_for(
    layouts: &[(&'static Profile, PathBuf)],
    discovered: &[PathBuf],
) -> Vec<ProposedRecordType> {
    layouts
        .iter()
        .map(|(p, dir)| ProposedRecordType {
            name: p.name.to_string(),
            dir: dir.to_string_lossy().to_string(),
            record_count: discovered
                .iter()
                .filter(|rel| {
                    rel.parent() == Some(dir.as_path())
                        && rel
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(is_record_shaped)
                })
                .count(),
            prefix: Some(p.name.to_ascii_uppercase()),
            header_shape: HeaderShape::YamlFrontmatter,
            exclude_files: Vec::new(),
        })
        .collect()
}

const LISTED_PATHS: usize = 20;

/// Record-shaped files no selected type governs, named by path: reported, never
/// an error (SPEC-5). A hidden directory is never mentioned.
fn unclassified_notice(discovered: &[PathBuf], covered: &[PathBuf]) -> Option<Notice> {
    let mut paths: Vec<String> = discovered
        .iter()
        .filter(|rel| {
            rel.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(is_record_shaped)
                && !is_hidden_path(rel)
                && !covered.iter().any(|dir| rel.starts_with(dir))
        })
        .map(|rel| rel.to_string_lossy().to_string())
        .collect();
    if paths.is_empty() {
        return None;
    }
    let more = paths.len().saturating_sub(LISTED_PATHS);
    let total = paths.len();
    paths.truncate(LISTED_PATHS);
    let tail = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    Some(Notice::new(
        NoticeSeverity::Warning,
        NOTICE_INIT_SCAN.to_string(),
        format!(
            "not classified, because they are outside every selected type's directory: {total} record-shaped file(s): {}{tail}; they are governed by nothing until a type is declared for their directory",
            paths.join(", ")
        ),
    ))
}

/// What happened to one starter template.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct TemplateOutcome {
    #[serde(rename = "type")]
    record_type: String,
    path: PathBuf,
    /// `written`, `kept` (one already exists and is never overwritten) or
    /// `would-write` (a dry run).
    state: &'static str,
}

fn write_templates(
    repo_root: &Path,
    profiles: &[&'static Profile],
    dry_run: bool,
) -> Result<Vec<TemplateOutcome>, String> {
    let mut out = Vec::new();
    for p in profiles {
        let rel = PathBuf::from(".urzua/templates").join(format!("{}.md", p.name));
        let full = repo_root.join(&rel);
        let state = if full.exists() {
            "kept"
        } else if dry_run {
            "would-write"
        } else {
            if let Some(parent) = full.parent() {
                urzua_io::confine_to_root(repo_root, &full)?;
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
            }
            urzua_io::write_atomic_in(repo_root, &full, p.template)
                .map_err(|e| format!("could not write {}: {e}", full.display()))?;
            "written"
        };
        out.push(TemplateOutcome {
            record_type: p.name.to_string(),
            path: rel,
            state,
        });
    }
    Ok(out)
}

/// Whether a menu can be shown: only to a person at a terminal.
pub(crate) fn can_prompt() -> bool {
    std::io::stdin().is_terminal()
}

/// Reads the type selection from `input`, showing the menu on `out` (stderr in
/// practice: stdout carries only the JSON report). Names or menu numbers,
/// separated by commas or spaces.
pub(crate) fn prompt_selection(
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<Vec<String>, String> {
    let _ = writeln!(
        out,
        "No records found. Choose the record types to govern (numbers or names, comma-separated):"
    );
    for (i, p) in PROFILES.iter().enumerate() {
        let _ = writeln!(out, "  {}) {:<5} {}", i + 1, p.name, p.summary);
    }
    let _ = write!(out, "types> ");
    let _ = out.flush();
    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|e| format!("could not read the selection: {e}"))?;
    let mut chosen = Vec::new();
    for token in line.split(|c: char| c == ',' || c.is_whitespace()) {
        if token.is_empty() {
            continue;
        }
        let name = match token.parse::<usize>() {
            Ok(n) => PROFILES
                .get(n.wrapping_sub(1))
                .map(|p| p.name.to_string())
                .ok_or_else(|| format!("{n} is not a menu number (1-{})", PROFILES.len()))?,
            Err(_) => token.to_string(),
        };
        chosen.push(name);
    }
    if chosen.is_empty() {
        return Err(format!(
            "no record type selected; pass --types with some of: {}",
            available()
        ));
    }
    Ok(chosen)
}

/// The menu on a terminal, then the same path as `--types`.
pub(crate) fn run_interactive(ctx: &InitContext, dir: Option<&Path>) -> ExitCode {
    let stdin = std::io::stdin();
    let selection = prompt_selection(&mut stdin.lock(), &mut std::io::stderr());
    match selection {
        Ok(types) => run(ctx, &types, dir),
        Err(why) => emit(&CouldNotRun::from(why)),
    }
}

/// Greenfield mode. Writes the templates (never over an existing one), then the
/// config whole or not at all. An existing config is a no-op only when this
/// selection would write exactly it.
pub(crate) fn run(ctx: &InitContext, types: &[String], dir: Option<&Path>) -> ExitCode {
    let profiles = match select_profiles(types) {
        Ok(p) => p,
        Err(why) => return emit(&CouldNotRun::from(why)),
    };
    let root = match layout_root(dir) {
        Ok(r) => r,
        Err(why) => return emit(&CouldNotRun::from(why)),
    };
    let layouts = layouts(&profiles, &root);
    let proposed = proposed_for(&layouts, ctx.discovered);
    let rendered = render(&layouts, &proposed);
    // The file written must load: greenfield output is the product's first
    // sixty seconds, and a config `check` refuses is the worst outcome.
    if let Err(e) = urzua_core::config::parse(&rendered) {
        return emit(&CouldNotRun::from(format!(
            "greenfield generated a config that does not load ({e}); this is a bug in `urzua init`, nothing was written"
        )));
    }

    let mut notices = Vec::new();
    notices.extend(unclassified_notice(
        ctx.discovered,
        &layouts.iter().map(|(_, d)| d.clone()).collect::<Vec<_>>(),
    ));
    notices.extend(withheld_rule_notices(&DECLARED));

    let mut written = !ctx.dry_run;
    if ctx.config_exists {
        let existing = std::fs::read_to_string(ctx.config_path).unwrap_or_default();
        if ctx.dry_run {
            notices.push(Notice::new(NoticeSeverity::Info, NOTICE_INIT_SCAN.to_string(), format!(
                "{} already exists; this dry run shows what greenfield would write and nothing was written",
                ctx.config_path.display()
            )));
        } else if existing == rendered {
            notices.push(Notice::new(
                NoticeSeverity::Info,
                NOTICE_INIT_SCAN.to_string(),
                format!(
                    "{} already holds exactly this configuration; nothing to change",
                    ctx.config_path.display()
                ),
            ));
            written = false;
        } else {
            return emit(&CouldNotRun::from(format!(
                "{} already exists and differs from what --types would write -- refusing to overwrite. Edit it directly, or remove it to re-run init.",
                ctx.config_path.display()
            )));
        }
    }

    let templates = match write_templates(ctx.repo_root, &profiles, ctx.dry_run) {
        Ok(t) => t,
        Err(why) => return emit(&CouldNotRun::from(why)),
    };
    if written {
        if let Err(why) = crate::commands::init::write_config(
            ctx.repo_root,
            ctx.config_path,
            ctx.explicit_config,
            &rendered,
        ) {
            return emit(&CouldNotRun::from(why));
        }
    }
    emit(&InitReport {
        status: ReportStatus::Ok,
        dry_run: ctx.dry_run,
        config_path: ctx.relative_config_path(),
        proposed,
        written,
        config_yaml: ctx.dry_run.then_some(rendered),
        templates,
        notices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn select(input: &str) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        prompt_selection(&mut input.as_bytes(), &mut out)
    }

    #[test]
    fn the_menu_accepts_numbers_and_names() {
        assert_eq!(select("1, rfc 3\n").unwrap(), ["adr", "rfc", "spec"]);
    }

    #[test]
    fn an_empty_or_out_of_range_selection_is_refused() {
        assert!(select("\n").is_err());
        assert!(select("").is_err());
        assert!(select("9\n").unwrap_err().contains("not a menu number"));
        assert!(select("0\n").is_err());
    }

    #[test]
    fn the_menu_is_shown_on_the_prompt_writer_not_stdout() {
        let mut out = Vec::new();
        let _ = prompt_selection(&mut "adr\n".as_bytes(), &mut out);
        let shown = String::from_utf8(out).unwrap();
        for p in PROFILES {
            assert!(shown.contains(p.name), "{shown}");
        }
    }

    #[test]
    fn every_profile_loads_and_starts_in_its_first_status() {
        let profiles: Vec<&Profile> = PROFILES.iter().collect();
        let layouts = layouts(&profiles, Path::new("docs"));
        let proposed = proposed_for(&layouts, &[]);
        let config = urzua_core::config::parse(&render(&layouts, &proposed)).unwrap();
        for p in PROFILES {
            let ty = config.record_types.get(p.name).unwrap();
            let decl = ty.status_decl().unwrap();
            assert_eq!(decl.values.first().map(String::as_str), Some(p.values[0]));
            assert!(p.required_fields.contains(&"Stable-Id"), "{}", p.name);
        }
    }

    #[test]
    fn a_layout_root_cannot_escape_or_hide() {
        assert!(layout_root(Some(Path::new("../x"))).is_err());
        assert!(layout_root(Some(Path::new("/x"))).is_err());
        assert!(layout_root(Some(Path::new("docs/.x"))).is_err());
        assert_eq!(
            layout_root(Some(Path::new("./a/b/"))).unwrap(),
            Path::new("a/b")
        );
        assert_eq!(layout_root(None).unwrap(), Path::new("docs"));
    }
}
