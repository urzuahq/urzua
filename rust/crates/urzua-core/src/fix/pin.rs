//! `urzua fix --pin --replace` and `--remove`: moving or dropping a pinned
//! locator with an attributed row, so `diff.pin-changed-unattested` can tell a
//! reviewed change of evidence from a hand edit.

use super::{append_log_rows, Attestation};
use crate::line_edit::LineDoc;

/// What starts the row a replacement writes; `parse_replaced_row` reads it.
pub const REPLACED_ROW_PREFIX: &str = "Pin moved to another locator (attested by ";
/// What starts the row a removal writes; `parse_removed_row` reads it.
pub const REMOVED_ROW_PREFIX: &str = "Pinned locator removed (attested by ";

/// One replacement a row names, hashes in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replaced {
    pub from_locator: String,
    pub from_hash: String,
    pub to_locator: String,
    pub to_hash: String,
}

/// A pinned entry as written: `category:spelling@sha256:hash`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedEntry {
    pub category: String,
    pub spelling: String,
    pub hash: String,
}

impl PinnedEntry {
    pub fn text(&self) -> String {
        format!("{}:{}@sha256:{}", self.category, self.spelling, self.hash)
    }
}

/// The text before the revision log, where evidence lives; edits never touch
/// the log's rows, which may quote a pin.
fn evidence_end(content: &str) -> usize {
    content.find("**Revision log**").unwrap_or(content.len())
}

fn label_why(reason: &str) -> String {
    format!("Reason: {reason} **Why:** the evidence a claim rests on changed and a person or agent attested why, to be confirmed by a reviewer in the PR.")
}

/// Replaces every occurrence of `old` with `new` in the evidence and appends
/// the attributed row.
pub fn apply_replace(
    content: &str,
    old: &PinnedEntry,
    new: &PinnedEntry,
    (today, attestation, reason): (&str, &Attestation, &str),
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Result<String, String> {
    let end = evidence_end(content);
    let (head, tail) = content.split_at(end);
    let needle = old.text();
    if !head.contains(&needle) {
        return Err(format!("'{needle}' is not in the record's evidence"));
    }
    let edited = format!("{}{tail}", head.replace(&needle, &new.text()));
    let row = format!(
        "{REPLACED_ROW_PREFIX}{}): `{}`@sha256:{} -> `{}`@sha256:{}. {}",
        attestation.label(),
        locator_name(old),
        old.hash,
        locator_name(new),
        new.hash,
        label_why(reason)
    );
    let mut doc = LineDoc::parse(&edited);
    append_log_rows(
        &mut doc,
        log_id,
        &shape.for_evidence(),
        today,
        &[row],
        "pin move",
    )?;
    Ok(doc.to_string())
}

/// Where `--remove` edits, and the Done-means items that still cite the pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removal {
    /// The 1-based, inclusive line range edited.
    pub span: (usize, usize),
    /// `(item number, line)` of each item whose evidence clause cites the pin
    /// and is not edited.
    pub remaining: Vec<(usize, usize)>,
}

