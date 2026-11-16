//! `urzua fix --conflicts` (`MILE-284`): a record left unmerged by a merge or
//! rebase is resolved only when every difference between the two sides lies in
//! what a re-pin writes -- the pin hashes of the evidence field and of each
//! Done-means item's evidence clause, the version field and rows added to the
//! revision log. Pure: the CLI reads the three index stages, runs the
//! three-way merge of the normalised texts and hashes the merged evidence.

use crate::fix::Attestation;
use crate::line_edit::LineDoc;

const PIN_MARK: &str = "@sha256:URZUA-MERGED-PIN";
const VERSION_MARK: &str = "URZUA-MERGED-VERSION";

/// The three index stages of an unmerged record.
pub struct Stages<'a> {
    pub base: &'a str,
    pub ours: &'a str,
    pub theirs: &'a str,
    /// Theirs is the side being merged into this branch (a merge or a
    /// cherry-pick); `false` in a rebase, where ours is the upstream being
    /// replayed onto and theirs is this branch's own commit.
    pub incoming_is_theirs: bool,
}

/// What the record's type declares that a resolution reads.
pub struct Shape<'a> {
    pub evidence_field: &'a str,
    pub version_field: Option<&'a str>,
    pub log_id: &'a crate::config::RevisionLogId,
    /// Already the evidence shape: its `tool_class` is the class written.
    pub log_shape: &'a crate::config::RevisionLogShape,
    /// The classes `version_bump` names; empty when the type declares none.
    pub bump_classes: &'a [String],
    /// The type's Done-means items, `None` when it declares none.
    pub items: Option<&'a crate::rules::ItemsReader>,
}

/// One pin, as each stage records it at one site of the record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinSides {
    pub spelling: String,
    /// Where the pin is: the evidence field, or `Done means item <n>`.
    pub site: String,
    /// The locator path, for reading the merged file.
    #[serde(skip)]
    pub path: String,
    /// Which site this is across the stages: the field name, or the item's
    /// text with its occurrence among items of the same text.
    #[serde(skip)]
    key: String,
    /// Which occurrence of this locator within the site, from 0: a locator
    /// written twice in one site is two pins.
    #[serde(skip)]
    occurrence: usize,
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
}

impl PinSides {
    /// Both sides moved the pin away from the base to different hashes: only
    /// the merged file can say what it pins now.
    pub fn needs_recompute(&self) -> bool {
        matches!((&self.ours, &self.theirs), (Some(o), Some(t)) if o != t
            && Some(o) != self.base.as_ref() && Some(t) != self.base.as_ref())
    }

    /// The hash a one-sided or agreed change keeps, `None` when it must be
    /// recomputed.
    fn kept(&self) -> Option<String> {
        if self.needs_recompute() {
            return None;
        }
        match (&self.ours, &self.theirs) {
            (Some(o), Some(t)) if Some(o) == self.base.as_ref() => Some(t.clone()),
            (Some(o), _) => Some(o.clone()),
            (None, t) => t.clone(),
        }
    }
}

/// The normalised stages and what was lifted out of them.
#[derive(Debug)]
pub struct Prepared {
    pub base: String,
    pub ours: String,
    pub theirs: String,
    pub pins: Vec<PinSides>,
    pub versions: (Option<String>, Option<String>),
    /// Every row after the log's first, in the order written back: the
    /// incoming side's rows as it has them, then each row only this branch
    /// has after the last row dated on or before it. A merge and a rebase of
    /// the same two branches give the same order.
    pub rows: Vec<String>,
    /// How many of `rows` the base lacks: the rows the two sides added.
    pub rows_added: usize,
    /// The rows the base lacks, in `rows` order.
    pub added_rows: Vec<String>,
    /// The classes of the rows only ours added, and only theirs: a row the
    /// other side lacks is a change against it, which a version bump may need.
    pub one_sided_classes: (Vec<String>, Vec<String>),
    /// Why the Done-means items differ beyond their pin hashes while a pin in
    /// them moved; such a record is refused.
    pub item_mismatch: Option<String>,
}

