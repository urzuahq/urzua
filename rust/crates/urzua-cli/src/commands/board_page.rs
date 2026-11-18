//! `urzua board`'s page: a template over the export's view model. It reads the
//! export's keys and computes nothing a consumer of `urzua export` could not
//! read; layout comes from the configuration's declarations (`type_roles`,
//! `part_of`, states), never from a type or status name.
//!
//! Every string that came from a record is HTML-escaped where it is written,
//! and a record's text is shown as plain text. The page's own script only
//! filters what is already on the page.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use urzua_core::report::{ExportReport, ExportedRecord};
use urzua_core::view_model::StateCategory;

use super::board_assets::{SCRIPT, STYLE};

/// What `urzua check` reported, reduced to the line the page shows.
pub(crate) struct CheckLine {
    pub status: String,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub waived: usize,
}

/// What the page says about `urzua check`.
pub(crate) enum CheckNote<'a> {
    /// The summary, computed when the page was written.
    Line(&'a CheckLine),
    /// A served page does not wait for a check: it links the path that runs one.
    Linked,
    Unavailable,
}

impl CheckLine {
    /// The summary as one line of text.
    pub(crate) fn text(&self) -> String {
        format!(
            "urzua check: {} - {} errors, {} warnings, {} info{}",
            self.status,
            self.errors,
            self.warnings,
            self.infos,
            if self.waived > 0 {
                format!(" ({} waived)", self.waived)
            } else {
                String::new()
            }
        )
    }
}

/// HTML-escapes text for an element body or a quoted attribute value.
pub(crate) fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The CSP source of an inline element: `'sha256-<base64 of the digest>'`.
fn csp_hash(content: &str) -> String {
    let hex = urzua_io::sha256_hex_of_str(content);
    let bytes: Vec<u8> = (0..hex.len() / 2)
        .filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok())
        .collect();
    let mut out = String::from("'sha256-");
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(BASE64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out.push('\'');
    out
}

fn state_word(state: StateCategory) -> &'static str {
    match state {
        StateCategory::Finished => "finished",
        StateCategory::Dropped => "dropped",
        StateCategory::UnderWay => "under way",
        StateCategory::NotStarted => "not started",
        StateCategory::Open => "open",
        StateCategory::NoStatuses => "no status",
    }
}

fn state_key(state: StateCategory) -> &'static str {
    match state {
        StateCategory::Finished => "finished",
        StateCategory::Dropped => "dropped",
        StateCategory::UnderWay => "under-way",
        StateCategory::NotStarted => "not-started",
        StateCategory::Open => "open",
        StateCategory::NoStatuses => "none",
    }
}

/// The most record text, in bytes, a page carries inline. Past it the text is
/// left out of the page: a served page links each record's text, a file says so.
const TEXT_BUDGET: usize = 2 * 1024 * 1024;

/// Where a record's text is shown.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Text {
    Inline,
    /// At `record/<id>` on the server that serves the page.
    Linked,
    LeftOut,
}

/// Whether an id may name a path: the server answers only these.
pub(crate) fn is_path_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The export, indexed for linking.
struct Page<'a> {
    e: &'a ExportReport,
    by_id: HashMap<&'a str, usize>,
    anchors: Vec<String>,
    text: Text,
}

impl<'a> Page<'a> {
    fn new(e: &'a ExportReport, serving: bool) -> Self {
        let bytes: usize = e
            .records
            .iter()
            .filter_map(|r| r.raw_text.as_ref())
            .map(String::len)
            .sum();
        let text = match (bytes <= TEXT_BUDGET, serving) {
            (true, _) => Text::Inline,
            (false, true) => Text::Linked,
            (false, false) => Text::LeftOut,
        };
        let mut anchors: Vec<String> = Vec::with_capacity(e.records.len());
        for (i, r) in e.records.iter().enumerate() {
            let stem: String =
                r.id.clone()
                    .unwrap_or_else(|| format!("p{i}"))
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
            let mut anchor = format!("r-{stem}");
            if anchors.contains(&anchor) {
                anchor = format!("{anchor}-{i}");
            }
            anchors.push(anchor);
        }
        let mut by_id = HashMap::new();
        for (i, r) in e.records.iter().enumerate() {
            if let Some(id) = r.id.as_deref() {
                by_id.entry(id).or_insert(i);
            }
        }
        Page {
            e,
            by_id,
            anchors,
            text,
        }
    }

