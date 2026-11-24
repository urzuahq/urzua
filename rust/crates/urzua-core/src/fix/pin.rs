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

/// Removes the first copy of the pinned entry `old` (with its separating
/// comma) from the evidence and appends the attributed row.
pub fn apply_remove(
    content: &str,
    old: &PinnedEntry,
    (today, attestation, reason): (&str, &Attestation, &str),
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Result<String, String> {
    let end = evidence_end(content);
    let (head, tail) = content.split_at(end);
    let needle = old.text();
    // The field precedes the body, so its copy is the first; a copy in a
    // Done-means item (ADR-96) is the item's own criterion and stays.
    let removed = head
        .find(&needle)
        .and_then(|at| {
            let (before, after) = (&head[..at], &head[at + needle.len()..]);
            if let Some(kept) = before.strip_suffix(", ") {
                Some(format!("{kept}{after}"))
            } else {
                after.strip_prefix(", ").map(|rest| format!("{before}{rest}"))
            }
        })
        .ok_or_else(|| {
            format!("'{needle}' is not one of several evidence entries; remove a record's only locator by hand")
        })?;
    let edited = format!("{removed}{tail}");
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