/// A row with runs of whitespace collapsed, so padding does not make two
/// copies of one row look different.
fn row_key(row: &str) -> String {
    row.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normalises the three stages: each pin hash and the version value become a
/// placeholder and every revision-log row after the first is lifted out, so a
/// three-way merge of what is left conflicts exactly when the record's real
/// content does.
pub fn prepare(stages: &Stages<'_>, shape: &Shape<'_>) -> Result<Prepared, String> {
    if [stages.base, stages.ours, stages.theirs]
        .iter()
        .any(|t| t.starts_with('\u{feff}'))
    {
        return Err("the record starts with a byte-order mark (BOM), which the line edits do not handle; remove it".into());
    }
    let mut base = normalise(stages.base, shape)?;
    let ours = normalise(stages.ours, shape)?;
    let theirs = normalise(stages.theirs, shape)?;
    for side in [&ours, &theirs] {
        if let Some(gone) = base
            .rows
            .iter()
            .find(|r| !side.rows.iter().any(|o| row_key(&o.raw) == row_key(&r.raw)))
        {
            return Err(format!(
                "a side removed or rewrote the revision-log row '{}', which a re-pin never does",
                gone.raw.trim()
            ));
        }
    }
    let item_pins = |n: &Normalised| -> Vec<FoundPin> {
        n.pins
            .iter()
            .filter(|p| p.item.is_some())
            .cloned()
            .collect()
    };
    let moved_in_items =
        item_pins(&base) != item_pins(&ours) || item_pins(&ours) != item_pins(&theirs);
    let item_mismatch = moved_in_items
        .then(|| item_difference(&ours.items, &theirs.items))
        .flatten();
    if base.items != ours.items {
        base.pins.retain(|p| p.item.is_none());
    }
    let mut pins: Vec<PinSides> = Vec::new();
    for (side, found) in [(0, &base.pins), (1, &ours.pins), (2, &theirs.pins)] {
        for f in found {
            let at = match pins.iter().position(|p| {
                p.key == f.key && p.spelling == f.spelling && p.occurrence == f.occurrence
            }) {
                Some(at) => at,
                None => {
                    pins.push(PinSides {
                        spelling: f.spelling.clone(),
                        site: f.site.clone(),
                        path: f.path.clone(),
                        key: f.key.clone(),
                        occurrence: f.occurrence,
                        base: None,
                        ours: None,
                        theirs: None,
                    });
                    pins.len() - 1
                }
            };
            let slot = match side {
                0 => &mut pins[at].base,
                1 => &mut pins[at].ours,
                _ => &mut pins[at].theirs,
            };
            *slot = Some(f.hash.clone());
        }
    }
    let only = |a: &[LogRow], b: &[LogRow]| -> Vec<String> {
        a.iter()
            .filter(|r| !b.iter().any(|o| row_key(&o.raw) == row_key(&r.raw)))
            .map(|r| r.class.clone())
            .collect()
    };
    let one_sided_classes = (
        only(&ours.rows, &theirs.rows),
        only(&theirs.rows, &ours.rows),
    );
    let (incoming, this) = if stages.incoming_is_theirs {
        (&theirs.rows, &ours.rows)
    } else {
        (&ours.rows, &theirs.rows)
    };
    let rows = ordered_rows(incoming, this);
    let added_rows: Vec<String> = rows
        .iter()
        .filter(|r| !base.rows.iter().any(|b| row_key(&b.raw) == row_key(&r.raw)))
        .map(|r| r.raw.clone())
        .collect();
    let rows_added = added_rows.len();
    Ok(Prepared {
        base: base.text,
        ours: ours.text,
        theirs: theirs.text,
        pins,
        versions: (ours.version, theirs.version),
        rows: rows.into_iter().map(|r| r.raw.clone()).collect(),
        rows_added,
        added_rows,
        one_sided_classes,
        item_mismatch,
    })
}

/// The incoming side's rows, then each row only this branch has, placed after
/// the last row dated on or before it, so within a date this branch's rows
/// come last.
fn ordered_rows<'r>(incoming: &'r [LogRow], this: &'r [LogRow]) -> Vec<&'r LogRow> {
    let mut out: Vec<&LogRow> = Vec::new();
    for row in incoming {
        if !out.iter().any(|o| row_key(&o.raw) == row_key(&row.raw)) {
            out.push(row);
        }
    }
    for row in this {
        if out.iter().any(|o| row_key(&o.raw) == row_key(&row.raw)) {
            continue;
        }
        let at = out
            .iter()
            .rposition(|o| o.date <= row.date)
            .map_or(0, |i| i + 1);
        out.insert(at, row);
    }
    out
}

/// The first Done-means item that differs between the two sides once pin
/// hashes are masked, named for the refusal.
fn item_difference(ours: &[ItemLines], theirs: &[ItemLines]) -> Option<String> {
    let n = (0..ours.len().max(theirs.len())).find(|&i| ours.get(i) != theirs.get(i))?;
    let text = |side: &[ItemLines]| {
        side.get(n).map_or("(no such item)".to_string(), |i| {
            i.text.chars().take(80).collect()
        })
    };
    Some(format!(
        "Done means item {} differs between the two sides beyond its pin hashes (ours: '{}'; theirs: '{}'): an item's text, waiver, locators or the items' order changed while a pin in the items moved; resolve it by hand",
        n + 1,
        text(ours),
        text(theirs)
    ))
}

struct LogRow {
    date: String,
    raw: String,
    class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FoundPin {
    site: String,
    key: String,
    /// The item's position, for items.
    item: Option<usize>,
    spelling: String,
    path: String,
    hash: String,
    occurrence: usize,
}

/// An item's masked lines: equal on two sides when only pin hashes differ.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemLines {
    text: String,
    lines: Vec<String>,
}

struct Normalised {
    text: String,
    pins: Vec<FoundPin>,
    version: Option<String>,
    rows: Vec<LogRow>,
    items: Vec<ItemLines>,
}

/// How many lines the front matter takes, its closing `---` included; 0 when
/// there is none.
fn header_lines(doc: &LineDoc) -> usize {
    let end = header_end(doc);
    if end > 1 {
        end
    } else {
        0
    }
}

/// The 1-based lines of the evidence field: its key line and any indented
/// continuation lines of a value folded over several lines.
fn field_lines(doc: &LineDoc, header: usize, field: &str) -> Vec<usize> {
    let key = format!("{field}:");
    let Some(first) = (2..header).find(|n| doc.line(*n).is_some_and(|l| l.starts_with(&key)))
    else {
        return Vec::new();
    };
    let mut lines = vec![first];
    lines.extend(
        (first + 1..header)
            .take_while(|n| doc.line(*n).is_some_and(|l| l.starts_with([' ', '\t']))),
    );
    lines
}

/// Each `(item position, item key, clause line)` of the record's Done-means
/// items that carry an evidence clause, and every item's masked identity.
fn item_clauses(
    content: &str,
    header: usize,
    shape: &Shape<'_>,
) -> Vec<(usize, String, crate::done_items::Item)> {
    let Some(reader) = shape.items else {
        return Vec::new();
    };
    let items = reader.items(content, header).items;
    let mut seen: Vec<String> = Vec::new();
    items
        .into_iter()
        .map(|item| {
            let text = item.text.trim().to_string();
            let occurrence = seen.iter().filter(|t| **t == text).count();
            seen.push(text.clone());
            (item.n, format!("item\u{1f}{text}\u{1f}{occurrence}"), item)
        })
        .collect()
}

fn normalise(content: &str, shape: &Shape<'_>) -> Result<Normalised, String> {
    let mut doc = LineDoc::parse(content);
    let header = header_lines(&doc);
    let mut pins = Vec::new();
    let mut version = None;
    let version_key = shape.version_field.map(|v| format!("{v}:"));
    let fields = field_lines(&doc, header, shape.evidence_field);
    let mut seen = Vec::new();
    for n in &fields {
        let line = doc.line(*n).unwrap_or_default().to_string();
        let value = if *n == fields[0] {
            &line[shape.evidence_field.len() + 1..]
        } else {
            line.as_str()
        };
        let (text, found) = mask_pins(&line, value, &mut seen);
        pins.extend(
            found
                .into_iter()
                .map(|(spelling, path, hash, occurrence)| FoundPin {
                    site: site_name(shape.evidence_field, occurrence),
                    occurrence,
                    key: shape.evidence_field.to_string(),
                    item: None,
                    spelling,
                    path,
                    hash,
                }),
        );
        doc.replace_line(*n, text);
    }
    for n in 2..header.max(1) {
        let line = doc.line(n).unwrap_or_default().to_string();
        if let Some(value) = version_key.as_deref().and_then(|k| line.strip_prefix(k)) {
            let kept = line.len() - value.trim_start().len();
            version = Some(value.trim().to_string());
            doc.replace_line(n, format!("{}{VERSION_MARK}", &line[..kept]));
        }
    }
    let mut items = Vec::new();
    for (position, key, item) in item_clauses(content, header, shape) {
        if let Some(clause) = &item.evidence {
            let line = doc.line(clause.line).unwrap_or_default().to_string();
            if let Some(at) = line.rfind(clause.text.as_str()) {
                let (masked, found) = mask_pins(&line[at..], &line[at..], &mut Vec::new());
                pins.extend(
                    found
                        .into_iter()
                        .map(|(spelling, path, hash, occurrence)| FoundPin {
                            site: site_name(&format!("Done means item {position}"), occurrence),
                            occurrence,
                            key: key.clone(),
                            item: Some(position),
                            spelling,
                            path,
                            hash,
                        }),
                );
                doc.replace_line(clause.line, format!("{}{masked}", &line[..at]));
            }
        }
        let mut lines: Vec<usize> = [
            Some(item.line),
            item.evidence.as_ref().map(|c| c.line),
            item.waiver.as_ref().map(|w| w.line),
        ]
        .into_iter()
        .flatten()
        .collect();
        lines.dedup();
        items.push(ItemLines {
            text: item.text.clone(),
            lines: lines
                .iter()
                .map(|n| doc.line(*n).unwrap_or_default().to_string())
                .collect(),
        });
    }
    let log = crate::rules::find_revision_log_entries(content, shape.log_id, shape.log_shape)
        .ok_or("the record has no revision log")?
        .map_err(|p| p.message)?;
    let mut rows = Vec::new();
    for entry in log.entries.iter().skip(1).rev() {
        rows.push(LogRow {
            date: entry.date.clone(),
            raw: doc.line(entry.line).unwrap_or_default().to_string(),
            class: crate::config::strip_emphasis(&entry.change_class).to_string(),
        });
        doc.remove_line(entry.line);
    }
    rows.reverse();
    Ok(Normalised {
        text: doc.to_string(),
        pins,
        version,
        rows,
        items,
    })
}

/// The 1-based line of the front matter's closing `---`, or 1 when there is
/// no front matter.
fn header_end(doc: &LineDoc) -> usize {
    if doc.line(1).map(str::trim_end) != Some("---") {
        return 1;
    }
    (2..)
        .take_while(|n| doc.line(*n).is_some())
        .find(|n| doc.line(*n).map(str::trim_end) == Some("---"))
        .unwrap_or(1)
}

fn site_name(site: &str, occurrence: usize) -> String {
    if occurrence == 0 {
        site.to_string()
    } else {
        format!("{site}, occurrence {}", occurrence + 1)
    }
}

/// The placeholder of one occurrence of a locator: the same on every side
/// when the locator sits at the same position of the same site.
fn pin_mark(spelling: &str, occurrence: usize) -> String {
    format!("{spelling}{PIN_MARK}-{occurrence}")
}

/// `text` with the first exact occurrence of `old` at or after byte `from`
/// replaced by `new` (a path character before it, or an alphanumeric after
/// it, is not a match), and the byte after the replacement.
fn replace_first(text: &str, old: &str, new: &str, from: usize) -> Option<(String, usize)> {
    let path_char = |c: char| c.is_alphanumeric() || "_./-~@+%".contains(c);
    text[from..]
        .match_indices(old)
        .map(|(at, _)| at + from)
        .find(|&at| {
            text[..at].chars().next_back().is_none_or(|c| !path_char(c))
                && text[at + old.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_ascii_alphanumeric())
        })
        .map(|at| {
            (
                format!("{}{new}{}", &text[..at], &text[at + old.len()..]),
                at + new.len(),
            )
        })
}

/// `line` with each pin hash in `value` masked, one occurrence at a time, and
/// each pin's spelling, path, hash and occurrence within the site; `seen`
/// counts the spellings already met in the site.
fn mask_pins(
    line: &str,
    value: &str,
    seen: &mut Vec<String>,
) -> (String, Vec<(String, String, String, usize)>) {
    let unquoted = value.trim().trim_matches(|c| c == '\'' || c == '"');
    let mut text = line.to_string();
    let mut cursor = line.len() - value.len();
    let mut pins = Vec::new();
    for entry in crate::locator::realized_by_locators(unquoted) {
        let Ok(locator) = entry.parsed else { continue };
        let Some(crate::locator::Anchor::Pin(hash)) = locator.anchor else {
            continue;
        };
        let occurrence = seen.iter().filter(|s| **s == locator.spelling).count();
        let old = format!("{}@sha256:{hash}", locator.spelling);
        let new = pin_mark(&locator.spelling, occurrence);
        if let Some((masked, end)) = replace_first(&text, &old, &new, cursor) {
            text = masked;
            cursor = end;
            seen.push(locator.spelling.clone());
            pins.push((locator.spelling, locator.path, hash, occurrence));
        }
    }
    (text, pins)
}

/// A pin as the resolution leaves it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinOutcome {
    pub locator: String,
    pub site: String,
    pub base: Option<String>,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub merged: String,
    pub recomputed: bool,
}