    fn record(&self, i: usize) -> &'a ExportedRecord {
        &self.e.records[i]
    }

    fn is_work(&self, r: &ExportedRecord) -> bool {
        self.e
            .type_roles
            .get(&r.record_type)
            .is_some_and(|t| t.work)
    }

    /// A link to a record on this page, or its id as plain text.
    fn link(&self, id: &str) -> String {
        match self.by_id.get(id) {
            Some(&i) => format!("<a href=\"#{}\">{}</a>", esc(&self.anchors[i]), esc(id)),
            None => esc(id),
        }
    }

    fn links(&self, ids: &[String]) -> String {
        ids.iter()
            .map(|id| self.link(id))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The waits-on text: amber, with the ids as in-page links.
    fn waits(&self, r: &ExportedRecord) -> Option<String> {
        let v = &r.view;
        (v.state.is_open() && !v.waiting_on.is_empty()).then(|| self.links(&v.waiting_on))
    }

    /// The right-hand details of a row, in the order they are dropped for room:
    /// what it waits on, a chip per declared-value field, the checklist count,
    /// how many records refer to it, and a parent's progress.
    fn details(&self, r: &ExportedRecord) -> String {
        let v = &r.view;
        let mut out = String::new();
        if let Some(links) = self.waits(r) {
            let _ = write!(out, "<span class=\"x w\">waits on {links}</span>");
        }
        if let Some(role) = self.e.type_roles.get(&r.record_type) {
            for name in &role.value_fields {
                let text = v.fields.get(name).map(field_text).unwrap_or_default();
                if !text.is_empty() {
                    let _ = write!(
                        out,
                        "<span class=\"x k\" title=\"{}\">{}</span>",
                        esc(name),
                        esc(&text)
                    );
                }
            }
        }
        if let Some((resolved, total)) = checklist_ratio(r) {
            let _ = write!(
                out,
                "<span class=\"x\" title=\"checklist items resolved\">{resolved}/{total}</span>"
            );
        }
        let referrers: usize = v.referred_by.values().map(Vec::len).sum();
        if referrers > 0 {
            let _ = write!(out, "<span class=\"x\">{referrers} refer</span>");
        }
        if !v.children.is_empty() {
            let _ = write!(
                out,
                "<span class=\"x\">{} of {} done{}</span>",
                v.children_by_state.finished,
                v.children.len(),
                if v.state_disagrees_with_children {
                    " · disagrees"
                } else {
                    ""
                }
            );
        }
        out
    }

    /// The row: a disclosure slot, the id, the title (one line), and at the
    /// right the details, the dot and the status.
    fn summary(&self, i: usize, parent: bool) -> String {
        let r = self.record(i);
        let id = esc(r.id.as_deref().unwrap_or(&r.path.to_string_lossy()));
        let title = if r.view.title_is_fallback {
            String::new()
        } else {
            esc(&r.view.title)
        };
        let hover = if title.chars().count() > TITLE_HOVER {
            format!(" title=\"{title}\"")
        } else {
            String::new()
        };
        format!(
            "<summary><span class=\"ch{}\"></span><span class=\"id\">{id}</span><span class=\"t\"{hover}>{title}</span><span class=\"s\">{}{}</span></summary>",
            if parent { " tri" } else { "" },
            self.details(r),
            status_cell(r)
        )
    }

    /// What a tap opens: the lines the row has no room for, the references and
    /// what refers to the record, the checklist, and its text.
    fn more(&self, i: usize) -> String {
        let r = self.record(i);
        let v = &r.view;
        let role = self.e.type_roles.get(&r.record_type);
        let mut out = String::from("<div class=\"more\">");
        if let Some(parent) = &v.parent {
            let _ = write!(out, "<div>part of {}</div>", self.link(parent));
        }
        if let Some(links) = self.waits(r) {
            let _ = write!(out, "<div class=\"w\">waits on {links}</div>");
        }
        if let Some(role) = role {
            for name in &role.value_fields {
                if let Some(value) = v.fields.get(name) {
                    let _ = write!(out, "<div>{}: {}</div>", esc(name), esc(&field_text(value)));
                }
            }
        }
        let shown_once = |field: &String| {
            let parent =
                v.parent.is_some() && role.is_some_and(|t| t.part_of.as_ref() == Some(field));
            let waits =
                self.waits(r).is_some() && role.is_some_and(|t| t.waits_through.contains(field));
            parent || waits
        };
        for (field, ids) in v.refers_to.iter().filter(|(f, _)| !shown_once(f)) {
            let _ = write!(out, "<div>{}: {}</div>", esc(field), self.links(ids));
        }
        for (field, ids) in &v.unresolved_refs {
            let _ = write!(
                out,
                "<div>{}: {} (no such record)</div>",
                esc(field),
                esc(&ids.join(", "))
            );
        }
        for (field, ids) in &v.referred_by {
            let _ = write!(
                out,
                "<div>referred to through {}: {}</div>",
                esc(field),
                self.links(ids)
            );
        }
        out.push_str(&self.checklist_items(r));
        out.push_str(&self.text_block(r));
        out.push_str("</div>");
        out
    }

    fn checklist_items(&self, r: &ExportedRecord) -> String {
        if self.text != Text::Inline {
            return String::new();
        }
        let Some(items) = r
            .sections
            .as_ref()
            .and_then(|s| s.iter().find_map(|s| s.items.as_ref()))
        else {
            return String::new();
        };
        let mut out = String::new();
        for item in items {
            let mark = if item.resolved {
                "[x]"
            } else if item.waived {
                "[w]"
            } else {
                "[ ]"
            };
            let evidence = item
                .evidence
                .as_deref()
                .map(|e| format!(" — {}", esc(e)))
                .unwrap_or_default();
            let _ = write!(
                out,
                "<div class=\"item\"><code>{mark}</code> {}{evidence}</div>",
                esc(&item.text)
            );
        }
        out
    }

    fn text_block(&self, r: &ExportedRecord) -> String {
        match (self.text, r.raw_text.as_ref(), r.id.as_deref()) {
            (Text::Inline, Some(text), _) => format!("<pre>{}</pre>", esc(text)),
            (Text::Linked, _, Some(id)) if is_path_id(id) => {
                format!(
                    "<div><a href=\"record/{}\">The record's text</a></div>",
                    esc(id)
                )
            }
            _ => String::new(),
        }
    }

    fn attrs(&self, i: usize, level: usize, parent: bool) -> String {
        let r = self.record(i);
        let mut flags = Vec::new();
        if !r.view.gated_moves.is_empty() {
            flags.push("needs");
        }
        if r.view.ready {
            flags.push("ready");
        }
        format!(
            "id=\"{}\" class=\"rec lv{}{}\" data-record=\"{}\" data-state=\"{}\" data-flags=\"{}\"",
            esc(&self.anchors[i]),
            level.min(8),
            if parent { " p" } else { "" },
            esc(&r.record_type),
            state_key(r.view.state),
            flags.join(" ")
        )
    }

    /// Whether a child row is worth seeing without a tap: under way, ready,
    /// or waiting.
    fn is_active(&self, i: usize) -> bool {
        let v = &self.record(i).view;
        v.state == StateCategory::UnderWay
            || v.ready
            || (v.state.is_open() && !v.waiting_on.is_empty())
    }

    /// One record as a row. A parent holds its children (`kids`) and starts
    /// open only when one of them is active.
    fn entry(&self, i: usize, level: usize, kids: &str) -> String {
        let r = self.record(i);
        let parent = !r.view.children.is_empty();
        let open = parent
            && r.view
                .children
                .iter()
                .filter_map(|c| self.by_id.get(c.as_str()))
                .any(|&c| self.is_active(c));
        format!(
            "<li {}><details{}>{}{kids}{}</details></li>",
            self.attrs(i, level, parent),
            if open { " open" } else { "" },
            self.summary(i, parent),
            self.more(i)
        )
    }

    /// A muted row that opens in place to the finished and dropped records it
    /// stands for.
    fn fold(&self, ids: &[usize], level: usize) -> String {
        if ids.is_empty() {
            return String::new();
        }
        let count = |state| {
            ids.iter()
                .filter(|&&i| self.record(i).view.state == state)
                .count()
        };
        let parts: Vec<String> = [
            (count(StateCategory::Finished), "finished"),
            (count(StateCategory::Dropped), "dropped"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, word)| format!("{n} {word}"))
        .collect();
        let rows: String = ids.iter().map(|&i| self.entry(i, level + 1, "")).collect();
        format!(
            "<li class=\"fold lv{}\"><details><summary><span class=\"ch\"></span>{}, collapsed</summary><ul class=\"kids\">{rows}</ul></details></li>",
            level.min(8),
            parts.join(", ")
        )
    }
}

