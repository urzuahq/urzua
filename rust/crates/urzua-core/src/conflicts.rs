//! `urzua fix --conflicts` (`MILE-284`): a record left unmerged by a merge or
//! rebase is resolved only when every difference between the two sides lies in
//! what a re-pin writes -- the evidence field's pin hashes, the version field
//! and rows appended to the revision log. Pure: the CLI reads the three index
//! stages, runs the three-way merge of the normalised texts and hashes the
//! merged evidence.

use crate::fix::Attestation;
use crate::line_edit::LineDoc;

const PIN_MARK: &str = "@sha256:URZUA-MERGED-PIN";
const VERSION_MARK: &str = "URZUA-MERGED-VERSION";

/// The three index stages of an unmerged record.
pub struct Stages<'a> {
    pub base: &'a str,
    pub ours: &'a str,
    pub theirs: &'a str,
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
}

/// One pin of the evidence field, as each stage records it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinSides {
    pub spelling: String,
    /// The locator path, for reading the merged file.
    #[serde(skip)]
    pub path: String,
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
    /// Rows either side appended after the base's last row, de-duplicated,
    /// in date order and then by text so the order is the same whichever
    /// side git calls ours (a rebase swaps them).
    pub appended: Vec<String>,
    /// The classes of the rows only ours appended, and only theirs: a row the
    /// other side lacks is a change against it, which a version bump may need.
    pub one_sided_classes: (Vec<String>, Vec<String>),
}