/// The resolved record and what it holds.
#[derive(Debug)]
pub struct Resolved {
    pub content: String,
    pub pins: Vec<PinOutcome>,
    pub version: Option<String>,
    pub rows_kept: usize,
}

/// The higher of two versions compared as dot-separated numbers; an error
/// when either is not one, since a guess would be a silent edit.
pub fn higher_version(a: &str, b: &str) -> Result<String, String> {
    let parse = |v: &str| -> Option<Vec<u64>> {
        v.trim_matches(|c| c == '\'' || c == '"')
            .split('.')
            .map(|p| p.parse().ok())
            .collect()
    };
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => Ok(if y > x { b } else { a }.to_string()),
        _ => Err(format!(
            "the two sides' versions '{a}' and '{b}' are not dot-separated numbers, so the higher cannot be chosen"
        )),
    }
}

/// Builds the resolved record from the clean merge of the normalised texts:
/// pins restored at their sites (each recomputed one from `recomputed`, by
/// its index in `prepared.pins`), the higher version, the rows in
/// `prepared.rows` order and, when a pin was recomputed or the merge set a
/// version neither side had, one machine-written row saying so.
pub fn rebuild(
    merged: &str,
    prepared: &Prepared,
    recomputed: &[(usize, String)],
    shape: &Shape<'_>,
    today: &str,
    attestation: &Attestation,
) -> Result<Resolved, String> {
    let (mut text, pins) = restore_pins(merged, prepared, recomputed, shape)?;
    let recomputed_any = pins.iter().any(|p| p.recomputed);
    let version = merged_version(prepared, shape, recomputed_any)?;
    let note = version_set_by_merge(prepared, version.as_deref());
    if let Some(v) = &version {
        text = text.replace(VERSION_MARK, v);
    }
    let mut doc = LineDoc::parse(&text);
    let log = crate::rules::find_revision_log_entries(&text, shape.log_id, shape.log_shape)
        .ok_or("the merged record has no revision log")?
        .map_err(|p| p.message)?;
    let mut after = log
        .entries
        .last()
        .map(|e| e.line)
        .ok_or("the merged record's revision log has no first row to keep the others after")?;
    for row in &prepared.rows {
        doc.insert_after(after, row.clone());
        after += 1;
    }
    let moved: Vec<&PinOutcome> = pins.iter().filter(|p| p.recomputed).collect();
    let narrative = match (moved.is_empty(), note) {
        (false, note) => Some(merge_narrative(&moved, attestation, note.as_deref())),
        (true, Some(note)) => Some(format!(
            "{VERSION_ROW_PREFIX}{}): {note} **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a person's row is never rewritten.",
            attestation.label()
        )),
        (true, None) => None,
    };
    if let Some(narrative) = narrative {
        crate::fix::append_log_rows(
            &mut doc,
            shape.log_id,
            shape.log_shape,
            today,
            &[narrative],
            "record a resolution on merge",
        )?;
    }
    Ok(Resolved {
        content: doc.to_string(),
        pins,
        version,
        rows_kept: prepared.rows_added,
    })
}

