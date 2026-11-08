//! A record's Done-means items (`ADR-96`, `MILE-214`): the declaration
//! `statuses.delivery.items` and the pure parser of the item grammar.
//!
//! Each list item of the type's declared section names its own evidence in the
//! existing `Realized-by` grammar, or carries a waiver. The engine reads three
//! declared words (the clause separator and the two markers), splits a line at
//! the first occurrence of one declared sequence, and hands the clause to the
//! parsers that already exist. Nothing in a clause is interpreted by position,
//! regex or guess; every deviation is a shape issue, never skipped.
//!
//! Pure: the caller reads the section and hands the text in.

// Implements: ADR-96

use serde::Deserialize;

/// `statuses.delivery.items` (`ADR-96` section 4).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsDecl {
    /// The section whose list items are the Done-means items. Must be a
    /// section the type requires.
    pub section: String,
    /// The text before a clause marker: ` — ` in this repository.
    pub clause_separator: String,
    /// Starts the evidence clause: `evidence:`.
    pub evidence_marker: String,
    /// Starts the waiver clause: `waived:`.
    pub waiver_marker: String,
    /// The status a partly delivered record is repaired to or held at.
    pub partial_status: String,
    /// The tiers at which an item's own locators resolve it. Omitted means the
    /// type's `delivered_tiers`.
    #[serde(default)]
    pub resolving_tiers: Option<Vec<String>>,
    #[serde(default)]
    pub waiver_categories: Vec<WaiverCategory>,
    /// Opt-in per-record kind (`ADR-96`, amendment of 2026-11-07): a field of
    /// the record names its kind, and each kind says which tiers count as
    /// delivered. Omitted keeps one rule for every record.
    #[serde(default)]
    pub kind: Option<KindDecl>,
    /// Opt-in placeholder vocabulary (`ADR-96`, amendment of 2026-11-08): item
    /// texts that mean "not yet written", in the shape and with the matching of
    /// `statuses.pending` (a whole value, trimmed and case-insensitive, or a
    /// `{pattern: ...}`). An item whose text matches is an unfilled placeholder:
    /// it never resolves, carries no evidence and no waiver, and
    /// `status.done-items-shape` reports it. Omitted keeps today's behaviour.
    #[serde(default)]
    pub pending: Option<Vec<crate::status_model::PendingEntry>>,
}

/// `statuses.delivery.items.kind`: the field that holds a record's kind and the
/// vocabulary of kinds it may hold.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindDecl {
    /// The header field holding the kind. It must be a field the type declares.
    pub field: String,
    pub kinds: Vec<KindEntry>,
}

/// One kind of the vocabulary. A kind that names nothing but its `name` is
/// judged by the type's own `delivered_tiers`, as a record with no kind is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindEntry {
    pub name: String,
    /// The tiers at which an item counts as delivered for this kind. Omitted
    /// means the type's `delivered_tiers`. Each must also be a `resolving_tiers`
    /// tier, or no item could reach it.
    #[serde(default)]
    pub delivered_tiers: Option<Vec<String>>,
    /// How a record an item cites is held: `success` means a locator that
    /// resolves to a record counts only while that record's status is one its
    /// own type declares `success`, so a Proposed decision or a Draft spec
    /// evidences nothing. Omitted reads no cited record's status.
    #[serde(default)]
    pub cited_records: Option<CitedRecords>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CitedRecords {
    Success,
}

/// One declared waiver category, in the shape `ADR-86` gave the permanent
/// waiver plus an optional `expires: required` flag.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaiverCategory {
    pub name: String,
    #[serde(default)]
    pub cite: Option<Requirement>,
    #[serde(default)]
    pub expires: Option<Requirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Requirement {
    Required,
}

impl WaiverCategory {
    pub fn cite_required(&self) -> bool {
        self.cite.is_some()
    }

    pub fn expires_required(&self) -> bool {
        self.expires.is_some()
    }
}

impl ItemsDecl {
    /// The declared entry for a kind value, `None` for an undeclared one.
    pub fn kind_entry(&self, value: &str) -> Option<&KindEntry> {
        self.kind
            .as_ref()?
            .kinds
            .iter()
            .find(|k| k.name == value.trim())
    }

    /// Whether an item's text is a declared placeholder. `false` when the type
    /// declares no `pending`.
    pub fn is_placeholder(&self, text: &str) -> bool {
        let text = text.trim();
        self.pending
            .as_ref()
            .is_some_and(|p| p.iter().any(|e| e.matches(text)))
    }