/// The dot of a record: its state, or ready where nothing says more. The
/// word is its title, and the status text beside it carries the rest.
fn dot(r: &ExportedRecord) -> String {
    let v = &r.view;
    let (class, word) = match v.state {
        StateCategory::Finished => ("g", "finished".to_string()),
        StateCategory::Dropped => ("r", "dropped".to_string()),
        StateCategory::UnderWay => ("o", "under way".to_string()),
        _ if v.ready => ("b", format!("ready ({})", state_word(v.state))),
        state => ("n", state_word(state).to_string()),
    };
    format!("<span class=\"d {class}\" title=\"{}\"></span>", esc(&word))
}

/// The dot and the status exactly as written.
fn status_cell(r: &ExportedRecord) -> String {
    let status = r.view.status.as_deref().unwrap_or_default();
    format!("{}<span class=\"st\">{}</span>", dot(r), esc(status))
}

fn field_text(value: &urzua_core::view_model::FieldValue) -> String {
    match value {
        urzua_core::view_model::FieldValue::Text(t) => t.clone(),
        urzua_core::view_model::FieldValue::List(l) => l.join(", "),
    }
}

/// The checklist items resolved, and how many there are, for a record with a
/// declared checklist.
fn checklist_ratio(r: &ExportedRecord) -> Option<(usize, usize)> {
    let items = r.sections.as_ref()?.iter().find_map(|s| s.items.as_ref())?;
    Some((items.iter().filter(|i| i.resolved).count(), items.len()))
}