/// The merged text with each pin placed back at its site, and each pin's
/// outcome.
fn restore_pins(
    merged: &str,
    prepared: &Prepared,
    recomputed: &[(usize, String)],
    shape: &Shape<'_>,
) -> Result<(String, Vec<PinOutcome>), String> {
    let mut doc = LineDoc::parse(merged);
    let header = header_lines(&doc);
    let mut sites: Vec<(usize, String)> = field_lines(&doc, header, shape.evidence_field)
        .into_iter()
        .map(|n| (n, shape.evidence_field.to_string()))
        .collect();
    sites.extend(
        item_clauses(merged, header, shape)
            .into_iter()
            .filter_map(|(_, key, item)| item.evidence.map(|c| (c.line, key))),
    );
    let mut pins = Vec::new();
    for (i, p) in prepared.pins.iter().enumerate() {
        let mark = pin_mark(&p.spelling, p.occurrence);
        let lines: Vec<usize> = sites
            .iter()
            .filter(|(n, key)| *key == p.key && doc.line(*n).is_some_and(|l| l.contains(&mark)))
            .map(|(n, _)| *n)
            .collect();
        if lines.is_empty() {
            continue;
        }
        let merged_hash = match p.kept() {
            Some(h) => h,
            None => recomputed
                .iter()
                .find(|(at, _)| *at == i)
                .map(|(_, h)| h.clone())
                .ok_or_else(|| format!("no merged hash for '{}' ({})", p.spelling, p.site))?,
        };
        let pinned = format!("{}@sha256:{merged_hash}", p.spelling);
        for n in lines {
            let line = doc.line(n).unwrap_or_default().to_string();
            doc.replace_line(
                n,
                replace_first(&line, &mark, &pinned, 0).map_or(line, |(t, _)| t),
            );
        }
        pins.push(PinOutcome {
            locator: p.spelling.clone(),
            site: p.site.clone(),
            base: p.base.clone(),
            ours: p.ours.clone(),
            theirs: p.theirs.clone(),
            merged: merged_hash,
            recomputed: p.needs_recompute(),
        });
    }
    let text = doc.to_string();
    if text.contains(PIN_MARK) {
        return Err(
            "a pin of the merged record could not be placed back at its site; resolve it by hand"
                .into(),
        );
    }
    Ok((text, pins))
}

