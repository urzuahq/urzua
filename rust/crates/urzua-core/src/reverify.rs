//! `urzua reverify` (`ADR-97` section 4), the pure half: what a review lists
//! per locator, how a first pin is written in a locator's own spelling, and the
//! two attributed revision rows (`Pinned` and `Re-verified`). The command
//! (`urzua-cli`) reads the tree, the clock and git; nothing here does.

use crate::config::{Config, RelationRole};
use crate::fix::{append_log_rows, evidence_edit_lines, Attestation, PinRead, PinSpan};
use crate::line_edit::LineDoc;
use crate::locator::{self, Anchor, Locator};
use crate::record::Record;
use crate::rules::{evidence_text, relation_field_name};

/// What the tool could prove about one locator at review time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocatorCheck {
    /// Pinned, and the content still equals the pin.
    HashChecked,
    /// Unpinned in a category that must be pinned: `--apply` pins it first.
    WillPin,
    /// Unpinned in a category that need not be pinned (`spec:`): re-resolved
    /// and checked by git history only.
    HistoryOnly,
    /// Pinned, and the content no longer equals the pin: `fix --rebaseline`.
    Changed,
    /// The file is missing, the scope finds nothing, or the locator does not parse.
    Unresolved,
}

/// One locator of a record's evidence, as the review reports it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LocatorReview {
    pub locator: String,
    pub category: String,
    pub resolved: bool,
    pub pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_hash: Option<String>,
    /// The resolved span of a scoped locator, so the reviewer reads what is pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<PinSpan>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    pub check: LocatorCheck,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The locator as the record wrote it, before any pin: what a first pin edits.
    #[serde(skip)]
    pub spelling: String,
    #[serde(skip)]
    pub scoped: bool,
}

/// The locators of one record and where they sit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatorsReview {
    pub field: String,
    /// The 1-based line of the evidence field; 0 when the record has none.
    pub field_line: usize,
    /// The lines of the Done-means item clauses (`ADR-96`).
    pub item_lines: Vec<usize>,
    pub locators: Vec<LocatorReview>,
}

/// Reads every locator of `record`'s evidence (the field and the Done-means
/// item clauses), once each. `read` is the caller's lookup of what a locator
/// hashes to now.
pub fn review_locators(
    record: &Record,
    config: &Config,
    require_pin: &[String],
    read: &dyn Fn(&Locator) -> PinRead,
) -> LocatorsReview {
    let field = relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
    let mut out = LocatorsReview {
        field: field.to_string(),
        field_line: record
            .header
            .fields
            .iter()
            .find(|f| f.key.as_str() == field)
            .map_or(0, |f| f.line),
        item_lines: record
            .items
            .iter()
            .flat_map(|i| i.items.iter())
            .filter_map(|i| i.evidence.as_ref())
            .filter(|c| c.valid)
            .map(|c| c.line)
            .collect(),
        locators: Vec::new(),
    };
    let Some(value) = evidence_text(record, field) else {
        return out;
    };
    let mut seen: Vec<(String, String, Option<Anchor>)> = Vec::new();
    for entry in locator::realized_by_locators(&value) {
        let locator = match entry.parsed {
            Ok(l) => l,
            Err(e) => {
                out.locators
                    .push(unparsed_review(entry.raw, entry.category, &e));
                continue;
            }
        };
        let key = (
            entry.category.clone(),
            locator.spelling.clone(),
            locator.anchor.clone(),
        );
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let required = require_pin.contains(&entry.category);
        out.locators
            .push(review_locator(&locator, entry.category, required, read));
    }
    out
}

/// The review of a locator that does not parse.
fn unparsed_review(raw: String, category: String, error: &dyn std::fmt::Display) -> LocatorReview {
    LocatorReview {
        locator: raw.clone(),
        category,
        resolved: false,
        pinned: false,
        recorded_hash: None,
        current_hash: None,
        span: None,
        files: Vec::new(),
        check: LocatorCheck::Unresolved,
        reason: Some(format!("does not parse: {error}")),
        spelling: raw,
        scoped: false,
    }
}

