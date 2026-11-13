//! `embodiment.claim-expired` and `embodiment.claim-renewal-unattested`
//! (`ADR-97`, `MILE-129`): a claim's verification expires after a period
//! declared per record type, status and evidence tier, and only
//! `urzua reverify` renews it.
//!
//! The standing of one claim (`claim_standing`) is shared with the command, so
//! the review report says exactly what the finding said.

use super::*;
use crate::config::{ClaimPeriod, RuleLevel};

const DAY_SECS: i64 = 86_400;

/// Where a claim's age is counted from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimAnchor {
    /// The date in the declared `anchor_field`: the day someone re-verified.
    Audited { field: String, date: String },
    /// The date of the commit that last touched the first evidence line: the
    /// `Realized-by` line, or for a record that has none its first Done-means
    /// item evidence clause.
    Line { date: String },
}

impl ClaimAnchor {
    /// The words a message uses: `Last-audited 2026-08-01` or `line last
    /// changed 2026-08-01`.
    pub fn describe(&self) -> String {
        match self {
            ClaimAnchor::Audited { field, date } => format!("{field} {date}"),
            ClaimAnchor::Line { date } => format!("line last changed {date}"),
        }
    }
}

/// One claim, measured: which entry judged it, what it is counted from, how
/// old it is and how long it may be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimStanding {
    /// The index of the `periods` entry that judged it.
    pub entry: usize,
    pub status: String,
    pub tier: String,
    pub days: u32,
    pub spread_days: u32,
    /// The entry's level, held to the rule's level.
    pub level: RuleLevel,
    /// Whether the entry declared a level lower than the rule's.
    pub level_below_rule: bool,
    pub anchor: ClaimAnchor,
    /// Whole UTC days from the anchor to today.
    pub age_days: i64,
    /// The first-cycle offset: `0` for a record with a valid anchor date.
    pub offset: u64,
    /// `days` plus the offset.
    pub limit: i64,
    /// False when the record has no `Stable-Id` to seed the offset from.
    pub seeded: bool,
    /// The last day the claim was valid.
    pub valid_through: String,
    /// A problem with the anchor field's value, reported beside the standing.
    pub anchor_problem: Option<String>,
}

impl ClaimStanding {
    pub fn expired(&self) -> bool {
        self.age_days > self.limit
    }

    /// The finding's message, which `urzua reverify` repeats in its report.
    pub fn message(&self, record_type: &str) -> String {
        let offset = if self.spread_days > 0 && matches!(self.anchor, ClaimAnchor::Line { .. }) {
            let seed = if self.seeded {
                String::new()
            } else {
                " (no Stable-Id, so the offset is 0)".to_string()
            };
            format!(
                " plus a first-cycle offset of {} of {} days ({} days in all){seed}",
                self.offset, self.spread_days, self.limit
            )
        } else {
            String::new()
        };
        format!(
            "the claim of this {record_type} ({}, computed tier {}) is {} days old, counted from {}, longer than the {}-day period{offset}; it was valid through {}",
            self.status,
            self.tier,
            self.age_days,
            self.anchor.describe(),
            self.days,
            self.valid_through
        )
    }
}

/// Why a record has no standing: it is eligible and not examined, or its age
/// cannot be measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimSkip {
    /// No locator field, a retained or sticky state, or no entry matches.
    Unexamined(&'static str),
    /// Neither a valid audit date nor a commit for the `Realized-by` line.
    NoAnchor,
    /// The header cannot be read.
    Unreadable,
}

fn rank(level: RuleLevel) -> u8 {
    match level {
        RuleLevel::Off => 0,
        RuleLevel::Info => 1,
        RuleLevel::Warn => 2,
        RuleLevel::Error => 3,
    }
}

/// The most specific entry that matches (`ADR-97` section 1): `type`, `status`
/// and `tier` outrank `type` and `status`, which outrank `type` alone.
pub fn matching_period<'a>(
    periods: &'a [ClaimPeriod],
    record_type: &str,
    status: &str,
    tier: &str,
) -> Option<(usize, &'a ClaimPeriod)> {
    periods
        .iter()
        .enumerate()
        .filter(|(_, p)| p.record_type == record_type)
        .filter(|(_, p)| {
            p.status
                .as_ref()
                .is_none_or(|s| s.iter().any(|x| x == status))
        })
        .filter(|(_, p)| p.tier.as_deref().is_none_or(|t| t == tier))
        .max_by_key(|(_, p)| usize::from(p.status.is_some()) + usize::from(p.tier.is_some()))
}