/// What the merge row says when the merge set a version neither side
/// had (a bump above the higher side's): the number, why, and each version a
/// row added by one side states that the merge superseded. `None` when the
/// version is one side's, which needs no row.
fn version_set_by_merge(prepared: &Prepared, final_version: Option<&str>) -> Option<String> {
    let (Some(o), Some(t)) = &prepared.versions else {
        return None;
    };
    let final_version = final_version?;
    if numbers(o) == numbers(final_version) || numbers(t) == numbers(final_version) {
        return None;
    }
    let bare = |v: &str| v.trim_matches(|c| c == '\'' || c == '"').to_string();
    let higher = higher_version(o, t).ok()?;
    let mut stated: Vec<String> = Vec::new();
    for v in [bare(o), bare(t)] {
        if !stated.contains(&v) && prepared.added_rows.iter().any(|r| states_version(r, &v)) {
            stated.push(v);
        }
    }
    let superseded = if stated.is_empty() {
        String::new()
    } else {
        format!(
            " A row added on a branch states version {}, which was true there and is superseded by this merge.",
            stated.join(" and ")
        )
    };
    Some(format!(
        "`Version` is set to {} by this merge, the next number above the higher side's {}, because `version_bump` requires a new version for the rows this merge combines.{superseded}",
        bare(final_version),
        bare(&higher)
    ))
}