/// The review of one parsed locator: what it resolves to now against its pin.
fn review_locator(
    locator: &Locator,
    category: String,
    required: bool,
    read: &dyn Fn(&Locator) -> PinRead,
) -> LocatorReview {
    let recorded = match &locator.anchor {
        Some(Anchor::Pin(h)) => Some(h.clone()),
        _ => None,
    };
    let mut review = LocatorReview {
        locator: locator.display(),
        category,
        resolved: false,
        pinned: recorded.is_some(),
        recorded_hash: recorded.clone(),
        current_hash: None,
        span: None,
        files: Vec::new(),
        check: LocatorCheck::Unresolved,
        reason: None,
        spelling: locator.spelling.clone(),
        scoped: locator.scope.is_some(),
    };
    if matches!(locator.anchor, Some(Anchor::Commit(_))) {
        review.reason = Some("a commit anchor is not supported".to_string());
        return review;
    }
    match read(locator) {
        PinRead::Current { hash, span, files } => {
            review.resolved = true;
            review.span = span;
            review.files = files;
            review.check = match &recorded {
                Some(r) if *r == hash => LocatorCheck::HashChecked,
                Some(_) => LocatorCheck::Changed,
                None if required => LocatorCheck::WillPin,
                None => LocatorCheck::HistoryOnly,
            };
            // A history-only locator is not hashed into the row.
            if review.check != LocatorCheck::HistoryOnly {
                review.current_hash = Some(hash);
            }
        }
        PinRead::Failed(reason) => review.reason = Some(reason),
        PinRead::Unreadable => {
            review.reason = Some(format!("file not found or unreadable: {}", locator.path));
        }
    }
    review
}

/// A locator that gets its first pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstPin {
    pub category: String,
    pub spelling: String,
    pub display: String,
    pub hash: String,
    pub scoped: bool,
}

impl FirstPin {
    /// The pins a review's `will-pin` locators make.
    pub fn from_review(locators: &[LocatorReview]) -> Vec<FirstPin> {
        locators
            .iter()
            .filter(|l| l.check == LocatorCheck::WillPin)
            .filter_map(|l| {
                Some(FirstPin {
                    category: l.category.clone(),
                    spelling: l.spelling.clone(),
                    display: l.locator.clone(),
                    hash: l.current_hash.clone()?,
                    scoped: l.scoped,
                })
            })
            .collect()
    }
}

fn path_char(c: char) -> bool {
    c.is_alphanumeric() || "_./-~@+%".contains(c)
}

/// `line` with the unpinned locator `category:spelling` given its pin, or
/// `None` when it is not on the line. A match starts at a token boundary and
/// ends where the locator does, so the pin on `a.rs` is not written onto
/// `lib/a.rs`, `a.rs#Foo` or an already pinned `a.rs@sha256:...`, and the pin on
/// `a.rs#Foo` is not written into `a.rs#Foo::bar` or `a.rs#Foo(x)`.
pub fn pin_unpinned_in_line(
    line: &str,
    category: &str,
    spelling: &str,
    hash: &str,
) -> Option<String> {
    let label = format!("{category}:");
    let mut out = String::with_capacity(line.len() + 80);
    let mut rest = 0;
    let mut found = false;
    for (at, _) in line.match_indices(spelling) {
        if at < rest {
            continue;
        }
        let before = line[..at].trim_end_matches([' ', '\t']);
        let Some(head) = before.strip_suffix(&label) else {
            continue;
        };
        if head
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let after_ok = line[at + spelling.len()..]
            .chars()
            .next()
            .is_none_or(|c| !path_char(c) && !matches!(c, '#' | ':' | '('));
        if after_ok {
            out.push_str(&line[rest..at + spelling.len()]);
            out.push_str("@sha256:");
            out.push_str(hash);
            rest = at + spelling.len();
            found = true;
        }
    }
    out.push_str(&line[rest..]);
    found.then_some(out)
}

/// One record in a review: its standing, its locators and what refuses it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordReview {
    pub record: std::path::PathBuf,
    pub status: String,
    pub tier: String,
    /// The expiry finding this review answers; absent for a record that has not expired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_days: Option<i64>,
    pub locators: Vec<LocatorReview>,
    /// For `--pin-only`: the commits that touched an unpinned locator since the
    /// `Realized-by` line was last touched, so the reviewer reads the delta.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commits_since: Vec<String>,
    /// Why the record cannot be written; empty when it can.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<String>,
}

/// `urzua reverify`'s report (`ADR-97` section 4).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReverifyReport {
    pub status: crate::report::FixStatus,
    pub as_of: crate::report::AsOf,
    /// `detect` writes nothing; `apply` wrote the records listed as applied.
    pub mode: &'static str,
    pub pin_only: bool,
    pub population: crate::report::Population,
    /// The one reason every row of this batch carries, printed once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub records: Vec<RecordReview>,
    pub failed: Vec<crate::report::FixFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<crate::report::AttestationReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<crate::report::Notice>,
}

