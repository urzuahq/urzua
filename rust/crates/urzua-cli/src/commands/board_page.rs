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
        StateCategory::NoStatuses => "no status",
    }
}

fn state_key(state: StateCategory) -> &'static str {
    match state {
        StateCategory::Finished => "finished",
        StateCategory::Dropped => "dropped",
        StateCategory::UnderWay => "under-way",
        StateCategory::NotStarted => "not-started",
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
        let bytes: usize = e.records.iter().filter_map(|r| r.raw_text.as_ref()).map(String::len).sum();
        let text = match (bytes <= TEXT_BUDGET, serving) {
            (true, _) => Text::Inline,
            (false, true) => Text::Linked,
            (false, false) => Text::LeftOut,
        };
        let mut anchors: Vec<String> = Vec::with_capacity(e.records.len());
        for (i, r) in e.records.iter().enumerate() {
            let stem: String = r
                .id
                .clone()
                .unwrap_or_else(|| format!("p{i}"))
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
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
        self.e.type_roles.get(&r.record_type).is_some_and(|t| t.work)
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

    fn marks(&self, r: &ExportedRecord) -> String {
        let v = &r.view;
        let mut out = format!(
            "<span class=\"mark m-{}\">{}</span>",
            state_key(v.state),
            state_word(v.state)
        );
        if v.state.is_open() && !v.waiting_on.is_empty() {
            out.push_str(" <span class=\"mark m-waiting\">waiting</span>");
        }
        if v.ready {
            out.push_str(" <span class=\"mark m-ready\">ready</span>");
        }
        if v.state_disagrees_with_children {
            out.push_str(" <span class=\"mark m-disagree\">status and children disagree</span>");
        }
        out
    }

    fn counts(&self, r: &ExportedRecord) -> String {
        let c = &r.view.children_by_state;
        let parts: Vec<String> = [
            (c.finished, "finished"),
            (c.dropped, "dropped"),
            (c.under_way, "under way"),
            (c.not_started, "not started"),
            (c.none, "no status"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, word)| format!("{n} {word}"))
        .collect();
        format!(
            "<span class=\"counts\">{} {}: {}</span>",
            r.view.children.len(),
            if r.view.children.len() == 1 { "child" } else { "children" },
            parts.join(", ")
        )
    }

    /// The row a record opens with: id, title, status, marks.
    fn row(&self, i: usize, extra: &str) -> String {
        let r = self.record(i);
        let title = if r.view.title_is_fallback {
            String::new()
        } else {
            format!("<span class=\"title\">{}</span>", esc(&r.view.title))
        };
        let status = r
            .view
            .status
            .as_deref()
            .map(|s| format!("<span class=\"status\">{}</span>", esc(s)))
            .unwrap_or_default();
        format!(
            "<span class=\"id\">{}</span> {title} {status} {} {extra}",
            esc(r.id.as_deref().unwrap_or(&r.path.to_string_lossy())),
            self.marks(r)
        )
    }

    /// What the record waits on, its checklist, its declared-value fields and
    /// its references: the lines under its row.
    fn meta(&self, i: usize) -> String {
        let r = self.record(i);
        let v = &r.view;
        let mut out = String::from("<div class=\"meta\">");
        if let Some(parent) = &v.parent {
            let _ = write!(out, "<div>part of: {}</div>", self.link(parent));
        }
        if v.state.is_open() && !v.waiting_on.is_empty() {
            let _ = write!(out, "<div>waits on: {}</div>", self.links(&v.waiting_on));
        }
        if let Some(line) = checklist_line(r) {
            let _ = write!(out, "<div>{}</div>", esc(&line));
        }
        if let Some(role) = self.e.type_roles.get(&r.record_type) {
            for name in &role.value_fields {
                if let Some(value) = v.fields.get(name) {
                    let _ = write!(out, "<div>{}: {}</div>", esc(name), esc(&field_text(value)));
                }
            }
        }
        for (field, ids) in &v.refers_to {
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
        out.push_str("</div>");
        out
    }

    /// The expandable area: what refers to the record, and its text.
    fn more(&self, i: usize) -> String {
        let r = self.record(i);
        let mut out = String::from("<details class=\"more\"><summary>What refers to it and its text</summary>");
        out.push_str("<div class=\"meta\">");
        if r.view.referred_by.is_empty() {
            out.push_str("<div>Nothing refers to it.</div>");
        }
        for (field, ids) in &r.view.referred_by {
            let _ = write!(out, "<div>referred to through {}: {}</div>", esc(field), self.links(ids));
        }
        out.push_str("</div>");
        match (self.text, r.raw_text.as_ref(), r.id.as_deref()) {
            (Text::Inline, Some(text), _) => {
                let _ = write!(out, "<pre>{}</pre>", esc(text));
            }
            (Text::Linked, _, Some(id)) if is_path_id(id) => {
                let _ = write!(out, "<p class=\"note\"><a href=\"record/{}\">The record's text</a>, as plain text.</p>", esc(id));
            }
            _ => {}
        }
        out.push_str("</details>");
        out
    }

    fn attrs(&self, i: usize) -> String {
        let r = self.record(i);
        let search = format!(
            "{} {}",
            r.id.as_deref().unwrap_or_default(),
            r.view.title
        )
        .to_lowercase();
        format!(
            "id=\"{}\" class=\"rec\" data-record=\"{}\" data-state=\"{}\" data-search=\"{}\"",
            esc(&self.anchors[i]),
            esc(&r.record_type),
            state_key(r.view.state),
            esc(&search)
        )
    }

    /// A work record: its row and lines always visible, text on demand.
    fn work_entry(&self, i: usize, kids: &str) -> String {
        let r = self.record(i);
        if r.view.children.is_empty() {
            return format!(
                "<li {}><div class=\"row\">{}{}{}</div></li>",
                self.attrs(i),
                self.row(i, ""),
                self.meta(i),
                self.more(i)
            );
        }
        format!(
            "<li {}><details open><summary>{}</summary>{}{}{kids}</details></li>",
            self.attrs(i),
            self.row(i, &self.counts(r)),
            self.meta(i),
            self.more(i)
        )
    }

    /// Any other record: one line, its detail on demand.
    fn compact_entry(&self, i: usize) -> String {
        let r = self.record(i);
        let referrers: usize = r.view.referred_by.values().map(Vec::len).sum();
        let extra = if referrers > 0 {
            format!("<span class=\"counts\">{referrers} refer to it</span>")
        } else {
            String::new()
        };
        format!(
            "<li {}><details><summary>{}</summary>{}{}</details></li>",
            self.attrs(i),
            self.row(i, &extra),
            self.meta(i),
            self.more(i)
        )
    }
}

fn field_text(value: &urzua_core::view_model::FieldValue) -> String {
    match value {
        urzua_core::view_model::FieldValue::Text(t) => t.clone(),
        urzua_core::view_model::FieldValue::List(l) => l.join(", "),
    }
}

/// "3 of 5 items resolved" for a record with a declared checklist.
fn checklist_line(r: &ExportedRecord) -> Option<String> {
    let items = r
        .sections
        .as_ref()?
        .iter()
        .find_map(|s| s.items.as_ref())?;
    let resolved = items.iter().filter(|i| i.resolved).count();
    let waived = items.iter().filter(|i| i.waived).count();
    let mut line = format!("checklist: {resolved} of {} items resolved", items.len());
    if waived > 0 {
        let _ = write!(line, ", {waived} waived");
    }
    Some(line)
}

fn head(csp_style: &str, csp_script: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src {csp_style}; script-src {csp_script}; base-uri 'none'; form-action 'none'\">\n\
<title>urzua board</title>\n<style>{STYLE}</style>\n</head>\n"
    )
}

fn stamp(page: &Page<'_>, check: Option<&CheckLine>) -> String {
    let read = &page.e.read;
    let commit = read.commit.as_deref().map_or("no commit", |c| &c[..c.len().min(12)]);
    let branch = read.branch.as_deref().unwrap_or("no branch (detached head)");
    let changes = if read.uncommitted_record_changes {
        "record files have uncommitted changes"
    } else {
        "no record file has uncommitted changes"
    };
    let mut out = format!(
        "<p class=\"stamp\">Read <strong>{}</strong> from commit <code>{}</code> on branch <code>{}</code>; {}. This page shows the working tree.</p>\n",
        esc(&read.time),
        esc(commit),
        esc(branch),
        changes
    );
    match check {
        Some(c) => {
            let _ = writeln!(
                out,
                "<p class=\"check\">urzua check: <strong>{}</strong> &mdash; {} errors, {} warnings, {} info{}</p>",
                esc(&c.status),
                c.errors,
                c.warnings,
                c.infos,
                if c.waived > 0 { format!(" ({} waived)", c.waived) } else { String::new() }
            );
        }
        None => out.push_str("<p class=\"check\">urzua check: could not be run.</p>\n"),
    }
    out
}

fn controls(total: usize) -> String {
    format!(
        "<div class=\"controls\"><input id=\"q\" type=\"search\" placeholder=\"Search id and title\" aria-label=\"Search id and title\">\
<select id=\"s\" aria-label=\"Filter by state\"><option value=\"\">Any state</option>\
<option value=\"finished\">finished</option><option value=\"under-way\">under way</option>\
<option value=\"not-started\">not started</option><option value=\"dropped\">dropped</option>\
<option value=\"none\">no status</option></select><span id=\"shown\" class=\"note\">{total} of {total} records shown</span></div>\n"
    )
}

/// A brief list of links: open when short, collapsed under its count when long.
fn brief(page: &Page<'_>, title: &str, ids: &[usize], note: impl Fn(&ExportedRecord) -> String) -> String {
    let mut out = format!("<h3>{} ({})</h3>\n", esc(title), ids.len());
    if ids.is_empty() {
        out.push_str("<p class=\"note\">None.</p>\n");
        return out;
    }
    let open = if ids.len() <= 12 { " open" } else { "" };
    let _ = write!(out, "<details class=\"group\"{open}><summary>{} records</summary><ul class=\"brief\">", ids.len());
    for &i in ids {
        let r = page.record(i);
        let _ = write!(
            out,
            "<li>{} {} {}</li>",
            page.link(r.id.as_deref().unwrap_or_default()),
            esc(&r.view.title),
            note(r)
        );
    }
    out.push_str("</ul></details>\n");
    out
}

fn attention(page: &Page<'_>) -> String {
    let all = || (0..page.e.records.len()).filter(|&i| page.record(i).header_readable);
    let gated: Vec<usize> = all().filter(|&i| !page.record(i).view.gated_moves.is_empty()).collect();
    let under_way: Vec<usize> = all()
        .filter(|&i| page.record(i).view.state == StateCategory::UnderWay)
        .collect();
    let ready: Vec<usize> = all().filter(|&i| page.record(i).view.ready).collect();
    let mut out = String::from("<h2 id=\"attention\">What needs attention</h2>\n");
    out.push_str(&brief(page, "Waiting on a person", &gated, |r| {
        let moves: Vec<String> = r
            .view
            .gated_moves
            .iter()
            .map(|m| format!("to {} by {}", m.to, m.by))
            .collect();
        format!(
            "<span class=\"status\">{} &rarr; {}</span>",
            esc(r.view.status.as_deref().unwrap_or_default()),
            esc(&moves.join("; "))
        )
    }));
    for (name, role) in &page.e.type_roles {
        if role.gated_without_transitions {
            let _ = writeln!(
                out,
                "<p class=\"note\">The type {} declares a gate but no transitions, so the next move of its records cannot be read.</p>",
                esc(name)
            );
        }
    }
    out.push_str(&brief(page, "Under way", &under_way, |r| {
        format!("<span class=\"status\">{}</span>", esc(r.view.status.as_deref().unwrap_or_default()))
    }));
    out.push_str(&brief(page, "Ready", &ready, |r| {
        format!("<span class=\"status\">{}</span>", esc(&r.record_type))
    }));
    out
}

/// The open, readable work records a tree is drawn from, and what places each.
struct Tree {
    /// Work records in the tree: open, readable, of a work type.
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
}

/// A work record's entry with its open children inside it. A record already
/// drawn is not drawn again, so a loop in the declarations ends where it closes.
fn draw(page: &Page<'_>, tree: &Tree, i: usize, seen: &mut HashSet<usize>) -> Option<String> {
    if !seen.insert(i) {
        return None;
    }
    let children: Vec<usize> = page
        .record(i)
        .view
        .children
        .iter()
        .filter_map(|c| page.by_id.get(c.as_str()).copied())
        .filter(|&c| tree.in_tree[c])
        .collect();
    let mut inner = String::new();
    for c in children {
        if let Some(entry) = draw(page, tree, c, seen) {
            inner.push_str(&entry);
        }
    }
    let kids = if inner.is_empty() {
        String::new()
    } else {
        format!("<ul class=\"kids\">{inner}</ul>")
    };
    Some(page.work_entry(i, &kids))
}

fn group(title: &str, note: &str, entries: String, count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    let note = if note.is_empty() {
        String::new()
    } else {
        format!("<p class=\"note\">{}</p>", esc(note))
    };
    format!("<h3>{} ({count})</h3>{note}<ul>{entries}</ul>\n", esc(title))
}

/// Whether following `part_of` from `i` comes back to it.
fn in_loop(page: &Page<'_>, i: usize) -> bool {
    let mut at = i;
    for _ in 0..page.e.records.len() {
        match page.record(at).view.parent.as_deref().and_then(|p| page.by_id.get(p)) {
            Some(&next) if next == i => return true,
            Some(&next) => at = next,
            None => return false,
        }
    }
    false
}

fn work_section(page: &Page<'_>) -> String {
    let tree = Tree::of(page);
    let contained = page.e.type_roles.values().any(|t| t.work && t.part_of.is_some());
    let mut out = String::from("<h2 id=\"work\">Work</h2>\n");
    if !tree.in_tree.iter().any(|&b| b) {
        out.push_str("<p class=\"note\">No open work record.</p>\n");
        return out;
    }
    let mut seen = HashSet::new();
    if !contained {
        out.push_str("<p class=\"note\">The configuration declares no containment, so records are grouped by type.</p>\n");
        for name in page.e.type_roles.keys() {
            let ids: Vec<usize> = (0..page.e.records.len())
                .filter(|&i| tree.in_tree[i] && page.record(i).record_type == *name)
                .collect();
            let entries: String = ids
                .iter()
                .filter_map(|&i| draw(page, &tree, i, &mut seen))
                .collect();
            out.push_str(&group(name, "", entries, ids.len()));
        }
        return out;
    }
    let roots: Vec<usize> = (0..page.e.records.len())
        .filter(|&i| tree.in_tree[i] && (!tree.parent_in_tree(page, i) || in_loop(page, i)))
        .collect();
    let (parents, leaves): (Vec<usize>, Vec<usize>) = roots
        .into_iter()
        .partition(|&i| !page.record(i).view.children.is_empty());
    let (no_parent, elsewhere): (Vec<usize>, Vec<usize>) = leaves
        .into_iter()
        .partition(|&i| page.record(i).view.parent.is_none());
    let mut render = |ids: &[usize]| -> (String, usize) {
        let drawn: Vec<String> = ids
            .iter()
            .filter_map(|&i| draw(page, &tree, i, &mut seen))
            .collect();
        (drawn.concat(), drawn.len())
    };
    let (entries, n) = render(&parents);
    out.push_str(&group("Parents", "", entries, n));
    let (entries, n) = render(&no_parent);
    out.push_str(&group("No parent", "Records that are not part of another record.", entries, n));
    let (entries, n) = render(&elsewhere);
    out.push_str(&group(
        "Part of a record shown elsewhere",
        "The record each is part of is finished, dropped, of another kind or unreadable; its entry says which.",
        entries,
        n,
    ));
    out
}

fn other_section(page: &Page<'_>) -> String {
    let tree = Tree::of(page);
    let mut out = String::from("<h2 id=\"other\">Other records</h2>\n");
    let mut any = false;
    for name in page.e.type_roles.keys() {
        let ids: Vec<usize> = (0..page.e.records.len())
            .filter(|&i| {
                let r = page.record(i);
                r.record_type == *name
                    && r.header_readable
                    && !tree.in_tree[i]
                    && r.view.state.is_open()
            })
            .collect();
        let entries: String = ids.iter().map(|&i| page.compact_entry(i)).collect();
        any |= !ids.is_empty();
        out.push_str(&group(name, "", entries, ids.len()));
    }
    if !any {
        out.push_str("<p class=\"note\">None.</p>\n");
    }
    out
}

fn closed_section(page: &Page<'_>) -> String {
    let ids: Vec<usize> = (0..page.e.records.len())
        .filter(|&i| page.record(i).header_readable && !page.record(i).view.state.is_open())
        .collect();
    let mut out = format!(
        "<h2 id=\"closed\">Finished and dropped</h2>\n<details class=\"group\" id=\"closed-records\"><summary>{} records finished or dropped</summary>",
        ids.len()
    );
    for name in page.e.type_roles.keys() {
        let of_type: Vec<usize> = ids
            .iter()
            .copied()
            .filter(|&i| page.record(i).record_type == *name)
            .collect();
        let entries: String = of_type.iter().map(|&i| page.compact_entry(i)).collect();
        out.push_str(&group(name, "", entries, of_type.len()));
    }
    out.push_str("</details>\n");
    out
}

fn unreadable_section(page: &Page<'_>) -> String {
    let ids: Vec<usize> = (0..page.e.records.len())
        .filter(|&i| !page.record(i).header_readable)
        .collect();
    let mut out = String::from("<h2 id=\"unreadable\">Headers that could not be read</h2>\n");
    if ids.is_empty() {
        out.push_str("<p class=\"note\">Every record's header was read.</p>\n");
        return out;
    }
    out.push_str("<p class=\"note\">These records are listed with their text; their state and links are unknown.</p><ul>");
    for i in ids {
        out.push_str(&page.compact_entry(i));
    }
    out.push_str("</ul>\n");
    out
}

/// The whole page.
pub(crate) fn render(export: &ExportReport, check: Option<&CheckLine>, serving: bool) -> String {
    let page = Page::new(export, serving);
    let mut out = head(&csp_hash(STYLE), &csp_hash(SCRIPT));
    out.push_str("<body>\n<header class=\"top\"><h1>urzua board</h1>\n");
    out.push_str(&stamp(&page, check));
    match page.text {
        Text::Inline => {}
        Text::Linked => out.push_str("<p class=\"note\">The records hold too much text to put in the page; each entry links its own.</p>\n"),
        Text::LeftOut => out.push_str("<p class=\"note\">The records hold too much text for a file: it is left out. Run <code>urzua board --serve</code> to read each record's text, or <code>urzua export --format json</code> for all of it.</p>\n"),
    }
    out.push_str(&controls(export.records.len()));
    out.push_str("</header>\n<main>\n");
    out.push_str(&attention(&page));
    out.push_str(&work_section(&page));
    out.push_str(&other_section(&page));
    out.push_str(&closed_section(&page));
    out.push_str(&unreadable_section(&page));
    let _ = write!(out, "</main>\n<script>{SCRIPT}</script>\n</body>\n</html>\n");
    out
}