fn states_version(row: &str, version: &str) -> bool {
    let lower = row.to_ascii_lowercase();
    if !(lower.contains("version") || lower.contains("bump")) {
        return false;
    }
    let edge = |c: Option<char>| c.is_none_or(|c| !(c.is_ascii_digit() || c == '.'));
    row.match_indices(version).any(|(at, _)| {
        let before = row[..at].chars().next_back();
        let after = row[at + version.len()..].chars().next();
        edge(before)
            && (edge(after)
                || (after == Some('.') && edge(row[at + version.len() + 1..].chars().next())))
    })
}

/// The higher version, bumped once more when the rows this merge adds against
/// a side are of a class `version_bump` names and the version would not
/// otherwise differ from that side's (`revision-log.version-bump-required`).
fn merged_version(
    prepared: &Prepared,
    shape: &Shape<'_>,
    merge_row: bool,
) -> Result<Option<String>, String> {
    let bumps = |classes: &[String]| classes.iter().any(|c| shape.bump_classes.contains(c));
    let written = merge_row && bumps(std::slice::from_ref(&shape.log_shape.tool_class));
    let (ours_only, theirs_only) = &prepared.one_sided_classes;
    let (ours, theirs) = &prepared.versions;
    let (Some(o), Some(t)) = (ours, theirs) else {
        return Ok(ours.clone().or(theirs.clone()));
    };
    let higher = higher_version(o, t)?;
    let same = |a: &str, b: &str| numbers(a) == numbers(b);
    let must_leave_ours = written || bumps(theirs_only);
    let must_leave_theirs = written || bumps(ours_only);
    if (must_leave_ours && same(&higher, o)) || (must_leave_theirs && same(&higher, t)) {
        return Ok(Some(bump_version(&higher)));
    }
    Ok(Some(higher))
}