/// The tier a record's locators compute to under the `embodiment`
/// declaration, with no drift: the computed tier, not the stated one.
pub fn computed_tier(
    record: &Record,
    config: &Config,
    back_pointed: &BackPointerEvidence,
) -> String {
    let locator_field = relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::EmbodimentLocator,
    );
    let realized = evidence_text(record, locator_field)
        .map(|v| parse_realized_by(&v))
        .unwrap_or_default();
    compute_embodiment(
        &config.embodiment,
        &realized,
        false,
        back_pointed.code.contains(&record.path),
        back_pointed.test.contains(&record.path),
    )
    .to_string()
}

/// `YYYY-MM-DD` for a count of days since 1970-01-01 (proleptic Gregorian).
pub fn date_from_days(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// The 1-based file line a claim's age is counted from when no audit date
/// exists (`ADR-97`, amendment of 2026-11-08): the locator field's line when
/// it holds a value, else the first valid Done-means item evidence clause,
/// else an empty field's own line. `None` when the record has neither source.
/// One function for the rule, the measurement and `urzua reverify`.
pub fn evidence_anchor_line(record: &Record, locator_field: &str) -> Option<usize> {
    let field = record
        .header
        .fields
        .iter()
        .find(|f| f.key.as_str() == locator_field);
    if let Some(f) = field.filter(|f| !f.value.trim().is_empty()) {
        return Some(f.line);
    }
    record
        .items
        .as_ref()
        .and_then(|i| i.valid_evidence_lines().into_iter().min())
        .or_else(|| field.map(|f| f.line))
}

/// The `periods` entry that judges `record`, or why none does: the record has no
/// locator field, holds a retained or sticky state, or no entry matches. It
/// needs no clock and no history, so a claim that cannot be dated is still known
/// to be covered by its entry.
pub fn covering_entry<'a>(
    record: &Record,
    config: &Config,
    opts: &'a ClaimExpiredOpts,
    tier: &str,
) -> Result<(usize, &'a ClaimPeriod, String), ClaimSkip> {
    use crate::config::RelationRole::{EmbodimentState, Status};
    let status_field = relation_field_name(config, &record.record_type, Status);
    let locator_field = relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::EmbodimentLocator,
    );
    if record.header.is_unreadable() {
        return Err(ClaimSkip::Unreadable);
    }
    if evidence_text(record, locator_field).is_none() {
        return Err(ClaimSkip::Unexamined(
            "no locator field and no Done-means item evidence clause",
        ));
    }
    let status = record
        .header
        .get(status_field)
        .map(str::trim)
        .unwrap_or_default();
    // A retained claim is history; a sticky state is a human's (`ADR-97`).
    if config
        .record_types
        .get(&record.record_type)
        .and_then(|t| t.status_decl())
        .is_some_and(|d| d.claim_retained.iter().any(|s| s == status))
    {
        return Err(ClaimSkip::Unexamined("claim retained"));
    }
    let state_field = relation_field_name(config, &record.record_type, EmbodimentState);
    if let Some(stated) = record.header.get(state_field) {
        if sticky_states(config).contains(stated.trim()) {
            return Err(ClaimSkip::Unexamined("sticky state"));
        }
    }
    let Some((entry_index, entry)) =
        matching_period(&opts.periods, &record.record_type, status, tier)
    else {
        return Err(ClaimSkip::Unexamined("no period matches"));
    };
    Ok((entry_index, entry, status.to_string()))
}

