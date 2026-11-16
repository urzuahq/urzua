//! A revision log for an evidence row to land in, created when the record has
//! none: a re-pin or first pin is a tool-written row, and a log a person typed
//! by hand to receive it is the edit `diff.pin-changed-unattested` exists to
//! tell apart from one.

use crate::config::{RevisionLogId, RevisionLogShape};
use crate::line_edit::LineDoc;

/// What an evidence write does about the record's revision log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogPlan {
    /// The record has a log section; the row goes there.
    Present,
    /// The record has none; the write creates it, empty, at the end.
    Create,
    /// The record has no body section for a new log to follow.
    NoBody,
}

/// Whether `content` has a revision log, needs one created, or has nothing a
/// new one could follow.
pub fn plan(content: &str, id: &RevisionLogId, shape: &RevisionLogShape) -> LogPlan {
    if crate::rules::find_revision_log_entries(content, id, shape).is_some() {
        LogPlan::Present
    } else if has_body_section(content) {
        LogPlan::Create
    } else {
        LogPlan::NoBody
    }
}

/// The refusal for [`LogPlan::NoBody`].
pub fn no_body_refusal(id: &RevisionLogId, verb: &str) -> String {
    format!(
        "record has no body section for a new Revision log ({}) to follow -- refusing to {verb}; give the record a heading first",
        id.describe()
    )
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

/// `content` with `section` appended after a blank line.
pub fn append(content: &str, section: &[String]) -> String {
    let mut doc = LineDoc::parse(content);
    let mut at = content.lines().count();
    if content.lines().last().is_some_and(|l| !l.trim().is_empty()) {
        doc.insert_after(at, String::new());
        at += 1;
    }
    for line in section {
        doc.insert_after(at, line.clone());
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
    let header = format!("| {date} | Change | {class} |");
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

/// Whether the body below the header holds a Markdown heading outside a code
/// fence.
fn has_body_section(content: &str) -> bool {
    let mut lines = content.trim_start_matches('\u{feff}').lines().peekable();
    if lines.peek().is_some_and(|l| l.trim_end() == "---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim_end() == "---" {
                break;
            }
        }
    }
    let mut fenced = false;
    for line in lines {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        let hashes = t.chars().take_while(|c| *c == '#').count();
        if !fenced && (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
            return true;
        }
    }
    false
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
    fn append_separates_the_section_by_one_blank_line_and_keeps_crlf() {
        let s = vec!["> X".to_string()];
        assert_eq!(append("# T\n", &s), "# T\n\n> X\n");
        assert_eq!(append("# T\n\n", &s), "# T\n\n> X\n");
        assert_eq!(append("# T\r\n", &s), "# T\r\n\r\n> X\r\n");
    }
}