fn numbers(v: &str) -> Option<Vec<u64>> {
    v.trim_matches(|c| c == '\'' || c == '"')
        .split('.')
        .map(|p| p.parse().ok())
        .collect()
}

/// `v` with its last number incremented, quoting kept.
pub fn bump_version(v: &str) -> String {
    let at = v.rfind(|c: char| c.is_ascii_digit()).map_or(0, |i| i + 1);
    let start = v[..at]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let n: u64 = v[start..at].parse().unwrap_or(0);
    format!("{}{}{}", &v[..start], n + 1, &v[at..])
}

/// What starts the row `fix --conflicts` writes; `merge_row_locators` parses it.
pub const MERGE_ROW_PREFIX: &str = "Pin recomputed on merge (attested by ";

/// What starts the row `fix --conflicts` writes when it recomputed no pin but
/// set a version neither side had.
pub const VERSION_ROW_PREFIX: &str = "Version set on merge (attested by ";

/// Whether `row` is one `fix --conflicts` writes.
pub fn is_machine_row(row: &str) -> bool {
    merge_row_locators(row).is_some() || row.contains(VERSION_ROW_PREFIX)
}

/// The locators a merge row names as recomputed, or `None` when `row` is not one.
pub fn merge_row_locators(row: &str) -> Option<Vec<String>> {
    let at = row.find(MERGE_ROW_PREFIX)?;
    let rest = &row[at..];
    let list = rest
        .split_once("re-pinned, ")?
        .1
        .split_once("; the new hash")?
        .0;
    let mut out: Vec<String> = list
        .split(", `")
        .filter_map(|item| item.trim_start_matches('`').split_once('`'))
        .map(|(loc, _)| loc.to_string())
        .collect();
    out.dedup();
    Some(out)
}