    pub fn category(&self, name: &str) -> Option<&WaiverCategory> {
        self.waiver_categories.iter().find(|c| c.name == name)
    }
}

/// The evidence clause of an item: the text after the evidence marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    /// The 1-based file line the clause is on.
    pub line: usize,
    /// The locators text, trimmed (`category:path, category:path`).
    pub text: String,
    /// Whether it is entirely locators; a clause that is not is also a shape
    /// issue, and the item reads as unresolved.
    pub valid: bool,
}

/// A valid waiver clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    pub line: usize,
    pub category: String,
    pub cite: Option<String>,
    pub date: Option<String>,
    pub reason: String,
}

impl Waiver {
    /// The waiver's expiry date has come. `today` is empty when the caller has
    /// none, and nothing is then judged expired (a waived item stays excused).
    pub fn is_expired(&self, today: &str) -> bool {
        !today.is_empty() && self.date.as_deref().is_some_and(|d| d <= today)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Position in the section, from 1.
    pub n: usize,
    /// The file line of the item's first line.
    pub line: usize,
    /// The item text: its first line before any clause.
    pub text: String,
    pub evidence: Option<Clause>,
    pub waiver: Option<Waiver>,
    /// The item text matches the declared `pending` vocabulary: an unfilled
    /// placeholder. Its clauses are dropped, so it is neither evidence nor
    /// waived.
    pub placeholder: bool,
}

/// One deviation from the grammar, reported by `status.done-items-shape`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeIssue {
    pub line: usize,
    pub item: Option<usize>,
    pub message: String,
}

/// A record's parsed Done-means section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DoneItems {
    /// The heading line of the section, `None` when the record has none.
    pub section_line: Option<usize>,
    pub items: Vec<Item>,
    pub issues: Vec<ShapeIssue>,
}

