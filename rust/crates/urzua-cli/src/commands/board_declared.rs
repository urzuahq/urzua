//! What the configuration declares about each record type, as the board's
//! "Build your own view" prompt and its "About this board" panel read it. It
//! names no type, status or field of its own: every word comes from the
//! configuration or from the export's key lists below.

use std::fmt::Write;

use urzua_core::config::Config;
use urzua_core::status_model::StatusDecl;

/// The keys at the top of `urzua export --format json` that every export
/// carries, and those it carries only when something asks for them.
pub(crate) const TOP_KEYS: [&str; 9] = [
    "status",
    "record_count",
    "types",
    "type_roles",
    "read",
    "records",
    "next_action",
    "next_action_reason",
    "next_action_inputs",
];
pub(crate) const TOP_OPTIONAL: [(&str, &str); 1] =
    [("roadmap", "when the configuration declares roadmap.pairs")];

/// The keys of each record in the export, and those present only in some
/// cases.
pub(crate) const RECORD_KEYS: [&str; 25] = [
    "type",
    "path",
    "id",
    "stable_id",
    "header_readable",
    "header",
    "sections",
    "preamble",
    "title",
    "title_is_fallback",
    "fields",
    "status",
    "state",
    "state_decided_by",
    "waiting_on",
    "ready",
    "gated_moves",
    "parent",
    "parent_ambiguous",
    "children",
    "children_by_state",
    "state_disagrees_with_children",
    "refers_to",
    "unresolved_refs",
    "referred_by",
];
pub(crate) const RECORD_OPTIONAL: [(&str, &str); 2] = [
    ("raw_text", "unless --no-text or --no-raw-text"),
    ("link", "when the configuration declares export.link_prefix"),
];

pub(crate) struct Gate {
    pub to: Vec<String>,
    pub by: String,
}

pub(crate) struct TypeFacts {
    pub name: String,
    pub dir: String,
    pub fields: Vec<String>,
    pub field_values: Vec<(String, Vec<String>)>,
    pub statuses: Vec<String>,
    pub success: Vec<String>,
    pub failure: Vec<String>,
    pub progress: Vec<String>,
    pub initial: Vec<String>,
    pub gates: Vec<Gate>,
}

pub(crate) fn declared(config: &Config) -> Vec<TypeFacts> {
    config
        .record_types
        .iter()
        .map(|(name, ty)| {
            let decl = ty.status_decl();
            let pick = |f: fn(&StatusDecl) -> &Vec<String>| {
                decl.as_ref().map(|d| f(d).clone()).unwrap_or_default()
            };
            let gates = config
                .authority
                .iter()
                .flat_map(|a| a.gates.iter())
                .filter(|g| g.record_type.as_deref() == Some(name.as_str()))
                .map(|g| Gate {
                    to: g.to.clone(),
                    by: g.by.clone(),
                })
                .collect();
            TypeFacts {
                name: name.clone(),
                dir: ty.dir.clone(),
                fields: ty
                    .declared_fields()
                    .iter()
                    .map(|f| f.as_str().to_string())
                    .collect(),
                field_values: ty
                    .field_values
                    .iter()
                    .flatten()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                statuses: pick(|d| &d.values),
                success: pick(|d| &d.success),
                failure: pick(|d| &d.failure),
                progress: pick(|d| &d.progress),
                initial: pick(|d| &d.initial),
                gates,
            }
        })
        .collect()
}

fn keys(names: &[&str], optional: &[(&str, &str)]) -> String {
    let mut parts: Vec<String> = names.iter().map(|k| format!("`{k}`")).collect();
    parts.extend(optional.iter().map(|(k, when)| format!("`{k}` ({when})")));
    parts.join(", ")
}

fn list(items: &[String]) -> String {
    items.join(", ")
}

fn status_clause(t: &TypeFacts) -> String {
    if t.statuses.is_empty() {
        return "declares no statuses".to_string();
    }
    let mut out = format!("statuses {}", list(&t.statuses));
    for (label, set) in [
        ("initial", &t.initial),
        ("success", &t.success),
        ("failure", &t.failure),
        ("progress", &t.progress),
    ] {
        if !set.is_empty() {
            let _ = write!(out, "; {label}: {}", list(set));
        }
    }
    out
}

fn type_line(t: &TypeFacts) -> String {
    let mut out = format!("- {} ({})", t.name, t.dir);
    if !t.fields.is_empty() {
        let _ = write!(out, ": fields {}", list(&t.fields));
    }
    for (field, values) in &t.field_values {
        let _ = write!(out, "; {field} takes {}", list(values));
    }
    let _ = write!(out, "; {}", status_clause(t));
    out
}

/// The text a person pastes into a coding agent: how to read the export, the
/// keys it carries, and what this configuration declares.
pub(crate) fn prompt(types: &[TypeFacts]) -> String {
    let mut out = String::new();
    out.push_str(
        "I want a custom view of the records in this repository, built from `urzua export --format json`.\n\n\
Run it from the repository root. It prints one JSON object holding every record. Add `--no-text` to leave out each record's `raw_text` and each section's `content` (most of the size), `--no-raw-text` to leave out `raw_text` only, and `--no-checklist-items` to skip judging checklist items (faster; leaves out `items`).\n\n",
    );
    let _ = writeln!(out, "Top-level keys: {}.", keys(&TOP_KEYS, &TOP_OPTIONAL));
    let _ = writeln!(
        out,
        "Keys of each entry in `records`: {}.",
        keys(&RECORD_KEYS, &RECORD_OPTIONAL)
    );
    out.push_str(
        "`state` is one of finished, dropped, under-way, not-started, open, none, computed from the configuration. The three `next_action` keys are the envelope every urzua command prints; a view can ignore them. Read `type_roles` for what each type means to a view (`work`, `part_of`, `value_fields`, `waits_through`) and `read` for the commit and branch the records were read at.\n\n",
    );
    out.push_str("This repository's configuration declares these record types; use no others:\n");
    for t in types {
        out.push_str(&type_line(t));
        out.push('\n');
    }
    out.push_str(
        "\n`urzua board` is the plain default view and works with any configuration; it is not meant to be extended. The export is the stable input for a custom view: read it, not the record files, and take type and status names from it rather than assuming them.\n\n\
Write the view as one self-contained file I can open from disk (no network requests) unless I say otherwise. Re-run the export to refresh it.\n\n\
Build me a view that ...\n",
    );
    out
}
