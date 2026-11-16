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
    pub log_shape: &'a crate::config::RevisionLogShape,
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
}

/// Normalises the three stages: each evidence pin hash and the version value
/// become a placeholder and appended log rows are lifted out, so a three-way
/// merge of what is left conflicts exactly when the record's real content does.
pub fn prepare(stages: &Stages<'_>, shape: &Shape<'_>) -> Result<Prepared, String> {
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
    let mut appended: Vec<(String, String)> = Vec::new();
    for row in ours.appended.into_iter().chain(theirs.appended) {
        if !appended.iter().any(|(_, r)| *r == row.1) {
            appended.push(row);
        }
    }
    appended.sort();
    Ok(Prepared {
        base: base.text,
        ours: ours.text,
        theirs: theirs.text,
        pins,
        versions: (ours.version, theirs.version),
        appended: appended.into_iter().map(|(_, r)| r).collect(),
    })
}

struct Normalised {
    text: String,
    pins: Vec<(String, String, String)>,
    version: Option<String>,
    appended: Vec<(String, String)>,
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
        appended.push((entry.date.clone(), raw));
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
    let version = match &prepared.versions {
        (Some(a), Some(b)) => Some(higher_version(a, b)?),
        (a, b) => a.clone().or(b.clone()),
    };
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

fn merge_narrative(moved: &[&PinOutcome], attestation: &Attestation) -> String {
    let short = |h: &Option<String>| {
        h.as_deref()
            .map(|h| h.get(..12).unwrap_or(h).to_string())
            .unwrap_or_default()
    };
    let list = moved
        .iter()
        .map(|p| {
            format!(
                "`{}` {} and {} -> {}",
                p.locator,
                short(&p.ours),
                short(&p.theirs),
                short(&Some(p.merged.clone()))
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Pin recomputed on merge (attested by {}): both merged sides re-pinned, {list}; the new hash is the merged file's. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; the merged pin is covered by the pull request's review.",
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
    fn the_higher_version_is_numeric_and_a_non_number_is_refused() {
        assert_eq!(higher_version("1.9", "1.10").unwrap(), "1.10");
        assert_eq!(higher_version("'3'", "'2'").unwrap(), "'3'");
        assert!(higher_version("draft", "2").is_err());
    }
}