impl DoneItems {
    /// Every item's evidence locators text, in item order: what the readers of
    /// the evidence field add to `Realized-by` (`ADR-96` section 5).
    pub fn locator_text(&self) -> Option<String> {
        let parts: Vec<&str> = self
            .items
            .iter()
            .filter_map(|i| i.evidence.as_ref())
            .filter(|c| c.valid)
            .map(|c| c.text.as_str())
            .collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// The file lines that carry a valid evidence clause: the ones
    /// [`DoneItems::locator_text`] reads.
    pub fn valid_evidence_lines(&self) -> Vec<usize> {
        self.items
            .iter()
            .filter_map(|i| i.evidence.as_ref())
            .filter(|c| c.valid)
            .map(|c| c.line)
            .collect()
    }

    /// The file lines that carry an evidence clause.
    pub fn evidence_lines(&self) -> Vec<usize> {
        self.items
            .iter()
            .filter_map(|i| i.evidence.as_ref())
            .map(|c| c.line)
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    First,
    Continuation,
    Sub,
    Blank,
}

struct ItemLine {
    number: usize,
    kind: LineKind,
    text: String,
}

struct Builder {
    first_line: usize,
    lines: Vec<ItemLine>,
}

/// The width of a leading run of spaces, a tab counting as four.
fn indent_of(line: &str) -> (usize, &str) {
    let mut width = 0;
    for (at, c) in line.char_indices() {
        match c {
            ' ' => width += 1,
            '\t' => width += 4,
            _ => return (width, &line[at..]),
        }
    }
    (width, "")
}

/// The text after a list marker (`-`, `*`, `+`, `1.`, `1)`) and its space, or
/// `None` when the line starts no list item.
fn after_marker(rest: &str) -> Option<&str> {
    let mut chars = rest.chars();
    let first = chars.next()?;
    if matches!(first, '-' | '*' | '+') {
        return rest[1..].strip_prefix(' ');
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let after = &rest[digits..];
    let after = after
        .strip_prefix('.')
        .or_else(|| after.strip_prefix(')'))?;
    after.strip_prefix(' ')
}

fn fence_opening(rest_after_indent: &str, indent: usize) -> Option<(char, usize)> {
    if indent > 3 {
        return None;
    }
    let marker = rest_after_indent
        .chars()
        .next()
        .filter(|c| *c == '`' || *c == '~')?;
    let len = rest_after_indent
        .chars()
        .take_while(|c| *c == marker)
        .count();
    (len >= 3).then_some((marker, len))
}

/// Parses the text of the declared section. `first_line` is the 1-based file
/// line of the first line of `body`. `section_line` is the heading's line.
pub fn parse_items(
    body: &str,
    first_line: usize,
    section_line: usize,
    decl: &ItemsDecl,
) -> DoneItems {
    let mut out = DoneItems {
        section_line: Some(section_line),
        ..Default::default()
    };
    let mut builders: Vec<Builder> = Vec::new();
    let mut current: Option<Builder> = None;
    let mut item_indent: Option<usize> = None;
    let mut fence: Option<(char, usize)> = None;
    let mut orphans: Vec<(usize, String)> = Vec::new();

    for (i, raw) in body.lines().enumerate() {
        let number = first_line + i;
        let line = raw.trim_end();
        let (indent, rest) = indent_of(line);
        if let Some((marker, len)) = fence {
            let run = rest.chars().take_while(|c| *c == marker).count();
            if indent <= 3 && run >= len && rest[run..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(opened) = fence_opening(rest, indent) {
            fence = Some(opened);
            continue;
        }
        if rest.is_empty() {
            if let Some(b) = current.as_mut() {
                b.lines.push(ItemLine {
                    number,
                    kind: LineKind::Blank,
                    text: String::new(),
                });
            }
            continue;
        }
        if let Some(text) = after_marker(rest) {
            let base = *item_indent.get_or_insert(indent);
            if indent == base {
                if let Some(done) = current.take() {
                    builders.push(done);
                }
                current = Some(Builder {
                    first_line: number,
                    lines: vec![ItemLine {
                        number,
                        kind: LineKind::First,
                        text: text.to_string(),
                    }],
                });
            } else if indent > base {
                if let Some(b) = current.as_mut() {
                    b.lines.push(ItemLine {
                        number,
                        kind: LineKind::Sub,
                        text: text.to_string(),
                    });
                }
            } else {
                if let Some(done) = current.take() {
                    builders.push(done);
                }
                out.issues.push(ShapeIssue {
                    line: number,
                    item: None,
                    message: format!(
                        "a list marker is indented less than the item indent ({base} spaces), so it is neither an item nor a sub-item"
                    ),
                });
            }
            continue;
        }
        // Plain text. Deeper than the item marker it continues the item.
        match (current.as_mut(), item_indent) {
            (Some(b), Some(base)) if indent > base => b.lines.push(ItemLine {
                number,
                kind: LineKind::Continuation,
                text: rest.to_string(),
            }),
            _ => {
                if let Some(done) = current.take() {
                    builders.push(done);
                }
                if rest.starts_with(decl.evidence_marker.as_str())
                    || rest.starts_with(decl.waiver_marker.as_str())
                {
                    orphans.push((number, rest.to_string()));
                }
            }
        }
    }
    if let Some(done) = current.take() {
        builders.push(done);
    }

    for (number, text) in orphans {
        out.issues.push(ShapeIssue {
            line: number,
            item: None,
            message: format!(
                "'{}' starts a clause on a line that belongs to no item: a final-line clause is indented under the item it ends",
                text.split_whitespace().next().unwrap_or_default()
            ),
        });
    }
    if builders.is_empty() {
        out.issues.push(ShapeIssue {
            line: section_line,
            item: None,
            message: "the section holds no list item (prose only); write each criterion as a list item that ends in its evidence".to_string(),
        });
    }
    for (index, builder) in builders.into_iter().enumerate() {
        let item = build_item(index + 1, builder, decl, &mut out.issues);
        out.items.push(item);
    }
    out
}

/// Where in `line` a clause starts: the first occurrence of the separator
/// followed by the evidence marker or the waiver marker.
fn first_clause(line: &str, decl: &ItemsDecl) -> Option<(usize, bool)> {
    let ev = format!("{}{}", decl.clause_separator, decl.evidence_marker);
    let wv = format!("{}{}", decl.clause_separator, decl.waiver_marker);
    match (line.find(&ev), line.find(&wv)) {
        (Some(e), Some(w)) if e <= w => Some((e, true)),
        (Some(_), Some(w)) => Some((w, false)),
        (Some(e), None) => Some((e, true)),
        (None, Some(w)) => Some((w, false)),
        (None, None) => None,
    }
}

fn build_item(n: usize, builder: Builder, decl: &ItemsDecl, issues: &mut Vec<ShapeIssue>) -> Item {
    let mut issue = |line: usize, message: String| {
        issues.push(ShapeIssue {
            line,
            item: Some(n),
            message,
        })
    };
    let first = &builder.lines[0];
    let mut text = first.text.clone();
    let mut evidence_text: Option<(usize, String)> = None;
    let mut waiver_text: Option<(usize, String)> = None;
    let mut on_first_line = false;
    if let Some((at, is_evidence)) = first_clause(&first.text, decl) {
        on_first_line = true;
        text = first.text[..at].to_string();
        let rest = &first.text[at + decl.clause_separator.len()..];
        if is_evidence {
            let after = &rest[decl.evidence_marker.len()..];
            let wv = format!("{}{}", decl.clause_separator, decl.waiver_marker);
            match after.find(&wv) {
                Some(w) => {
                    evidence_text = Some((first.number, after[..w].trim().to_string()));
                    waiver_text = Some((first.number, after[w + wv.len()..].trim().to_string()));
                }
                None => evidence_text = Some((first.number, after.trim().to_string())),
            }
        } else {
            waiver_text = Some((
                first.number,
                rest[decl.waiver_marker.len()..].trim().to_string(),
            ));
        }
    }

    // The final one or two lines, each starting at its marker.
    let tail: Vec<&ItemLine> = builder
        .lines
        .iter()
        .skip(1)
        .filter(|l| l.kind != LineKind::Blank)
        .collect();
    let starts = |l: &&ItemLine, marker: &str| {
        l.kind == LineKind::Continuation && l.text.starts_with(marker)
    };
    let mut final_ev: Option<&ItemLine> = None;
    let mut final_wv: Option<&ItemLine> = None;
    if let Some(last) = tail.last() {
        if starts(last, &decl.waiver_marker) {
            final_wv = Some(last);
            if tail.len() >= 2 && starts(&tail[tail.len() - 2], &decl.evidence_marker) {
                final_ev = Some(tail[tail.len() - 2]);
            }
        } else if starts(last, &decl.evidence_marker) {
            final_ev = Some(last);
            if tail.len() >= 2 && starts(&tail[tail.len() - 2], &decl.waiver_marker) {
                issue(
                    tail[tail.len() - 2].number,
                    "the waiver clause is before the evidence clause; the evidence clause comes first".to_string(),
                );
                final_wv = Some(tail[tail.len() - 2]);
            }
        }
    }
    if on_first_line && (final_ev.is_some() || final_wv.is_some()) {
        let at = final_ev.or(final_wv).map_or(first.number, |l| l.number);
        issue(
            at,
            "clauses are on the item's first line and on final lines at once; put them in one place".to_string(),
        );
    } else {
        if let Some(l) = final_ev {
            evidence_text = Some((
                l.number,
                l.text[decl.evidence_marker.len()..].trim().to_string(),
            ));
        }
        if let Some(l) = final_wv {
            waiver_text = Some((
                l.number,
                l.text[decl.waiver_marker.len()..].trim().to_string(),
            ));
        }
    }

    let placeholder = judge_text(&text, builder.first_line, decl, &mut issue);
    let evidence = evidence_text.map(|(line, clause)| {
        let problems = locator_problems(&clause);
        for p in &problems {
            issue(line, p.clone());
        }
        Clause {
            line,
            text: clause,
            valid: problems.is_empty(),
        }
    });
    let waiver =
        waiver_text.and_then(|(line, clause)| parse_waiver(line, &clause, decl, &mut issue));
    // A placeholder is not evidence and cannot be waived.
    let (evidence, waiver) = if placeholder {
        (None, None)
    } else {
        (evidence, waiver)
    };
    Item {
        n,
        line: builder.first_line,
        text: text.trim().to_string(),
        evidence,
        waiver,
        placeholder,
    }
}

/// The shape issues of an item's own text: a leading checkbox, and a text that
/// matches the declared `pending` vocabulary. Returns whether it is a placeholder.
fn judge_text(
    text: &str,
    line: usize,
    decl: &ItemsDecl,
    issue: &mut impl FnMut(usize, String),
) -> bool {
    let lead = text.trim_start();
    if lead.starts_with("[x]") || lead.starts_with("[X]") || lead.starts_with("[ ]") {
        issue(
            line,
            "the item starts with a checkbox; state is computed from the evidence, not ticked"
                .to_string(),
        );
    }
    let placeholder = decl.is_placeholder(text);
    if placeholder {
        issue(line, placeholder_message(text));
    }
    placeholder
}

fn placeholder_message(text: &str) -> String {
    format!(
        "the item '{}' is an unfilled placeholder (it matches the declared `pending` vocabulary); replace it with a real criterion that names its evidence, or delete the item",
        text.trim()
    )
}

/// What is wrong with an evidence clause as a list of locators, empty when it
/// is entirely locators. The splitter is the `Realized-by` one (`BUG-129`).
pub fn locator_problems(clause: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if clause.trim().is_empty() {
        problems.push("the evidence clause names no locator".to_string());
        return problems;
    }
    if let Some(lead) = crate::locator::realized_by_unrecognised(clause) {
        problems.push(format!(
            "the evidence clause holds '{lead}', text that belongs to no locator entry: write `category:path` with no backticks, quotes or link (grammar: {})",
            crate::locator::GRAMMAR
        ));
    }
    for (_, raw) in crate::locator::realized_by_entries(clause) {
        let raw = raw.trim();
        if let Some(at) = raw.find(char::is_whitespace) {
            problems.push(format!(
                "the locator '{}' is followed by stray text '{}': a locator holds no whitespace, so write the explanation outside the evidence clause",
                &raw[..at],
                raw[at..].trim()
            ));
        }
    }
    problems
}

fn is_record_id(text: &str) -> bool {
    let Some((prefix, number)) = text.rsplit_once('-') else {
        return false;
    };
    !prefix.is_empty()
        && prefix.chars().all(|c| c.is_ascii_alphanumeric())
        && prefix.starts_with(|c: char| c.is_ascii_alphabetic())
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

fn parse_waiver(
    line: usize,
    clause: &str,
    decl: &ItemsDecl,
    issue: &mut impl FnMut(usize, String),
) -> Option<Waiver> {
    let Some((head, reason)) = clause.split_once(" -- ") else {
        issue(
            line,
            format!(
                "the waiver clause '{}{}' has no ` -- <reason>`; the reason is required prose",
                decl.waiver_marker,
                if clause.is_empty() {
                    String::new()
                } else {
                    format!(" {clause}")
                }
            ),
        );
        return None;
    };
    let reason = reason.trim();
    let (category, parenthesis) = match head.split_once(" (") {
        Some((category, rest)) => match rest.strip_suffix(')') {
            Some(inner) => (category.trim(), Some(inner)),
            None => {
                issue(
                    line,
                    format!("the waiver's parenthesis '({rest}' is not closed"),
                );
                return None;
            }
        },
        None => (head.trim(), None),
    };
    let Some(declared) = decl.category(category) else {
        issue(
            line,
            format!(
                "'{category}' is not a declared waiver category (declared: {})",
                decl.waiver_categories
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        return None;
    };
    let mut ok = true;
    if reason.is_empty() {
        issue(line, "the waiver's reason is empty".to_string());
        ok = false;
    }
    let (mut cite, mut date) = (None, None);
    if let Some(inner) = parenthesis {
        let (c, d) = match inner.split_once(", ") {
            Some((c, d)) => (c.trim(), Some(d.trim())),
            None => (inner.trim(), None),
        };
        if is_record_id(c) {
            cite = Some(c.to_string());
        } else {
            issue(
                line,
                format!("the waiver's cite '{c}' is not a record id such as MILE-215 (a pull request or issue number goes in the reason)"),
            );
            ok = false;
        }
        match d {
            Some(d) if !declared.expires_required() => {
                issue(
                    line,
                    format!("'{category}' takes no date, but the waiver gives '{d}'"),
                );
                ok = false;
            }
            Some(d) if crate::waiver::is_calendar_date(d) => date = Some(d.to_string()),
            Some(d) => {
                issue(
                    line,
                    format!("the waiver's date '{d}' is not a real YYYY-MM-DD date"),
                );
                ok = false;
            }
            None => {}
        }
    }
    if ok && parenthesis.is_none() && declared.cite_required() {
        issue(
            line,
            format!(
                "'{category}' requires a cite{}: write `{category} (<record id>{}) -- <reason>`",
                if declared.expires_required() {
                    " and an expiry date"
                } else {
                    ""
                },
                if declared.expires_required() {
                    ", <YYYY-MM-DD>"
                } else {
                    ""
                }
            ),
        );
        ok = false;
    } else if ok && parenthesis.is_some() && declared.expires_required() && date.is_none() {
        issue(
            line,
            format!("'{category}' requires an expiry date: write `{category} (<record id>, <YYYY-MM-DD>) -- <reason>`"),
        );
        ok = false;
    }
    ok.then(|| Waiver {
        line,
        category: category.to_string(),
        cite,
        date,
        reason: reason.to_string(),
    })
}

#[cfg(test)]
mod tests;