fn head(csp_style: &str, csp_script: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src {csp_style}; script-src {csp_script}; base-uri 'none'; form-action 'none'\">\n\
<title>urzua board</title>\n<style>{STYLE}</style>\n</head>\n"
    )
}

/// `2026-11-18T06:16:35Z` as `2026-11-18 06:16 UTC`.
fn short_time(time: &str) -> String {
    match (time.get(..10), time.get(11..16)) {
        (Some(date), Some(clock)) => format!("{date} {clock} UTC"),
        _ => time.to_string(),
    }
}

/// The header: the name, the read stamp, the check summary.
fn header(page: &Page<'_>, check: &CheckNote<'_>) -> String {
    let read = &page.e.read;
    let commit = read
        .commit
        .as_deref()
        .map_or("no commit", |c| &c[..c.len().min(7)]);
    let branch = read.branch.as_deref().unwrap_or("detached head");
    let changes = if read.uncommitted_record_changes {
        ", uncommitted record changes"
    } else {
        ""
    };
    let mut out = format!(
        "<div class=\"top\"><b>urzua board</b><span class=\"meta\">{} · {} · read {} · working tree{changes}</span></div>\n",
        esc(branch),
        esc(commit),
        esc(&short_time(&read.time))
    );
    match check {
        CheckNote::Line(c) => {
            let _ = writeln!(out, "<p class=\"note\">{}</p>", esc(&c.text()));
        }
        CheckNote::Linked => out.push_str(
            "<p class=\"note\">urzua check is not run for this page, because it takes as long as <code>urzua check</code> does. <a href=\"check\">Run it</a> for the summary.</p>\n",
        ),
        CheckNote::Unavailable => out.push_str("<p class=\"note\">urzua check could not be run.</p>\n"),
    }
    match page.text {
        Text::Inline => {}
        Text::Linked => out.push_str("<p class=\"note\">The records hold too much text to put in the page; each row links its own.</p>\n"),
        Text::LeftOut => out.push_str("<p class=\"note\">The records hold too much text for a file: it is left out. Run <code>urzua board --serve</code> to read each record's text, or <code>urzua export --format json</code> for all of it.</p>\n"),
    }
    out
}