impl crate::report::Report for ReverifyReport {
    fn notices(&self) -> &[crate::report::Notice] {
        &self.notices
    }
    fn exit_code(&self) -> std::process::ExitCode {
        use crate::report::FixStatus;
        match self.status {
            FixStatus::NotRun => std::process::ExitCode::from(2),
            FixStatus::PartialFailure => std::process::ExitCode::from(1),
            FixStatus::Ok | FixStatus::RepairsAvailable | FixStatus::Applied => {
                std::process::ExitCode::from(0)
            }
        }
    }
}

/// What a renewal records.
#[derive(Debug, Clone)]
pub struct Renewal<'a> {
    pub anchor_field: &'a str,
    pub age_days: i64,
    pub days: u32,
    /// The repository declares no `require_pin`: unpinned locators are `H`,
    /// not `S spec locators`, in the row.
    pub require_pin_empty: bool,
}

/// Everything one record's write needs.
#[derive(Debug, Clone)]
pub struct ReverifyPlan<'a> {
    pub review: &'a LocatorsReview,
    pub pins: &'a [FirstPin],
    pub renewal: Option<Renewal<'a>>,
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// The `Pinned` row (`ADR-97` section 4).
pub fn pinned_row(attestation: &Attestation, pins: &[FirstPin], reason: &str) -> String {
    let file_level = pins.iter().filter(|p| !p.scoped).count();
    let evidence = pins
        .iter()
        .map(|p| format!("{}@sha256:{}", p.display, short(&p.hash)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Pinned (attested by {}): {} locators pinned, {} file-level, {} scoped. Evidence: {evidence} Reason: {reason} **Why:** a claim is renewed only over pinned code and test evidence.",
        attestation.label(),
        pins.len(),
        file_level,
        pins.len() - file_level,
    )
}

/// The `Re-verified` row (`ADR-97` section 4): every locator re-resolved, how
/// many the tool could hash, how many only git history vouches for.
pub fn reverified_row(
    attestation: &Attestation,
    review: &LocatorsReview,
    renewal: &Renewal<'_>,
    reason: &str,
) -> String {
    let total = review.locators.len();
    let hashed = review
        .locators
        .iter()
        .filter(|l| matches!(l.check, LocatorCheck::HashChecked | LocatorCheck::WillPin))
        .count();
    let history = total - hashed;
    let evidence = review
        .locators
        .iter()
        .map(
            |l| match l.current_hash.as_deref().or(l.recorded_hash.as_deref()) {
                Some(h) if l.check != LocatorCheck::HistoryOnly => {
                    format!("{}@sha256:{}", l.locator, short(h))
                }
                _ => l.locator.clone(),
            },
        )
        .collect::<Vec<_>>()
        .join(", ");
    let history_text = if renewal.require_pin_empty {
        format!("{history} unpinned locators checked by git history only")
    } else {
        format!("{history} spec locators checked by git history only")
    };
    format!(
        "Re-verified (attested by {}): {total} locators re-resolved, {hashed} hash-checked, {history_text}. Evidence: {evidence} Reason: {reason} **Why:** the claim was {} days old against a {}-day period.",
        attestation.label(),
        renewal.age_days,
        renewal.days,
    )
}

/// New file content for one record: the first pins on the evidence field and
/// the Done-means item clauses, the anchor field set to `today`, and the
/// attributed rows, in that order. Pure. Fails, writing nothing, when a pin
/// cannot be placed (the record changed since the review) or the record has no
/// revision log to record the attestation in.
pub fn apply_reverify(
    content: &str,
    plan: &ReverifyPlan<'_>,
    today: &str,
    attestation: &Attestation,
    reason: &str,
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Result<String, String> {
    let mut doc = LineDoc::parse(content);
    let review = plan.review;
    let lines = evidence_edit_lines(
        &doc,
        &review.field,
        review.field_line,
        &review.item_lines,
        "the review",
    )?;
    for pin in plan.pins {
        let mut placed = false;
        for &at in &lines {
            let line = doc.line(at).unwrap_or_default().to_string();
            if let Some(edited) =
                pin_unpinned_in_line(&line, &pin.category, &pin.spelling, &pin.hash)
            {
                doc.replace_line(at, edited);
                placed = true;
            }
        }
        if !placed {
            return Err(format!(
                "the unpinned locator '{}' is no longer on the '{}' line or in a Done means item clause -- the record may have changed since the review",
                pin.display, review.field
            ));
        }
    }
    if let Some(renewal) = &plan.renewal {
        set_anchor(&mut doc, renewal.anchor_field, today)?;
    }

    let mut rows = Vec::new();
    if !plan.pins.is_empty() {
        rows.push(pinned_row(attestation, plan.pins, reason));
    }
    if let Some(renewal) = &plan.renewal {
        rows.push(reverified_row(attestation, review, renewal, reason));
    }
    append_log_rows(&mut doc, log_id, shape, today, &rows, "write")?;
    Ok(doc.to_string())
}

/// Sets `field` to `today` in the header, adding the line just before the
/// closing delimiter when the record has none.
fn set_anchor(doc: &mut LineDoc, field: &str, today: &str) -> Result<(), String> {
    let first = doc.line(1).map(str::trim_end);
    if first != Some("---") {
        return Err("the record has no YAML frontmatter to hold the anchor field".to_string());
    }
    let closing = (2..)
        .map_while(|n| doc.line(n).map(|l| (n, l)))
        .find(|(_, l)| l.trim_end() == "---")
        .map(|(n, _)| n)
        .ok_or_else(|| "the record's frontmatter is not closed".to_string())?;
    let key = format!("{field}:");
    let existing = (2..closing).find(|n| doc.line(*n).is_some_and(|l| l.starts_with(&key)));
    match existing {
        Some(n) => doc.replace_line(n, format!("{field}: {today}")),
        None => doc.insert_after(closing - 1, format!("{field}: {today}")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn a_first_pin_is_written_on_the_exact_locator_and_nowhere_else_observed_failing() {
        let line = "Realized-by: code:src/a.rs, code:lib/src/a.rs, code:src/a.rs#Foo, code:src/a.rs@sha256:ffff, test:src/a.rs";
        let out = pin_unpinned_in_line(line, "code", "src/a.rs", H).unwrap();
        assert_eq!(
            out,
            format!("Realized-by: code:src/a.rs@sha256:{H}, code:lib/src/a.rs, code:src/a.rs#Foo, code:src/a.rs@sha256:ffff, test:src/a.rs")
        );
        // The category matters: the same path under another category is untouched.
        let out = pin_unpinned_in_line(line, "test", "src/a.rs", H).unwrap();
        assert!(out.ends_with(&format!("test:src/a.rs@sha256:{H}")), "{out}");
        assert!(out.matches("@sha256:").count() == 2, "{out}");
        assert_eq!(pin_unpinned_in_line(line, "spec", "src/a.rs", H), None);
    }

    #[test]
    fn a_scoped_locator_keeps_its_spelling_and_a_clause_with_a_space_is_found() {
        let line = "- Built. -- evidence: code: src/a.rs#Foo::bar, test:t.rs";
        let out = pin_unpinned_in_line(line, "code", "src/a.rs#Foo::bar", H).unwrap();
        assert!(
            out.contains(&format!("code: src/a.rs#Foo::bar@sha256:{H}, test:t.rs")),
            "{out}"
        );
    }

    #[test]
    fn a_first_pin_is_not_spliced_into_a_longer_anchored_locator_observed_failing() {
        let line = "Realized-by: code:src/a.rs#Foo::bar, code:src/a.rs#Foo";
        let out = pin_unpinned_in_line(line, "code", "src/a.rs#Foo", H).unwrap();
        assert_eq!(
            out,
            format!("Realized-by: code:src/a.rs#Foo::bar, code:src/a.rs#Foo@sha256:{H}")
        );
        // Only the longer one on the line: the shorter spelling is not on it.
        assert_eq!(
            pin_unpinned_in_line("code:src/a.rs#Foo::bar", "code", "src/a.rs#Foo", H),
            None
        );
        assert_eq!(
            pin_unpinned_in_line("code:src/a.rs#Foo(x)", "code", "src/a.rs#Foo", H),
            None
        );
        let out = pin_unpinned_in_line("code:src/a.rs#Foo::bar", "code", "src/a.rs#Foo::bar", H);
        assert_eq!(out, Some(format!("code:src/a.rs#Foo::bar@sha256:{H}")));
    }

    #[test]
    fn the_same_locator_twice_on_a_line_is_pinned_twice() {
        let line = "evidence: code:a.rs and again code:a.rs , done";
        let out = pin_unpinned_in_line(line, "code", "a.rs", H).unwrap();
        assert_eq!(out.matches("@sha256:").count(), 2, "{out}");
    }
}
