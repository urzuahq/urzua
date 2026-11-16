//! A revision log for an evidence row to land in, created when the record has
//! none: a re-pin or first pin is a tool-written row, and a log a person typed
//! by hand to receive it is the edit `diff.pin-changed-unattested` exists to
//! tell apart from one.

use crate::config::{RevisionLogId, RevisionLogShape};
use crate::line_edit::LineDoc;
use crate::rules::Fences;

/// What an evidence write does about the record's revision log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogPlan {
    /// The record has a log section; the row goes there.
    Present,
    /// The record has none; the write creates it, empty.
    Create,
    /// The record has no body section for a new log to follow.
    NoBody,
    /// The record ends inside a code fence opened on this 1-based line, where
    /// a new log would be read as code.
    OpenFence(usize),
}

/// Whether `content` has a revision log, needs one created, or has nowhere a
/// new one could go.
pub fn plan(content: &str, id: &RevisionLogId, shape: &RevisionLogShape) -> LogPlan {
    if crate::rules::find_revision_log_entries(content, id, shape).is_some() {
        return LogPlan::Present;
    }
    let body = body_headings(content);
    if let Some(line) = body.open_fence {
        LogPlan::OpenFence(line)
    } else if body.headings.is_empty() {
        LogPlan::NoBody
    } else {
        LogPlan::Create
    }
}

/// The refusal for a plan that creates nothing, `None` for one that writes.
pub fn refusal(plan: &LogPlan, id: &RevisionLogId, verb: &str) -> Option<String> {
    match plan {
        LogPlan::Present | LogPlan::Create => None,
        LogPlan::NoBody => Some(format!(
            "record has no body section for a new Revision log ({}) to follow -- refusing to {verb}; give the record a heading first",
            id.describe()
        )),
        LogPlan::OpenFence(line) => Some(format!(
            "record ends inside the code fence opened on line {line}, where a new Revision log ({}) would be read as code -- refusing to {verb}; close the fence first",
            id.describe()
        )),
    }
}

/// The empty log section a record of this type starts with: the template's
/// own section (identity line through the header's separator row) when the
/// template has a readable one, else one built from the declared identity and
/// columns.
pub fn section(
    id: &RevisionLogId,
    shape: &RevisionLogShape,
    template: Option<&str>,
) -> Vec<String> {
    template
        .and_then(|t| template_section(t, id, shape))
        .unwrap_or_else(|| declared_section(id, shape))
}

/// `content` with `section` placed where the type's template puts its log:
/// before the record's first heading that the template places after the log,
/// else at the end, set off by blank lines.
pub fn insert(
    content: &str,
    section: &[String],
    id: &RevisionLogId,
    template: Option<&str>,
) -> String {
    let after_log = template
        .map(|t| headings_after_log(t, id))
        .unwrap_or_default();
    let before = body_headings(content)
        .headings
        .into_iter()
        .find(|(_, text)| after_log.contains(text))
        .map(|(line, _)| line);
    let mut doc = LineDoc::parse(content);
    let lines: Vec<&str> = content.lines().collect();
    let mut at = before.map_or(lines.len(), |l| l - 1);
    let mut new: Vec<String> = Vec::new();
    if at > 0 && !lines[at - 1].trim().is_empty() {
        new.push(String::new());
    }
    new.extend(section.iter().cloned());
    if before.is_some() {
        new.push(String::new());
    }
    for line in new {
        doc.insert_after(at, line);
        at += 1;
    }
    doc.to_string()
}

fn template_section(
    template: &str,
    id: &RevisionLogId,
    shape: &RevisionLogShape,
) -> Option<Vec<String>> {
    let log = crate::rules::find_revision_log_entries(template, id, shape)?.ok()?;
    let lines: Vec<&str> = template.lines().collect();
    let start = crate::rules::log_marker_line(&lines, id)?;
    let end = log.last_table_line?;
    let header_end = log.entries.first().map_or(end, |e| e.line - 1);
    Some(
        lines
            .get(start..header_end)?
            .iter()
            .map(|l| l.to_string())
            .collect(),
    )
}

fn declared_section(id: &RevisionLogId, shape: &RevisionLogShape) -> Vec<String> {
    let date = shape.date_column.as_deref().unwrap_or("Date");
    let class = shape.class_column.as_deref().unwrap_or("Class");
    let header = format!("| {date} | {} | {class} |", shape.narrative_column);
    let separator = "|---|---|---|".to_string();
    match id {
        RevisionLogId::Marker(m) => vec![
            format!("> {}", m.trim()),
            ">".to_string(),
            format!("> {header}"),
            format!("> {separator}"),
        ],
        RevisionLogId::Heading(h) => {
            vec![format!("## {}", h.trim()), String::new(), header, separator]
        }
    }
}

/// The headings the template places after its log's identity line.
fn headings_after_log(template: &str, id: &RevisionLogId) -> Vec<String> {
    let lines: Vec<&str> = template.lines().collect();
    let Some(marker) = crate::rules::log_marker_line(&lines, id) else {
        return Vec::new();
    };
    body_headings(template)
        .headings
        .into_iter()
        .filter(|(line, _)| *line > marker + 1)
        .map(|(_, text)| text)
        .collect()
}

struct Body {
    /// Each Markdown heading outside a fence, as its 1-based line and text.
    headings: Vec<(usize, String)>,
    /// The line of the fence still open at the end, if one is.
    open_fence: Option<usize>,
}