/// The search box and the state filter, with the count each filter selects.
fn controls(page: &Page<'_>) -> String {
    let readable: Vec<&ExportedRecord> = page.e.records.iter().collect();
    let count =
        |keep: &dyn Fn(&ExportedRecord) -> bool| readable.iter().filter(|r| keep(r)).count();
    let state = |s: StateCategory| move |r: &ExportedRecord| r.view.state == s;
    let filters: Vec<(&str, &str, usize)> = vec![
        ("", "All", readable.len()),
        (
            "needs",
            "Needs you",
            count(&|r| !r.view.gated_moves.is_empty()),
        ),
        (
            "under-way",
            "Under way",
            count(&state(StateCategory::UnderWay)),
        ),
        ("ready", "Ready", count(&|r| r.view.ready)),
        (
            "finished",
            "Finished",
            count(&state(StateCategory::Finished)),
        ),
        ("dropped", "Dropped", count(&state(StateCategory::Dropped))),
        (
            "not-started",
            "Not started",
            count(&state(StateCategory::NotStarted)),
        ),
        ("open", "Open", count(&state(StateCategory::Open))),
    ];
    let buttons: String = filters
        .iter()
        .filter(|(key, _, n)| {
            key.is_empty() || ["needs", "under-way", "ready"].contains(key) || *n > 0
        })
        .map(|(key, label, n)| {
            format!(
                "<button type=\"button\" data-f=\"{key}\"{}>{label} {n}</button>",
                if key.is_empty() { " class=\"on\"" } else { "" }
            )
        })
        .collect();
    format!(
        "<div class=\"bar\"><input id=\"q\" type=\"search\" placeholder=\"Search id or title\" aria-label=\"Search id or title\"><div class=\"seg\" role=\"group\" aria-label=\"Filter by state\">{buttons}</div></div>\n<p id=\"shown\" class=\"note\" hidden></p>\n"
    )
}

/// A section label with its count, then its list.
fn section(
    label: &str,
    count: &str,
    type_name: bool,
    note: &str,
    rows: &str,
    group: &str,
) -> String {
    let note = if note.is_empty() {
        String::new()
    } else {
        format!("<p class=\"note\">{}</p>", esc(note))
    };
    let attr = if group.is_empty() {
        String::new()
    } else {
        format!(" data-sec=\"{group}\"")
    };
    format!(
        "<section{attr}><h3{}>{}<i>{}</i></h3>{note}<ul class=\"list\">{rows}</ul></section>\n",
        if type_name { " class=\"ty\"" } else { "" },
        esc(label),
        esc(count)
    )
}

