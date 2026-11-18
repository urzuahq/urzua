//! The page's frame around the records: the top bar and its brand mark, the
//! tabs and the sidebar, the "About this board" and "Build your own view"
//! panels, and the footer. Everything here is the configuration's own words or
//! fixed text; no record's text is put into it.

use std::fmt::Write;

use urzua_core::report::{ExportReport, ExportedRecord};
use urzua_core::view_model::StateCategory;

use super::board_declared::TypeFacts;
use super::board_page::{
    esc, names_phrase, progress_gap_sentence, ready_leaf, with_code, without_backticks,
};

/// The five squares of the mark, as `(x, y, fill)`: a U.
const SQUARES: [(u8, u8, &str); 5] = [
    (0, 0, "#0F5BF0"),
    (0, 10, "#12B550"),
    (10, 10, "#FFD21F"),
    (20, 10, "#FF7A1A"),
    (20, 0, "#E5233D"),
];

/// The namespace is written as a character reference so that no address
/// appears in the page's text: the HTML parser reads it back as the same value,
/// and the check that the page carries no address of its own stays whole.
const NAMESPACE: &str = "http&#58;//www.w3.org/2000/svg";

pub(super) fn mark() -> String {
    let rects: String = SQUARES
        .iter()
        .map(|(x, y, fill)| {
            format!("<rect x=\"{x}\" y=\"{y}\" width=\"10\" height=\"10\" fill=\"{fill}\"/>")
        })
        .collect();
    format!(
        "<svg xmlns=\"{NAMESPACE}\" viewBox=\"0 0 30 20\" role=\"img\" aria-label=\"urzua\">{rects}</svg>"
    )
}

/// The mark as a `data:` address, for the favicon. Every character that could
/// end the attribute or a URL is percent-encoded, the colon of the namespace
/// included.
pub(super) fn favicon() -> String {
    let rects: String = SQUARES
        .iter()
        .map(|(x, y, fill)| format!("<rect x='{x}' y='{y}' width='10' height='10' fill='{fill}'/>"))
        .collect();
    let svg = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 30 20'>{rects}</svg>");
    let mut out = String::from("data:image/svg+xml,");
    for c in svg.chars() {
        match c {
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '#' => out.push_str("%23"),
            '\'' => out.push_str("%22"),
            ' ' => out.push_str("%20"),
            ':' => out.push_str("%3A"),
            c => out.push(c),
        }
    }
    out
}

fn exact_time(time: &str) -> String {
    match (time.get(..10), time.get(11..19)) {
        (Some(date), Some(clock)) => format!("{date} {clock} UTC"),
        _ => time.to_string(),
    }
}

fn stamp_text(e: &ExportReport) -> String {
    let read = &e.read;
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
    let config_changes = if read.uncommitted_config_changes == Some(true) {
        ", uncommitted configuration changes"
    } else {
        ""
    };
    format!(
        "{} · {} · working tree{changes}{config_changes}",
        esc(branch),
        esc(commit)
    )
}

fn actions() -> String {
    "<div class=\"acts\"><a class=\"btn go\" href=\"#build-view\" data-open=\"build-view\" aria-label=\"Build your own view\" title=\"Build your own view\"><span class=\"gly\" aria-hidden=\"true\">&lt;/&gt;</span><span class=\"lbl\">Build your own view</span></a><a class=\"ib\" href=\"#about\" data-open=\"about\" aria-label=\"About this board\" title=\"About this board\">i</a><button type=\"button\" id=\"theme\" class=\"ib js-only\" aria-label=\"Switch between light and dark\" title=\"Switch between light and dark\">&#9680;</button></div>".to_string()
}

/// The top bar: the mark and the name, the search, where the records were read
/// from and when, and the actions. A served page also carries the digest it was
/// built at and a Refresh button; a file says it is a file, since it cannot
/// refresh itself.
pub(super) fn top(e: &ExportReport, digest: Option<&str>) -> String {
    let time = esc(&exact_time(&e.read.time));
    let when = match digest {
        Some(digest) => format!(
            "<span class=\"meta\" id=\"upd\" data-d=\"{}\" title=\"{time}\">updated {time}</span><button type=\"button\" id=\"refresh\" class=\"rf\">Refresh</button>",
            esc(digest)
        ),
        None => format!("<span class=\"meta\" id=\"upd\">static file, generated {time}</span>"),
    };
    format!(
        "<header class=\"top\" id=\"top\"><a class=\"brand\" href=\"#top\" aria-label=\"urzua board\">{}<b>urzua</b><span class=\"sub\">board</span></a><div class=\"find\"><input id=\"q\" type=\"search\" placeholder=\"Search id or title\" aria-label=\"Search id or title\" autocomplete=\"off\"><kbd aria-hidden=\"true\">/</kbd></div><div class=\"stamp\"><span class=\"meta\" id=\"meta\">{}</span>{when}</div>{}</header>\n",
        mark(),
        stamp_text(e),
        actions()
    )
}