/// The headings below the header, read with the same fence tracking as the
/// revision-log reader.
fn body_headings(content: &str) -> Body {
    let lines: Vec<&str> = content.lines().collect();
    let first = lines
        .first()
        .map(|l| l.trim_start_matches('\u{feff}').trim_end());
    let start = if first == Some("---") {
        lines
            .iter()
            .skip(1)
            .position(|l| l.trim_end() == "---")
            .map_or(lines.len(), |p| p + 2)
    } else {
        0
    };
    let mut fences = Fences::default();
    let mut headings = Vec::new();
    let mut opened = None;
    for (i, line) in lines.iter().enumerate().skip(start) {
        let was_open = fences.is_open();
        if !fences.is_prose(line) {
            if !was_open && fences.is_open() {
                opened = Some(i + 1);
            }
            continue;
        }
        let t = line.trim();
        let hashes = t.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
            headings.push((i + 1, t[hashes..].trim().to_string()));
        }
    }
    Body {
        headings,
        open_fence: opened.filter(|_| fences.is_open()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n";

    #[test]
    fn plan_reads_a_present_log_a_missing_one_and_a_bodiless_record() {
        let (id, shape) = (RevisionLogId::default(), RevisionLogShape::default());
        assert_eq!(
            plan(&format!("# T\n\n{LOG}"), &id, &shape),
            LogPlan::Present
        );
        assert_eq!(plan("---\nA: b\n---\n# T\n", &id, &shape), LogPlan::Create);
        assert_eq!(plan("---\nA: b\n---\n", &id, &shape), LogPlan::NoBody);
        assert_eq!(
            plan("---\nA: b\n---\n```\n# not a heading\n```\n", &id, &shape),
            LogPlan::NoBody
        );
    }

    #[test]
    fn a_declared_section_is_built_from_the_identity_and_columns() {
        let shape = RevisionLogShape {
            date_column: Some("When".into()),
            class_column: Some("Kind".into()),
            ..Default::default()
        };
        assert_eq!(
            section(&RevisionLogId::Heading("History".into()), &shape, None),
            [
                "## History",
                "",
                "| When | Change | Kind |",
                "|---|---|---|"
            ]
        );
        assert_eq!(
            section(
                &RevisionLogId::default(),
                &RevisionLogShape::default(),
                None
            )
            .join("\n")
                + "\n",
            LOG
        );
    }

    #[test]
    fn a_template_section_is_copied_without_its_rows() {
        let template = "# N\n\n> **Revision log**\n>\n> | Date | Note | Class |\n> |:--|:--|:--:|\n> | YYYY-MM-DD | Drafted. | **structural** |\n";
        assert_eq!(
            section(
                &RevisionLogId::default(),
                &RevisionLogShape::default(),
                Some(template)
            ),
            [
                "> **Revision log**",
                ">",
                "> | Date | Note | Class |",
                "> |:--|:--|:--:|"
            ]
        );
    }

    #[test]
    fn insert_appends_after_one_blank_line_and_keeps_crlf() {
        let id = RevisionLogId::default();
        let s = vec!["> X".to_string()];
        assert_eq!(insert("# T\n", &s, &id, None), "# T\n\n> X\n");
        assert_eq!(insert("# T\n\n", &s, &id, None), "# T\n\n> X\n");
        assert_eq!(insert("# T\r\n", &s, &id, None), "# T\r\n\r\n> X\r\n");
    }

    #[test]
    fn insert_goes_before_the_first_heading_the_template_places_after_the_log() {
        let id = RevisionLogId::Heading("History".into());
        let template = "# N\n\n## What\n\n## History\n\n| Date | Change | Class |\n|---|---|---|\n\n## Notes\n";
        let s = vec!["## History".to_string()];
        assert_eq!(
            insert(
                "# T\n\n## What\n\nA.\n\n## Notes\n\nB.\n",
                &s,
                &id,
                Some(template)
            ),
            "# T\n\n## What\n\nA.\n\n## History\n\n## Notes\n\nB.\n"
        );
        assert_eq!(
            insert("# T\n\n## What\nA.\n", &s, &id, Some(template)),
            "# T\n\n## What\nA.\n\n## History\n"
        );
    }

    #[test]
    fn a_record_ending_inside_a_fence_is_refused_naming_it() {
        let (id, shape) = (RevisionLogId::default(), RevisionLogShape::default());
        let open = "---\nA: b\n---\n# T\n\n```\ncode\n";
        assert_eq!(plan(open, &id, &shape), LogPlan::OpenFence(6));
        assert!(refusal(&LogPlan::OpenFence(6), &id, "re-baseline")
            .unwrap()
            .contains("line 6"));
        let quoted = "# T\n\n> ```\n> code\n> ```\n";
        assert_eq!(plan(quoted, &id, &shape), LogPlan::Create);
        let mixed = "```\n~~~\n# not a heading\n```\n# T\n";
        assert_eq!(plan(mixed, &id, &shape), LogPlan::Create);
        assert_eq!(
            plan("```\n~~~\n# not a heading\n", &id, &shape),
            LogPlan::OpenFence(1)
        );
    }

    #[test]
    fn the_declared_narrative_column_names_the_middle_header_cell() {
        let shape = RevisionLogShape {
            narrative_column: "What changed".into(),
            ..Default::default()
        };
        assert_eq!(
            section(&RevisionLogId::default(), &shape, None)[2],
            "> | Date | What changed | Class |"
        );
    }
}