/// Measures one record's claim, or says why it has no standing. `line_time` is
/// the commit time of the record's `Realized-by` line and `seed` the first 8
/// bytes of the SHA-256 of its `Stable-Id` as a big-endian number, both read by
/// the caller (the core opens no file and links no hash).
pub fn claim_standing(
    record: &Record,
    config: &Config,
    opts: &ClaimExpiredOpts,
    tier: &str,
    now_unix: i64,
    line_time: Option<i64>,
    seed: Option<u64>,
) -> Result<ClaimStanding, ClaimSkip> {
    let locator_field = relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::EmbodimentLocator,
    );
    let (entry_index, entry, status) = covering_entry(record, config, opts, tier)?;
    let today_day = now_unix.div_euclid(DAY_SECS);
    let mut anchor_problem = None;
    let mut anchor = None;
    if let Some(field) = opts.anchor_field.as_deref() {
        if let Some(value) = record.header.get(field).map(str::trim) {
            match date_unix(value) {
                Err(problem) => {
                    anchor_problem = Some(format!(
                        "anchor field {field} value '{value}' {problem}, so the claim's age counts from the {locator_field} line's last change"
                    ));
                }
                Ok(unix) if unix.div_euclid(DAY_SECS) > today_day => {
                    anchor_problem = Some(format!(
                        "anchor field {field} value '{value}' is in the future (a date written forward would extend a claim), so the claim's age counts from the {locator_field} line's last change"
                    ));
                }
                Ok(unix) => {
                    anchor = Some((
                        unix.div_euclid(DAY_SECS),
                        ClaimAnchor::Audited {
                            field: field.to_string(),
                            date: value.to_string(),
                        },
                    ));
                }
            }
        }
    }
    let spread = entry.spread_days.unwrap_or(0);
    let (anchor_day, anchor, offset) = match anchor {
        Some((day, anchor)) => (day, anchor, 0),
        None => {
            let Some(time) = line_time else {
                return Err(ClaimSkip::NoAnchor);
            };
            let day = time.div_euclid(DAY_SECS);
            let offset = seed.map_or(0, |s| s % (u64::from(spread) + 1));
            (
                day,
                ClaimAnchor::Line {
                    date: date_from_days(day),
                },
                offset,
            )
        }
    };
    let limit = i64::from(entry.days) + offset as i64;
    let entry_level = entry.level.unwrap_or(opts.level);
    let level = if rank(entry_level) > rank(opts.level) {
        opts.level
    } else {
        entry_level
    };
    Ok(ClaimStanding {
        entry: entry_index,
        status,
        tier: tier.to_string(),
        days: entry.days,
        spread_days: spread,
        level,
        level_below_rule: rank(level) < rank(opts.level),
        anchor,
        age_days: today_day - anchor_day,
        offset,
        limit,
        seeded: seed.is_some(),
        valid_through: date_from_days(anchor_day + limit),
        anchor_problem,
    })
}

/// What the caller measured for the expiry rule, per record.
pub struct ClaimInputs<'a> {
    pub now_unix: i64,
    /// The commit time of each record's `Realized-by` line.
    pub line_times: &'a HashMap<PathBuf, i64>,
    /// The seed of each record's first-cycle offset.
    pub seeds: &'a HashMap<PathBuf, u64>,
}

/// `embodiment.claim-expired` (`ADR-97` sections 1 to 3).
pub fn embodiment_claim_expired(
    records: &[Record],
    config: &Config,
    scope: Option<&ChangedOnly>,
    back_pointed: &BackPointerEvidence,
    inputs: &ClaimInputs<'_>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_CLAIM_EXPIRED;
    let opts: ClaimExpiredOpts = config.rule_opts(RULE_ID);
    let mut findings = Vec::new();
    let listed: Vec<&Record> = records
        .iter()
        .filter(|r| opts.periods.iter().any(|p| p.record_type == r.record_type))
        .collect();
    let standing_of = |record: &Record| {
        let tier = computed_tier(record, config, back_pointed);
        claim_standing(
            record,
            config,
            &opts,
            &tier,
            inputs.now_unix,
            inputs.line_times.get(&record.path).copied(),
            inputs.seeds.get(&record.path).copied(),
        )
    };
    let unmatched_entries = unmatched_period_entries(&listed, config, &opts, back_pointed);
    let mut candidates: Vec<ExpiryCandidate<'_>> =
        listed.into_iter().map(ExpiryCandidate::Record).collect();
    candidates.extend((0..unmatched_entries).map(|_| ExpiryCandidate::Entry));
    let (population, examined_records) = census_records(
        PopulationUnit::Record,
        candidates,
        |candidate| match candidate {
            ExpiryCandidate::Record(record) => record.path.clone(),
            ExpiryCandidate::Entry => PathBuf::new(),
        },
        |candidate| {
            let record = match candidate {
                ExpiryCandidate::Entry => return Outcome::OutOfScope,
                ExpiryCandidate::Record(record) => *record,
            };
            if config
                .record_types
                .get(&record.record_type)
                .is_some_and(|t| t.has_no_header())
            {
                return Outcome::OutOfScope;
            }
            let standing = match standing_of(record) {
                Ok(s) => s,
                Err(ClaimSkip::Unexamined(_)) => return Outcome::Absent,
                Err(ClaimSkip::NoAnchor | ClaimSkip::Unreadable) => return Outcome::Unreadable,
            };
            if scope.is_some_and(|s| s.defers(record)) {
                return Outcome::DeferredByScope;
            }
            findings.extend(claim_standing_findings(record, &opts, &standing));
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, examined_records),
        findings,
    )
}