/// The 1-based, inclusive line range of `field` in `record`'s header: from its
/// own line to the line before the next field, or to the end of the header.
/// The line is the header's best-effort one, so it is checked against `text`:
/// `None` when the record has no such field or that line does not begin it.
fn field_span(record: &crate::record::Record, text: &str, field: &str) -> Option<(usize, usize)> {
    let fields = &record.header.fields;
    let start = fields.iter().find(|f| f.key.as_str() == field)?.line;
    if !text.lines().nth(start.checked_sub(1)?)?.contains(field) {
        return None;
    }
    let next = fields.iter().map(|f| f.line).filter(|l| *l > start).min();
    let end = match (next, record.header.region) {
        (Some(n), _) => n - 1,
        (None, Some((_, last))) => last,
        (None, None) => start,
    };
    Some((start, end.max(start)))
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

/// The Done-means items whose evidence clause cites `old`, as `(n, clause line)`.
fn items_citing(record: &crate::record::Record, old: &PinnedEntry) -> Vec<(usize, usize)> {
    let Some(items) = &record.items else {
        return Vec::new();
    };
    items
        .items
        .iter()
        .filter_map(|item| {
            let clause = item.evidence.as_ref().filter(|c| c.valid)?;
            crate::locator::realized_by_locators(&clause.text)
                .into_iter()
                .any(|e| {
                    e.category == old.category
                        && e.parsed.is_ok_and(|l| {
                            l.spelling == old.spelling
                                && matches!(&l.anchor, Some(crate::locator::Anchor::Pin(h)) if *h == old.hash)
                        })
                })
                .then_some((item.n, clause.line))
        })
        .collect()
}

/// Where `--remove` of `old` edits in `text`, read as `record`. The evidence
/// field's copy is the one removed, and a copy in a Done-means item stays: the
/// maintainer's decision of 2026-11-25 (`BUG-232`). A pin the field does not
/// hold is removed from the first item that cites it and the rest are named.
pub fn locate_removal(
    record: &crate::record::Record,
    text: &str,
    field: &str,
    old: &PinnedEntry,
) -> Result<Removal, String> {
    let needle = old.text();
    let items = items_citing(record, old);
    let in_field = field_span(record, text, field).filter(|(s, e)| {
        text.lines()
            .skip(s - 1)
            .take(e + 1 - s)
            .any(|l| !is_comment(l) && l.contains(&needle))
    });
    let field_holds = record
        .header
        .get(field)
        .is_some_and(|v| v.contains(&needle));
    match (in_field, field_holds, items.split_first()) {
        (Some(span), _, _) => Ok(Removal {
            span,
            remaining: items,
        }),
        (None, true, _) => Err(format!(
            "'{needle}' is in {field}, but its lines in the record cannot be located (a quoted or flow-style key, for one); remove it from {field} by hand"
        )),
        (None, false, Some((first, rest))) => Ok(Removal {
            span: (first.1, first.1),
            remaining: rest.to_vec(),
        }),
        (None, false, None) => Err(format!("'{needle}' is not in the record's evidence")),
    }
}

/// Removes `needle` and its separating comma from `text`, whether the entries
/// share a line or a folded field puts each on its own. A match on a comment
/// line is not an entry.
fn remove_entry(text: &str, needle: &str) -> Option<String> {
    let mut from = 0;
    let at = loop {
        let at = from + text[from..].find(needle)?;
        let line_start = text[..at].rfind('\n').map_or(0, |n| n + 1);
        if !is_comment(&text[line_start..]) {
            break at;
        }
        from = at + needle.len();
    };
    let (before, after) = (&text[..at], &text[at + needle.len()..]);
    if let Some(kept) = before.trim_end().strip_suffix(',') {
        return Some(format!("{kept}{after}"));
    }
    let rest = after.trim_start().strip_prefix(',')?;
    let rest = rest.trim_start_matches([' ', '\t']);
    let next = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'));
    match (before.rfind('\n'), next) {
        (Some(nl), Some(next)) if before[nl + 1..].trim().is_empty() => {
            Some(format!("{}{next}", &before[..=nl]))
        }
        _ => Some(format!("{before}{rest}")),
    }
}

/// Removes the pinned entry `old` (with its separating comma) from the lines
/// `span` names, and appends the attributed row.
pub fn apply_remove(
    content: &str,
    old: &PinnedEntry,
    span: (usize, usize),
    (today, attestation, reason): (&str, &Attestation, &str),
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Result<String, String> {
    let needle = old.text();
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let (start, end) = (span.0.max(1) - 1, span.1.min(lines.len()));
    let not_there = || format!("'{needle}' is not in the lines it is removed from");
    if start >= end {
        return Err(not_there());
    }
    let mut field = lines[start..end].concat();
    if !field.contains(&needle) {
        return Err(not_there());
    }
    let sole = || {
        format!("'{needle}' is not one of several evidence entries; remove a record's only locator by hand")
    };
    field = remove_entry(&field, &needle).ok_or_else(sole)?;
    while let Some(next) = remove_entry(&field, &needle) {
        field = next;
    }
    let edited = format!(
        "{}{field}{}",
        lines[..start].concat(),
        lines[end..].concat()
    );
    let row = format!(
        "{REMOVED_ROW_PREFIX}{}): `{}`@sha256:{}. {}",
        attestation.label(),
        locator_name(old),
        old.hash,
        label_why(reason)
    );
    let mut doc = LineDoc::parse(&edited);
    append_log_rows(
        &mut doc,
        log_id,
        &shape.for_evidence(),
        today,
        &[row],
        "pin removal",
    )?;
    Ok(doc.to_string())
}

/// The locator as the rule names it: the parsed display (path and scope).
fn locator_name(entry: &PinnedEntry) -> String {
    crate::locator::parse_locator(&entry.spelling)
        .map(|l| l.display())
        .unwrap_or_else(|_| entry.spelling.clone())
}

fn after<'a>(row: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = &row[row.find(prefix)? + prefix.len()..];
    let rest = rest.split_once("): ")?.1;
    Some(rest.split_once(". Reason: ")?.0)
}

fn quoted_pin(text: &str) -> Option<(String, String)> {
    let (loc, rest) = text.trim().strip_prefix('`')?.split_once('`')?;
    Some((
        loc.to_string(),
        rest.strip_prefix("@sha256:")?.trim().to_string(),
    ))
}

/// The replacement a row names, or `None` when `row` is not one.
pub fn parse_replaced_row(row: &str) -> Option<Replaced> {
    let (from, to) = after(row, REPLACED_ROW_PREFIX)?.split_once(" -> ")?;
    let (from_locator, from_hash) = quoted_pin(from)?;
    let (to_locator, to_hash) = quoted_pin(to)?;
    Some(Replaced {
        from_locator,
        from_hash,
        to_locator,
        to_hash,
    })
}

/// The `(locator, hash)` a removal row names, or `None` when `row` is not one.
pub fn parse_removed_row(row: &str) -> Option<(String, String)> {
    quoted_pin(after(row, REMOVED_ROW_PREFIX)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_read_back_as_written() {
        let r = format!("2026-11-16 | {REPLACED_ROW_PREFIX}t, identity: asserted, actor: agent): `src/a.rs`@sha256:aaa -> `src/a.rs#f`@sha256:bbb. Reason: r **Why:** w | **evidence**");
        assert_eq!(
            parse_replaced_row(&r).unwrap(),
            Replaced {
                from_locator: "src/a.rs".into(),
                from_hash: "aaa".into(),
                to_locator: "src/a.rs#f".into(),
                to_hash: "bbb".into()
            }
        );
        let r = format!("2026-11-16 | {REMOVED_ROW_PREFIX}t, identity: asserted, actor: agent): `src/a.rs`@sha256:aaa. Reason: r **Why:** w | **evidence**");
        assert_eq!(
            parse_removed_row(&r).unwrap(),
            ("src/a.rs".into(), "aaa".into())
        );
        assert!(parse_replaced_row("Pin moved by hand").is_none());
    }
}