fn merge_narrative(moved: &[&PinOutcome], attestation: &Attestation, note: Option<&str>) -> String {
    let short = |h: &Option<String>| {
        h.as_deref()
            .map(|h| h.get(..12).unwrap_or(h).to_string())
            .unwrap_or_default()
    };
    let list = moved
        .iter()
        .map(|p| {
            let mut sides = [short(&p.ours), short(&p.theirs)];
            sides.sort();
            format!(
                "`{}` ({}) {} and {} -> {}",
                p.locator,
                p.site,
                sides[0],
                sides[1],
                short(&Some(p.merged.clone()))
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let note = note.map(|n| format!(" {n}")).unwrap_or_default();
    format!(
        "{MERGE_ROW_PREFIX}{}): both merged sides re-pinned, {list}; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state.{note} **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin.",
        attestation.label()
    )
}

/// The first line of a conflicted three-way merge's output that lies inside a
/// conflict, for the refusal: the placeholders say where it is not.
pub fn first_conflict_line(merged: &str) -> Option<String> {
    let mut lines = merged.lines();
    lines.find(|l| l.starts_with("<<<<<<<"))?;
    lines
        .find(|l| !l.starts_with("=======") && !l.starts_with("|||||||"))
        .map(|l| l.trim().chars().take(80).collect())
}

/// Whether `content` still carries a conflict marker line, so a record git
/// lists as unmerged but already resolved by hand is left for staging.
pub fn has_conflict_markers(content: &str) -> bool {
    content.lines().any(|l| l.starts_with("<<<<<<< "))
        && content.lines().any(|l| l.starts_with(">>>>>>> "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(base: &str, ours: &str, theirs: &str) -> PinSides {
        PinSides {
            spelling: "src/a.rs".into(),
            site: "Realized-by".into(),
            path: "src/a.rs".into(),
            key: "Realized-by".into(),
            occurrence: 0,
            base: Some(base.into()),
            ours: Some(ours.into()),
            theirs: Some(theirs.into()),
        }
    }

    #[test]
    fn only_a_pin_both_sides_moved_apart_is_recomputed() {
        assert!(pin("a", "b", "c").needs_recompute());
        assert!(!pin("a", "a", "c").needs_recompute());
        assert_eq!(pin("a", "a", "c").kept().as_deref(), Some("c"));
        assert_eq!(pin("a", "b", "a").kept().as_deref(), Some("b"));
        assert!(!pin("a", "b", "b").needs_recompute());
    }

    #[test]
    fn a_version_bump_increments_the_last_number_and_keeps_quotes() {
        assert_eq!(bump_version("'0.52'"), "'0.53'");
        assert_eq!(bump_version("2"), "3");
        assert_eq!(bump_version("1.9"), "1.10");
    }

    #[test]
    fn a_merge_row_names_its_locators_and_other_rows_do_not() {
        let row = format!("> | 2026-11-16 | {MERGE_ROW_PREFIX}t, identity: asserted, actor: agent): both merged sides re-pinned, `src/a.rs` aaa and bbb -> ccc, `src/b.rs#f` d and e -> f; the new hash is the merged file's. | **evidence** |");
        assert_eq!(
            merge_row_locators(&row).unwrap(),
            vec!["src/a.rs", "src/b.rs#f"]
        );
        assert!(merge_row_locators("> | 2026-11-16 | Re-baselined. | **x** |").is_none());
    }

    #[test]
    fn a_row_states_a_version_only_beside_the_word_version_or_bump() {
        assert!(states_version("> | d | Bumped to 2. | x |", "2"));
        assert!(states_version("> | d | `Version` 1.2 -> 1.3. | x |", "1.3"));
        assert!(!states_version("> | d | Bumped to 12. | x |", "2"));
        assert!(!states_version("> | d | Fixed 2 typos. | x |", "2"));
    }

    fn row(date: &str, raw: &str) -> LogRow {
        LogRow {
            date: date.into(),
            raw: raw.into(),
            class: String::new(),
        }
    }

    #[test]
    fn within_a_date_the_incoming_rows_come_first_and_this_branch_last() {
        let incoming = [
            row("2026-01-01", "b0"),
            row("2026-03-01", "m1"),
            row("2026-03-01", "m2"),
        ];
        let this = [
            row("2026-01-01", "b0"),
            row("2026-03-01", "y1"),
            row("2026-02-01", "y0"),
        ];
        let order: Vec<&str> = ordered_rows(&incoming, &this)
            .iter()
            .map(|r| r.raw.as_str())
            .collect();
        assert_eq!(order, ["b0", "y0", "m1", "m2", "y1"]);
    }

    #[test]
    fn rows_differing_only_in_padding_are_one_row() {
        assert_eq!(row_key("> |  a  | b |"), row_key("> | a | b |"));
    }

    #[test]
    fn the_higher_version_is_numeric_and_a_non_number_is_refused() {
        assert_eq!(higher_version("1.9", "1.10").unwrap(), "1.10");
        assert_eq!(higher_version("'3'", "'2'").unwrap(), "'3'");
        assert!(higher_version("draft", "2").is_err());
    }
}