/// One candidate per listed record, then one per `periods` entry: an entry
/// that no record matched is a declaration that reaches nothing, which
/// `config.scope-matches-nothing` reads as an out-of-scope candidate.
enum ExpiryCandidate<'a> {
    Record(&'a Record),
    Entry,
}

/// How many `periods` entries reach nothing. An entry is a declaration that
/// reaches nothing only when its type holds claims (a record with a locator)
/// and none is one the entry covers; a type with no claim yet has nothing for
/// the entry to match, as an empty corpus has nothing for any rule.
fn unmatched_period_entries(
    listed: &[&Record],
    config: &Config,
    opts: &ClaimExpiredOpts,
    back_pointed: &BackPointerEvidence,
) -> usize {
    let matched: HashSet<usize> = listed
        .iter()
        .filter_map(|r| {
            let tier = computed_tier(r, config, back_pointed);
            covering_entry(r, config, opts, &tier).ok().map(|(n, ..)| n)
        })
        .collect();
    let has_claims = |record_type: &str| {
        listed.iter().any(|r| {
            r.record_type == record_type
                && evidence_text(
                    r,
                    relation_field_name(
                        config,
                        &r.record_type,
                        crate::config::RelationRole::EmbodimentLocator,
                    ),
                )
                .is_some_and(|v| !v.trim().is_empty())
        })
    };
    opts.periods
        .iter()
        .enumerate()
        .filter(|(n, p)| !matched.contains(n) && has_claims(&p.record_type))
        .count()
}

/// The findings of one measured claim: a problem with its anchor field, then
/// its expiry.
fn claim_standing_findings(
    record: &Record,
    opts: &ClaimExpiredOpts,
    standing: &ClaimStanding,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(problem) = &standing.anchor_problem {
        let field = opts.anchor_field.as_deref().unwrap_or_default();
        findings.push(Finding::new(
            RULE_EMBODIMENT_CLAIM_EXPIRED,
            FindingSeverity::Warning,
            record.path.clone(),
            ActionSource::new("claim-anchor-invalid").field(field),
            problem.clone(),
        ));
    }
    if standing.expired() {
        findings.push(
            Finding::new(
                RULE_EMBODIMENT_CLAIM_EXPIRED,
                FindingSeverity::Warning,
                record.path.clone(),
                ActionSource::new("claim-expired").argv([
                    "urzua",
                    "reverify",
                    "--ids",
                    &record.path.display().to_string(),
                ]),
                standing.message(&record.record_type),
            )
            .declared_opt(
                standing
                    .level_below_rule
                    .then(|| standing.level.severity())
                    .flatten(),
            ),
        );
    }
    findings
}

/// The anchor date of a record's text, `None` when the field is absent or not a date.
fn anchor_in_text(text: &str, field: &str) -> Option<(String, i64)> {
    let header = crate::header::parse(text);
    let value = header.get(field)?.trim().to_string();
    let unix = date_unix(&value).ok()?;
    Some((value, unix))
}

/// The trimmed, non-empty value the text writes for `field`, `None` when the
/// field is absent or blank.
fn header_value(text: &str, field: &str) -> Option<String> {
    let header = crate::header::parse(text);
    let value = header.get(field)?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Per `Re-verified` row text, the UTC days (start, unix seconds) between which
/// it may be dated: from a day before the branch's first commit day to its adding commit's day.
pub type RowWindows = HashMap<String, (i64, i64)>;

/// What a `--base` run reads beyond the two copies of a record, for
/// `embodiment.claim-renewal-unattested`: the locators of each changed record
/// that had drifted at the merge base (empty without `--base`), the evaluation
/// time and, per changed record, the window of UTC days (the branch's first commit to the
/// commit that added it) in which each `Re-verified` row of its text may be
/// dated (a row git cannot date has no entry).
pub struct BaseReads<'a> {
    pub drift: &'a HashMap<PathBuf, HashSet<String>>,
    pub now_unix: i64,
    pub added_row_days: &'a HashMap<PathBuf, RowWindows>,
}