/// A row with runs of whitespace collapsed, so padding does not make two
/// copies of one row look different.
fn row_key(row: &str) -> String {
    row.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normalises the three stages: each evidence pin hash and the version value
/// become a placeholder and appended log rows are lifted out, so a three-way
/// merge of what is left conflicts exactly when the record's real content does.
pub fn prepare(stages: &Stages<'_>, shape: &Shape<'_>) -> Result<Prepared, String> {
    if [stages.base, stages.ours, stages.theirs]
        .iter()
        .any(|t| t.starts_with('\u{feff}'))
    {
        return Err("the record starts with a byte-order mark (BOM), which the line edits do not handle; remove it".into());
    }
    let base = normalise(stages.base, shape, 0)?;
    let base_rows = base.rows_total;
    let ours = normalise(stages.ours, shape, base_rows)?;
    let theirs = normalise(stages.theirs, shape, base_rows)?;
    let mut pins: Vec<PinSides> = Vec::new();
    for (side, found) in [(0, &base.pins), (1, &ours.pins), (2, &theirs.pins)] {
        for (spelling, path, hash) in found {
            let at = match pins.iter().position(|p| &p.spelling == spelling) {
                Some(at) => at,
                None => {
                    pins.push(PinSides {
                        spelling: spelling.clone(),
                        path: path.clone(),
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
            *slot = Some(hash.clone());
        }
    }
    let only = |a: &[LogRow], b: &[LogRow]| -> Vec<String> {
        a.iter()
            .filter(|r| !b.iter().any(|o| row_key(&o.raw) == row_key(&r.raw)))
            .map(|r| r.class.clone())
            .collect()
    };
    let one_sided_classes = (
        only(&ours.appended, &theirs.appended),
        only(&theirs.appended, &ours.appended),
    );
    let mut appended: Vec<(String, String, String)> = Vec::new();
    for row in ours.appended.into_iter().chain(theirs.appended) {
        let key = row_key(&row.raw);
        if !appended.iter().any(|(_, k, _)| *k == key) {
            appended.push((row.date, key, row.raw));
        }
    }
    appended.sort();
    Ok(Prepared {
        base: base.text,
        ours: ours.text,
        theirs: theirs.text,
        pins,
        versions: (ours.version, theirs.version),
        appended: appended.into_iter().map(|(_, _, r)| r).collect(),
        one_sided_classes,
    })
}

struct LogRow {
    date: String,
    raw: String,
    class: String,
}

struct Normalised {
    text: String,
    pins: Vec<(String, String, String)>,
    version: Option<String>,
    appended: Vec<LogRow>,
    rows_total: usize,
}

fn normalise(content: &str, shape: &Shape<'_>, base_rows: usize) -> Result<Normalised, String> {
    let mut doc = LineDoc::parse(content);
    let header_end = header_end(&doc);
    let mut pins = Vec::new();
    let mut version = None;
    let field_key = format!("{}:", shape.evidence_field);
    let version_key = shape.version_field.map(|v| format!("{v}:"));
    for n in 2..header_end {
        let line = doc.line(n).unwrap_or_default().to_string();
        if let Some(value) = line.strip_prefix(&field_key) {
            let (text, found) = mask_pins(&line, value);
            pins = found;
            doc.replace_line(n, text);
        } else if let Some(value) = version_key.as_deref().and_then(|k| line.strip_prefix(k)) {
            let kept = line.len() - value.trim_start().len();
            version = Some(value.trim().to_string());
            doc.replace_line(n, format!("{}{VERSION_MARK}", &line[..kept]));
        }
    }
    let log = crate::rules::find_revision_log_entries(content, shape.log_id, shape.log_shape)
        .ok_or("the record has no revision log")?
        .map_err(|p| p.message)?;
    let rows_total = log.entries.len();
    let mut appended = Vec::new();
    for entry in log.entries.iter().skip(base_rows).rev() {
        let raw = doc.line(entry.line).unwrap_or_default().to_string();
        appended.push(LogRow {
            date: entry.date.clone(),
            raw,
            class: crate::config::strip_emphasis(&entry.change_class).to_string(),
        });
        doc.remove_line(entry.line);
    }
    Ok(Normalised {
        text: doc.to_string(),
        pins,
        version,
        appended,
        rows_total,
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

/// `line` with each pin hash on it masked, and each pin's spelling, path and
/// hash.
fn mask_pins(line: &str, value: &str) -> (String, Vec<(String, String, String)>) {
    let unquoted = value.trim().trim_matches(|c| c == '\'' || c == '"');
    let mut text = line.to_string();
    let mut pins = Vec::new();
    for entry in crate::locator::realized_by_locators(unquoted) {
        let Ok(locator) = entry.parsed else { continue };
        let Some(crate::locator::Anchor::Pin(hash)) = locator.anchor else {
            continue;
        };
        let old = format!("{}@sha256:{hash}", locator.spelling);
        let new = format!("{}{PIN_MARK}", locator.spelling);
        if let Some(masked) = crate::fix::replace_exact_pin(&text, &old, &new) {
            text = masked;
            pins.push((locator.spelling, locator.path, hash));
        }
    }
    (text, pins)
}

/// A pin as the resolution leaves it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinOutcome {
    pub locator: String,
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
/// pins restored (each recomputed one from `recomputed`, by spelling), the
/// higher version, both sides' appended rows and, when a pin was recomputed,
/// one machine-written row saying so.
pub fn rebuild(
    merged: &str,
    prepared: &Prepared,
    recomputed: &[(String, String)],
    shape: &Shape<'_>,
    today: &str,
    attestation: &Attestation,
) -> Result<Resolved, String> {
    let mut text = merged.to_string();
    let mut pins = Vec::new();
    for p in &prepared.pins {
        let mark = format!("{}{PIN_MARK}", p.spelling);
        if !text.contains(&mark) {
            continue;
        }
        let merged_hash = match p.kept() {
            Some(h) => h,
            None => recomputed
                .iter()
                .find(|(s, _)| *s == p.spelling)
                .map(|(_, h)| h.clone())
                .ok_or_else(|| format!("no merged hash for '{}'", p.spelling))?,
        };
        text = text.replace(&mark, &format!("{}@sha256:{merged_hash}", p.spelling));
        pins.push(PinOutcome {
            locator: p.spelling.clone(),
            base: p.base.clone(),
            ours: p.ours.clone(),
            theirs: p.theirs.clone(),
            merged: merged_hash,
            recomputed: p.needs_recompute(),
        });
    }
    let merge_row = pins.iter().any(|p| p.recomputed);
    let version = merged_version(prepared, shape, merge_row)?;
    if let Some(v) = &version {
        text = text.replace(VERSION_MARK, v);
    }
    let mut doc = LineDoc::parse(&text);
    let log = crate::rules::find_revision_log_entries(&text, shape.log_id, shape.log_shape)
        .ok_or("the merged record has no revision log")?
        .map_err(|p| p.message)?;
    let mut after = log.entries.last().map(|e| e.line).unwrap_or_default();
    for row in &prepared.appended {
        doc.insert_after(after, row.clone());
        after += 1;
    }
    let moved: Vec<&PinOutcome> = pins.iter().filter(|p| p.recomputed).collect();
    if !moved.is_empty() {
        let narrative = merge_narrative(&moved, attestation);
        crate::fix::append_log_rows(
            &mut doc,
            shape.log_id,
            shape.log_shape,
            today,
            &[narrative],
            "record a pin recomputed on merge",
        )?;
    }
    Ok(Resolved {
        content: doc.to_string(),
        pins,
        version,
        rows_kept: prepared.appended.len(),
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

/// The locators a merge row names as recomputed, or `None` when `row` is not one.
pub fn merge_row_locators(row: &str) -> Option<Vec<String>> {
    Some(
        merge_row_pins(row)?
            .into_iter()
            .map(|p| p.locator)
            .collect(),
    )
}

/// The re-pins a merge row names, each `` `locator` a and b -> merged ``, or
/// `None` when `row` is not one.
pub fn merge_row_pins(row: &str) -> Option<Vec<crate::fix::RowPin>> {
    let at = row.find(MERGE_ROW_PREFIX)?;
    let rest = &row[at..];
    let list = rest
        .split_once("re-pinned, ")?
        .1
        .split_once("; the new hash")?
        .0;
    Some(
        list.split(", `")
            .filter_map(|item| item.trim_start_matches('`').split_once('`'))
            .map(|(loc, tail)| {
                let (sides, to) = tail.split_once(" -> ").unwrap_or((tail, ""));
                crate::fix::RowPin {
                    locator: loc.to_string(),
                    from: sides
                        .split(" and ")
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    to: to.trim().to_string(),
                }
            })
            .collect(),
    )
}

fn merge_narrative(moved: &[&PinOutcome], attestation: &Attestation) -> String {
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
                "`{}` {} and {} -> {}",
                p.locator,
                sides[0],
                sides[1],
                short(&Some(p.merged.clone()))
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{MERGE_ROW_PREFIX}{}): both merged sides re-pinned, {list}; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin.",
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
            path: "src/a.rs".into(),
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