/// A short list of one-line rows: the first few, then the rest under "N more".
fn brief_rows(page: &Page<'_>, ids: &[usize], right: &dyn Fn(&ExportedRecord) -> String) -> String {
    const SHOWN: usize = 8;
    let row = |&i: &usize| {
        let r = page.record(i);
        format!(
            "<li class=\"br\"><span class=\"id\">{}</span><span class=\"t\">{}</span><span class=\"s\">{}</span></li>",
            page.link(r.id.as_deref().unwrap_or_default()),
            esc(&r.view.title),
            right(r)
        )
    };
    let mut out: String = ids.iter().take(SHOWN).map(row).collect();
    if ids.len() > SHOWN {
        let rest: String = ids.iter().skip(SHOWN).map(row).collect();
        let _ = write!(
            out,
            "<li class=\"fold\"><details><summary><span class=\"ch\"></span>{} more</summary><ul class=\"kids\">{rest}</ul></details></li>",
            ids.len() - SHOWN
        );
    }
    out
}

fn attention(page: &Page<'_>) -> String {
    let all = || (0..page.e.records.len()).filter(|&i| page.record(i).header_readable);
    let gated: Vec<usize> = all()
        .filter(|&i| !page.record(i).view.gated_moves.is_empty())
        .collect();
    let under_way: Vec<usize> = all()
        .filter(|&i| page.record(i).view.state == StateCategory::UnderWay)
        .collect();
    let ready: Vec<usize> = all().filter(|&i| page.record(i).view.ready).collect();
    let mut out = String::new();
    let mut notes = String::new();
    for (name, role) in &page.e.type_roles {
        if role.gated_without_transitions {
            let _ = write!(
                notes,
                "The type {} declares a gate but no transitions, so the next move of its records cannot be read. ",
                esc(name)
            );
        }
    }
    let needs_right = |r: &ExportedRecord| {
        let moves: Vec<&str> = r.view.gated_moves.iter().map(|m| m.to.as_str()).collect();
        let mut groups: Vec<&str> = r.view.gated_moves.iter().map(|m| m.by.as_str()).collect();
        groups.dedup();
        format!(
            "<span>{} → {}</span><span class=\"x k\">{}</span>",
            esc(r.view.status.as_deref().unwrap_or_default()),
            esc(&moves.join(", ")),
            esc(&groups.join(", "))
        )
    };
    if !gated.is_empty() || !notes.is_empty() {
        out.push_str(&section(
            "Needs you",
            &gated.len().to_string(),
            false,
            notes.trim(),
            &brief_rows(page, &gated, &needs_right),
            "needs",
        ));
    }
    let without_progress: Vec<&str> = page
        .e
        .type_roles
        .iter()
        .filter(|(_, role)| role.statuses_declared && !role.progress_declared)
        .map(|(name, _)| name.as_str())
        .collect();
    let progress_note = if without_progress.is_empty() {
        String::new()
    } else {
        format!(
            "Nothing can be listed as under way for {}: no statuses.progress is declared for {}. Add it to the type's statuses in the configuration to name the statuses that mean work has begun; their records are shown as open until then.",
            without_progress.join(", "),
            if without_progress.len() == 1 { "it" } else { "them" }
        )
    };
    let by_status = |r: &ExportedRecord| status_cell(r);
    out.push_str(&section(
        "Under way",
        &under_way.len().to_string(),
        false,
        &progress_note,
        &brief_rows(page, &under_way, &by_status),
        "under-way",
    ));
    out.push_str(&section(
        "Ready",
        &ready.len().to_string(),
        false,
        "",
        &brief_rows(page, &ready, &by_status),
        "ready",
    ));
    out
}

/// The open, readable work records a tree is drawn from.
struct Tree {
    in_tree: Vec<bool>,
}

impl Tree {
    fn of(page: &Page<'_>) -> Self {
        let in_tree = (0..page.e.records.len())
            .map(|i| {
                let r = page.record(i);
                r.header_readable && page.is_work(r) && r.view.state.is_open()
            })
            .collect();
        Tree { in_tree }
    }

    fn parent_in_tree(&self, page: &Page<'_>, i: usize) -> bool {
        page.record(i)
            .view
            .parent
            .as_deref()
            .and_then(|p| page.by_id.get(p))
            .is_some_and(|&p| self.in_tree[p])
    }