/// `embodiment.claim-renewal-unattested` (`ADR-97` section 6): on a `--base`
/// run, a changed record whose anchor date is newer than its merge-base copy's
/// and which gained no `Re-verified` row dated that day, or whose anchor the
/// merge-base copy carried and the branch removed or left malformed.
pub fn embodiment_claim_renewal_unattested(
    records: &[Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
    base_text: &HashMap<PathBuf, String>,
    changed: Option<&[PathBuf]>,
    base: &BaseReads<'_>,
    scope: Option<&ChangedOnly>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_EMBODIMENT_CLAIM_RENEWAL_UNATTESTED;
    let Some(changed) = changed else {
        return not_applicable(RULE_ID);
    };
    let opts: ClaimExpiredOpts = config.rule_opts(RULE_EMBODIMENT_CLAIM_EXPIRED);
    let Some(field) = opts.anchor_field.as_deref() else {
        return not_applicable(RULE_ID);
    };
    let changed: HashSet<&PathBuf> = changed.iter().collect();
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| opts.periods.iter().any(|p| p.record_type == r.record_type))
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            if scope.is_some_and(|s| s.defers(record)) {
                return Outcome::DeferredByScope;
            }
            // Only a changed record can have been renewed, and only one the
            // merge base held can be compared with it.
            if !changed.contains(&record.path) {
                return Outcome::Absent;
            }
            let (Some(now_text), Some(then_text)) =
                (full_text.get(&record.path), base_text.get(&record.path))
            else {
                return Outcome::Absent;
            };
            let problems = renewal_problem(config, record, field, now_text, then_text)
                .into_iter()
                .chain(acknowledgement_problems(
                    config,
                    record,
                    (now_text, then_text),
                    base.drift.get(&record.path),
                    (base.now_unix, base.added_row_days.get(&record.path)),
                ));
            for message in problems {
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Warning,
                    record.path.clone(),
                    ActionSource::new("claim-renewal-unattested"),
                    message,
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, examined_records),
        findings,
    )
}