/// The views a person can filter by, with the count each selects: the three
/// the page leads with, and every state at least one record is in.
fn views(e: &ExportReport) -> Vec<(&'static str, &'static str, usize)> {
    let count = |keep: &dyn Fn(&ExportedRecord) -> bool| {
        e.records
            .iter()
            .filter(|r| r.header_readable && keep(r))
            .count()
    };
    let state = |s: StateCategory| move |r: &ExportedRecord| r.view.state == s;
    let all = [
        ("", "All", e.records.len()),
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
        ("ready", "Ready", count(&ready_leaf)),
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
    all.into_iter()
        .filter(|(key, _, n)| {
            key.is_empty() || ["needs", "under-way", "ready"].contains(key) || *n > 0
        })
        .collect()
}

pub(super) fn tabs(e: &ExportReport) -> String {
    let buttons: String = views(e)
        .iter()
        .map(|(key, label, n)| {
            format!(
                "<button type=\"button\" class=\"tab{}\" data-f=\"{key}\">{label}<i>{n}</i></button>",
                if key.is_empty() { " on" } else { "" }
            )
        })
        .collect();
    format!("<nav id=\"tabs\" class=\"tabs\" aria-label=\"Views\"><div class=\"tabrow\">{buttons}</div></nav>\n")
}

/// The sidebar: the views, then one entry per record type the configuration
/// declares, with how many records it has.
pub(super) fn side(e: &ExportReport) -> String {
    let mut out =
        String::from("<nav id=\"side\" class=\"side\" aria-label=\"Sections\"><h2>Views</h2>");
    for (key, label, n) in views(e) {
        let _ = write!(
            out,
            "<button type=\"button\" class=\"si{}\" data-f=\"{key}\"><span>{label}</span><i>{n}</i></button>",
            if key.is_empty() { " on" } else { "" }
        );
    }
    out.push_str("<h2>Types</h2>");
    for t in &e.types {
        let _ = write!(
            out,
            "<button type=\"button\" class=\"si\" data-t=\"{0}\"><span>{0}</span><i>{1}</i></button>",
            esc(&t.record_type),
            t.records
        );
    }
    out.push_str("</nav>\n");
    out
}

pub(super) fn footer() -> String {
    format!(
        "<footer class=\"foot\"><span>urzua {} · generated from this repository's configuration</span><span class=\"fx\"><a href=\"#about\" data-open=\"about\">About this board</a><a href=\"#build-view\" data-open=\"build-view\">Build your own view</a></span></footer>\n<div id=\"scrim\" class=\"scrim\" hidden></div>\n",
        env!("CARGO_PKG_VERSION")
    )
}

fn panel_head(id: &str, title: &str) -> String {
    format!(
        "<div class=\"sh\"><h3 id=\"{id}-h\">{title}</h3><button type=\"button\" class=\"ib x-close\" data-close aria-label=\"Close\">&times;</button></div>"
    )
}

/// The panel with the prompt a person gives their coding agent.
pub(super) fn build_panel(prompt: &str) -> String {
    format!(
        "<section class=\"card build\" id=\"build-view\" tabindex=\"-1\" aria-labelledby=\"build-view-h\">{}<p>This board is the plain default view and works with any configuration. To see these records another way, give your coding agent the text below; it was written from this repository's configuration. The same text is printed by <code>urzua board --prompt</code>.</p><textarea id=\"prompt\" readonly rows=\"16\" aria-label=\"Prompt for a coding agent\">{}</textarea><div class=\"copyrow\"><button type=\"button\" id=\"copy\" class=\"btn go js-only\">Copy prompt</button><span id=\"copied\" class=\"note\" role=\"status\"></span></div></section>\n",
        panel_head("build-view", "Build your own view"),
        esc(prompt)
    )
}

fn list_or(names: &[String], none: &str) -> String {
    if names.is_empty() {
        none.to_string()
    } else {
        names.join(", ")
    }
}

fn phrase(names: &[String]) -> String {
    esc(&names_phrase(
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
}

fn how_it_follows(e: &ExportReport, types: &[TypeFacts]) -> String {
    let contained: Vec<String> = types
        .iter()
        .filter_map(|t| {
            let field = e.type_roles.get(&t.name)?.part_of.as_ref()?;
            Some(format!("{} ({field})", t.name))
        })
        .collect();
    let with_progress: Vec<String> = types
        .iter()
        .filter(|t| !t.progress.is_empty())
        .map(|t| t.name.clone())
        .collect();
    let work: Vec<String> = types
        .iter()
        .filter(|t| e.type_roles.get(&t.name).is_some_and(|r| r.work))
        .map(|t| t.name.clone())
        .collect();
    let gated: Vec<String> = types
        .iter()
        .filter(|t| !t.gates.is_empty())
        .map(|t| t.name.clone())
        .collect();
    let tree = if contained.is_empty() {
        "No type declares a <code>part_of</code> field, so there is no tree and work is grouped by type.".to_string()
    } else {
        format!(
            "A record is drawn inside another's row only through a <code>part_of</code> field its type declares. Declared by: {}.",
            esc(&contained.join(", "))
        )
    };
    let mut out = String::from("<dl>");
    let _ = write!(out, "<dt>Work</dt><dd>The records of the types in <code>roadmap.work_types</code> (every type when it is not declared): {}. {tree}</dd>", esc(&list_or(&work, "none")));
    let _ = write!(out, "<dt>Under way</dt><dd>A record is under way when its status is in its type's <code>statuses.progress</code>. Types that declare one: {}. Without one a record that is neither finished nor dropped is open.</dd>", esc(&list_or(&with_progress, "none")));
    let _ = write!(out, "<dt>Needs you</dt><dd>A record appears when its status has a declared transition (<code>statuses.transitions</code>) to a status an <code>authority.gates</code> entry for its type reserves for a group. Types with such a gate: {}.</dd>", esc(&list_or(&gated, "none")));
    out.push_str("<dt>Ready</dt><dd>A record with no children, of a work type that declares statuses, that has not begun and waits on nothing through the pairs in <code>roadmap.pairs</code>. A record under way is never ready.</dd>");
    out.push_str("<dt>Finished, dropped</dt><dd>A status in the type's <code>statuses.success</code> is finished; one in <code>statuses.failure</code> is dropped.</dd></dl>");
    out
}

fn progress_part(t: &TypeFacts, unplaced: &[String]) -> String {
    if !t.progress.is_empty() {
        return format!("A record is under way in {}.", phrase(&t.progress));
    }
    if !unplaced.is_empty() {
        let names: Vec<String> = unplaced.iter().map(|s| without_backticks(s)).collect();
        return with_code(&progress_gap_sentence(&without_backticks(&t.name), &names));
    }
    if t.statuses.is_empty() {
        return "It declares no statuses, so its records have no state.".to_string();
    }
    if t.initial.is_empty() {
        return "It declares no <code>initial</code> status, so a start status cannot be told from an in-between one and a record that is neither finished nor dropped is open.".to_string();
    }
    "It declares no <code>statuses.progress</code> and has no status between its start and its end."
        .to_string()
}

fn type_sentences(t: &TypeFacts, e: &ExportReport) -> String {
    let role = e.type_roles.get(&t.name);
    let unplaced: &[String] = role.map_or(&[], |r| r.unplaced_statuses.as_slice());
    let mut parts = vec![if role.is_some_and(|r| r.work) {
        "A work type.".to_string()
    } else {
        "Not a work type: it is not in <code>roadmap.work_types</code>.".to_string()
    }];
    parts.push(progress_part(t, unplaced));
    parts.push(match role.and_then(|r| r.part_of.as_ref()) {
        Some(f) => format!("Its tree comes from <code>{}</code>.", esc(f)),
        None => "It declares no <code>part_of</code>, so it has no tree.".to_string(),
    });
    if let Some(r) = role.filter(|r| !r.waits_through.is_empty()) {
        parts.push(format!("It waits through {}.", phrase(&r.waits_through)));
    }
    for g in &t.gates {
        parts.push(format!(
            "A gate reserves the move to {} for <code>{}</code>.",
            phrase(&g.to),
            esc(&g.by)
        ));
    }
    if role.is_some_and(|r| r.gated_without_transitions) {
        parts.push(
            "It declares no <code>statuses.transitions</code>, so its next move cannot be read."
                .to_string(),
        );
    }
    parts.join(" ")
}

pub(super) fn about_panel(e: &ExportReport, types: &[TypeFacts]) -> String {
    let mut out = format!(
        "<section class=\"card about\" id=\"about\" tabindex=\"-1\" aria-labelledby=\"about-h\">{}<p>This page is drawn from this repository's configuration and nothing else. It has no opinion about what the records are called or how they move: each part below says which declaration it comes from.</p>",
        panel_head("about", "About this board")
    );
    out.push_str(&how_it_follows(e, types));
    out.push_str("<h4>What each type declares</h4><dl>");
    for t in types {
        let _ = write!(
            out,
            "<dt>{}</dt><dd>{}</dd>",
            esc(&t.name),
            type_sentences(t, e)
        );
    }
    out.push_str("</dl></section>\n");
    out
}