    /// A finished or dropped work record: collapsed, never in the tree.
    fn is_closed_work(&self, page: &Page<'_>, i: usize) -> bool {
        let r = page.record(i);
        r.header_readable && page.is_work(r) && !r.view.state.is_open()
    }
}

/// A work record's row with its open children inside it and its finished and
/// dropped children folded. A record already drawn is not drawn again, so a
/// loop in the declarations ends where it closes.
fn draw(
    page: &Page<'_>,
    tree: &Tree,
    i: usize,
    level: usize,
    seen: &mut HashSet<usize>,
) -> Option<String> {
    if !seen.insert(i) {
        return None;
    }
    let children: Vec<usize> = page
        .record(i)
        .view
        .children
        .iter()
        .filter_map(|c| page.by_id.get(c.as_str()).copied())
        .collect();
    let mut inner = String::new();
    let mut closed = Vec::new();
    for c in children {
        if tree.in_tree[c] {
            if let Some(entry) = draw(page, tree, c, level + 1, seen) {
                inner.push_str(&entry);
            }
        } else if tree.is_closed_work(page, c) && seen.insert(c) {
            closed.push(c);
        }
    }
    inner.push_str(&page.fold(&closed, level + 1));
    let kids = if inner.is_empty() {
        String::new()
    } else {
        format!("<ul class=\"kids\">{inner}</ul>")
    };
    Some(page.entry(i, level, &kids))
}

/// Whether following `part_of` from `i` comes back to it.
fn in_loop(page: &Page<'_>, i: usize) -> bool {
    let mut at = i;
    for _ in 0..page.e.records.len() {
        match page
            .record(at)
            .view
            .parent
            .as_deref()
            .and_then(|p| page.by_id.get(p))
        {
            Some(&next) if next == i => return true,
            Some(&next) => at = next,
            None => return false,
        }
    }
    false
}

/// Work without containment: one list per type, its finished and dropped
/// records folded at the end.
fn work_by_type(page: &Page<'_>, tree: &Tree, seen: &mut HashSet<usize>) -> String {
    let n = page.e.records.len();
    let mut out = String::from("<p class=\"note\">The configuration declares no containment, so work is grouped by type.</p>\n");
    for name in page.e.type_roles.keys() {
        let of_type = |i: &usize| page.record(*i).record_type == *name;
        let open: Vec<usize> = (0..n)
            .filter(|&i| tree.in_tree[i])
            .filter(of_type)
            .collect();
        let closed: Vec<usize> = (0..n)
            .filter(|&i| tree.is_closed_work(page, i))
            .filter(of_type)
            .collect();
        if open.is_empty() && closed.is_empty() {
            continue;
        }
        let mut rows: String = open
            .iter()
            .filter_map(|&i| draw(page, tree, i, 0, seen))
            .collect();
        rows.push_str(&page.fold(&closed, 0));
        out.push_str(&section(
            name,
            &(open.len() + closed.len()).to_string(),
            true,
            "",
            &rows,
            "",
        ));
    }
    out
}