/// The record's current evidence locators, each with the hash its pin holds
/// (`None` when unpinned or unparseable), in the order written.
fn current_locator_pins(config: &Config, record: &Record) -> Vec<(String, Option<String>)> {
    let locator_field = relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::EmbodimentLocator,
    );
    delivery_items::evidence_text(record, locator_field)
        .map(|value| {
            crate::locator::realized_by_locators(&value)
                .into_iter()
                .map(|e| match e.parsed {
                    Ok(l) => {
                        let pin = match &l.anchor {
                            Some(crate::locator::Anchor::Pin(hash)) => Some(hash.clone()),
                            _ => None,
                        };
                        (l.display(), pin)
                    }
                    Err(_) => (e.raw, None),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// What is wrong with the drift acknowledgements a branch added to a record:
/// each acknowledging `Re-verified` row the merge-base copy did not carry must
/// name only locators that had drifted at the merge base (`base_drift`, read
/// by the caller; absent when it could not be) and list the record's current
/// locators as its evidence. A row typed by hand that does not describe the
/// real drift is therefore a finding at the pull request, and a row the tool
/// wrote passes, as a re-pin row does once a reviewer has read it.
fn acknowledgement_problems(
    config: &Config,
    record: &Record,
    (now_text, then_text): (&str, &str),
    drifted: Option<&HashSet<String>>,
    (now_unix, added_days): (i64, Option<&RowWindows>),
) -> Vec<String> {
    let id = config.revision_log_id(&record.record_type);
    let shape = config.revision_log_shape(&record.record_type);
    let before: Vec<String> = crate::reverify::acknowledgements(then_text, &id, &shape)
        .into_iter()
        .map(|a| a.row)
        .collect();
    let current_pins = current_locator_pins(config, record);
    let current: HashSet<String> = current_pins.iter().map(|(l, _)| l.clone()).collect();
    let mut problems = Vec::new();
    for ack in crate::reverify::acknowledgements(now_text, &id, &shape)
        .into_iter()
        .filter(|a| !before.contains(&a.row))
    {
        let listed: HashSet<String> = ack.evidence.iter().cloned().collect();
        if listed != current {
            problems.push(format!(
                "the Re-verified row dated {} lists evidence that is not the record's current locators ({}): only `urzua reverify` writes the row for the claim it re-verified",
                ack.date,
                ack.evidence.join(", ")
            ));
        }
        for (locator, hash) in &ack.pins {
            let held = current_pins
                .iter()
                .find(|(l, _)| l == locator)
                .and_then(|(_, pin)| pin.as_deref());
            if !held.is_some_and(|pin| !hash.is_empty() && pin.starts_with(hash.as_str())) {
                problems.push(format!(
                    "the Re-verified row dated {} lists the pin {hash} for {locator}, which is not the pin the record now holds: only `urzua reverify` writes the row for the claim it re-verified",
                    ack.date
                ));
            }
        }
        let misdated = date_unix(&ack.date).is_ok_and(|day| {
            day > now_unix
                || added_days
                    .and_then(|d| d.get(&ack.row))
                    .is_some_and(|(first, added)| day < *first || day > *added)
        });
        if misdated {
            problems.push(format!(
                "the Re-verified row dated {} is not dated within the branch (after the evaluation date, more than a day before the UTC day of the branch's first commit, or after the UTC day of the commit that added it): only `urzua reverify` writes the row, dated the day it ran",
                ack.date
            ));
        }
        if let Some(drifted) = drifted {
            let invented: Vec<&str> = ack
                .names
                .iter()
                .filter(|n| !drifted.contains(*n))
                .map(String::as_str)
                .collect();
            if !invented.is_empty() {
                problems.push(format!(
                    "the Re-verified row dated {} acknowledges drift of {}, which had not drifted at the merge base: only `urzua reverify` writes the row for the drift it read",
                    ack.date,
                    invented.join(", ")
                ));
            }
        }
    }
    problems
}

/// What is wrong with a changed record's anchor date against its merge-base
/// copy: removed, or moved forward with no `Re-verified` row dated that day.
fn renewal_problem(
    config: &Config,
    record: &Record,
    field: &str,
    now_text: &str,
    then_text: &str,
) -> Option<String> {
    let now = anchor_in_text(now_text, field);
    let then = anchor_in_text(then_text, field);
    match (&now, &then) {
        (None, Some((date, _))) => {
            let present = crate::header::parse(now_text).get(field).is_some();
            Some(match (header_value(now_text, field), present) {
                (Some(value), _) => format!(
                    "the anchor field {field} ('{value}') is malformed, not a date: it carried {date} at the merge base, and an unreadable date is not a way back to an earlier first-cycle offset"
                ),
                (None, true) => format!(
                    "the anchor field {field} is empty: it carried {date} at the merge base, and a blank date is not a way back to an earlier first-cycle offset"
                ),
                (None, false) => format!(
                    "the anchor field {field} ({date} at the merge base) was removed: deleting it is not a way back to an earlier first-cycle offset"
                ),
            })
        }
        (Some((date, new)), then) if then.as_ref().is_none_or(|(_, old)| new > old) => {
            let id = config.revision_log_id(&record.record_type);
            let shape = config.revision_log_shape(&record.record_type);
            let attested = find_revision_log_entries(now_text, &id, &shape)
                .and_then(Result::ok)
                .is_some_and(|log| {
                    log.entries.iter().any(|e| {
                        e.date.trim() == date
                            && crate::reverify::parse_reverified_row(&e.row).is_some()
                    })
                });
            (!attested).then(|| {
                format!(
                    "the anchor field {field} moved to {date} with no Re-verified revision row dated {date}: only `urzua reverify` renews a claim"
                )
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_from_days_inverts_date_unix() {
        for s in [
            "1970-01-01",
            "2000-03-01",
            "2026-11-07",
            "2027-02-28",
            "2028-02-29",
        ] {
            let days = date_unix(s).unwrap() / DAY_SECS;
            assert_eq!(date_from_days(days), s);
        }
    }

    fn period(ty: &str, status: Option<&[&str]>, tier: Option<&str>, days: u32) -> ClaimPeriod {
        ClaimPeriod {
            record_type: ty.to_string(),
            status: status.map(|s| s.iter().map(|x| x.to_string()).collect()),
            tier: tier.map(str::to_string),
            days,
            spread_days: None,
            level: None,
        }
    }

    #[test]
    fn the_most_specific_entry_wins() {
        let periods = vec![
            period("adr", None, None, 1),
            period("adr", Some(&["Accepted"]), None, 2),
            period("adr", Some(&["Accepted"]), Some("Specified"), 3),
        ];
        let days =
            |status, tier| matching_period(&periods, "adr", status, tier).map(|(_, p)| p.days);
        assert_eq!(days("Accepted", "Specified"), Some(3));
        assert_eq!(days("Accepted", "Implemented"), Some(2));
        assert_eq!(days("Proposed", "Implemented"), Some(1));
        assert_eq!(matching_period(&periods, "bug", "Fixed", "Verified"), None);
    }
}