fn work_section(page: &Page<'_>) -> String {
    let tree = Tree::of(page);
    let n = page.e.records.len();
    let contained = page
        .e
        .type_roles
        .values()
        .any(|t| t.work && t.part_of.is_some());
    let total = (0..n)
        .filter(|&i| tree.in_tree[i] || tree.is_closed_work(page, i))
        .count();
    if total == 0 {
        return section("Work", "0", false, "No work record.", "", "");
    }
    let mut seen = HashSet::new();
    if !contained {
        return work_by_type(page, &tree, &mut seen);
    }
    let roots: Vec<usize> = (0..n)
        .filter(|&i| tree.in_tree[i] && (!tree.parent_in_tree(page, i) || in_loop(page, i)))
        .collect();
    let (parents, leaves): (Vec<usize>, Vec<usize>) = roots
        .into_iter()
        .partition(|&i| !page.record(i).view.children.is_empty());
    let (no_parent, elsewhere): (Vec<usize>, Vec<usize>) = leaves
        .into_iter()
        .partition(|&i| page.record(i).view.parent.is_none());
    let mut render = |ids: &[usize]| -> String {
        ids.iter()
            .filter_map(|&i| draw(page, &tree, i, 0, &mut seen))
            .collect()
    };
    let mut out = String::new();
    let rows = render(&parents);
    out.push_str(&section(
        "Work",
        &format!("{} parents · {total} records", parents.len()),
        false,
        "",
        &rows,
        "",
    ));
    if !no_parent.is_empty() {
        let rows = render(&no_parent);
        out.push_str(&section(
            "No parent",
            &no_parent.len().to_string(),
            false,
            "Records that are not part of another record.",
            &rows,
            "",
        ));
    }
    if !elsewhere.is_empty() {
        let rows = render(&elsewhere);
        out.push_str(&section(
            "Part of a record shown elsewhere",
            &elsewhere.len().to_string(),
            false,
            "The record each is part of is finished, dropped, of another kind or unreadable; its row says which.",
            &rows,
            "",
        ));
    }
    let closed: Vec<usize> = (0..n)
        .filter(|&i| tree.is_closed_work(page, i) && !seen.contains(&i))
        .collect();
    if !closed.is_empty() {
        let rows = page.fold(&closed, 0);
        out.push_str(&section(
            "Finished and dropped",
            &closed.len().to_string(),
            false,
            "",
            &rows,
            "",
        ));
    }
    out
}

/// A title longer than this carries its full text as a tooltip, for when the row cuts it short.
const TITLE_HOVER: usize = 40;

/// More than this many records in a type, and its finished and dropped ones fold.
const FOLD_OVER: usize = 10;

fn other_section(page: &Page<'_>) -> String {
    let n = page.e.records.len();
    let mut out = String::new();
    for name in page.e.type_roles.keys() {
        let of_type: Vec<usize> = (0..n)
            .filter(|&i| {
                let r = page.record(i);
                r.record_type == *name && r.header_readable && !page.is_work(r)
            })
            .collect();
        if of_type.is_empty() {
            continue;
        }
        let (open, closed): (Vec<usize>, Vec<usize>) = if of_type.len() > FOLD_OVER {
            of_type
                .iter()
                .partition(|&&i| page.record(i).view.state.is_open())
        } else {
            (of_type.clone(), Vec::new())
        };
        let mut rows: String = open.iter().map(|&i| page.entry(i, 0, "")).collect();
        rows.push_str(&page.fold(&closed, 0));
        out.push_str(&section(
            name,
            &of_type.len().to_string(),
            true,
            "",
            &rows,
            "",
        ));
    }
    out
}

fn unreadable_section(page: &Page<'_>) -> String {
    let ids: Vec<usize> = (0..page.e.records.len())
        .filter(|&i| !page.record(i).header_readable)
        .collect();
    if ids.is_empty() {
        return String::new();
    }
    let rows: String = ids.iter().map(|&i| page.entry(i, 0, "")).collect();
    section(
        "Headers that could not be read",
        &ids.len().to_string(),
        false,
        "These records are listed with their text; their state and links are unknown.",
        &rows,
        "",
    )
}

/// The whole page.
pub(crate) fn render(export: &ExportReport, check: &CheckNote<'_>, serving: bool) -> String {
    let page = Page::new(export, serving);
    let mut out = head(&csp_hash(STYLE), &csp_hash(SCRIPT));
    out.push_str("<body>\n<div class=\"wrap\">\n");
    out.push_str(&header(&page, check));
    out.push_str(&controls(&page));
    out.push_str(&attention(&page));
    out.push_str(&work_section(&page));
    out.push_str(&other_section(&page));
    out.push_str(&unreadable_section(&page));
    let _ = write!(out, "</div>\n<script>{SCRIPT}</script>\n</body>\n</html>\n");
    out
}
