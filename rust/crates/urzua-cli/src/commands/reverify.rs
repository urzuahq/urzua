//! `urzua reverify` (`ADR-97` section 4, `MILE-129`): the one attributed way to
//! renew a claim. Review first (read only), then `--apply`. It never moves an
//! existing pin: a changed pin is `fix --rebaseline`'s. It writes the first pin
//! on a `code:` or `test:` locator that has none, because a claim is not renewed
//! over evidence only git history vouches for; `--pin-only` writes those pins
//! and renews nothing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{Config, DriftPolicy, RuleLevel};
use urzua_core::fix::{revision_log_refusal, ActorKind, Attestation, PinRead};
use urzua_core::locator::Locator;
use urzua_core::record::Record;
use urzua_core::report::{
    census, CouldNotRun, FixStatus, Notice, Outcome, Population, PopulationUnit,
};
use urzua_core::reverify::{
    apply_reverify, judge_drift, latest_revision_date, review_locators, FirstPin, LocatorCheck,
    LocatorReview, LocatorsReview, RecordReview, Renewal, Repin, ReverifyPlan, ReverifyReport,
    REPIN_DRIFT_REFUSAL,
};
use urzua_core::rules::{self, ClaimExpiredOpts, ClaimSkip};

use crate::commands::fix::{
    back_pointer_evidence, excused_evidence_notice, identity_notices, into_failures, refuse,
    resolve_attestation, rule_enabled, validate_reason, Refusal, ResolvedAttestation,
};
use crate::discovery::{ClaimMeasures, DriftedRecords};
use crate::emit;
use crate::session::Session;

/// The `urzua reverify` command line.
pub struct ReverifyArgs {
    pub ids: Vec<String>,
    pub apply: bool,
    pub pin_only: bool,
    pub by: Option<String>,
    pub actor: ActorKind,
    pub force: bool,
    pub reason: Option<String>,
}

/// One record under review, with what the write needs.
struct Reviewed {
    review: RecordReview,
    locators: LocatorsReview,
    pins: Vec<FirstPin>,
    /// Pinned locators a renewal moves (`drift: acknowledge`).
    repins: Vec<Repin>,
    renewal: Option<(i64, u32)>,
    /// The drift a renewal acknowledges in its row (`drift: acknowledge`).
    acknowledged: Vec<String>,
}

pub fn run(config_path: Option<PathBuf>, args: ReverifyArgs) -> ExitCode {
    run_checked(config_path, args).unwrap_or_else(|e| emit(&CouldNotRun::from(e)))
}

/// `run` with every refusal as an `Err`, so each step can stop with `?`.
fn run_checked(config_path: Option<PathBuf>, args: ReverifyArgs) -> Result<ExitCode, Refusal> {
    let reason = check_usage(&args)?;
    let Session {
        repo_root,
        config,
        discovered,
        records,
        ..
    } = Session::open(config_path).map_err(refuse)?;
    if !rule_enabled(&config, rules::RULE_EMBODIMENT_CLAIM_EXPIRED) {
        return Err(refuse(
            "urzua reverify renews the claims that embodiment.claim-expired judges, and that rule is off or undeclared in the configuration: declare it (ADR-97) first",
        ));
    }
    let opts: ClaimExpiredOpts = config.rule_opts(rules::RULE_EMBODIMENT_CLAIM_EXPIRED);
    if args.apply && !args.force && args.ids.len() > opts.max_batch {
        return Err(refuse(format!(
            "urzua reverify --apply names {} records, more than max_batch of {}: name at most {} and run another batch with its own reason",
            args.ids.len(),
            opts.max_batch,
            opts.max_batch
        )));
    }
    let identity = if args.apply {
        Some(
            resolve_attestation(args.by.as_deref(), args.actor, &repo_root, &config)
                .map_err(refuse)?,
        )
    } else {
        None
    };
    // The pin hashes written come from this review, so the lock is taken first
    // (as `fix --rebaseline` does): evidence edited between a review and the
    // lock would otherwise be hashed stale and attested as current.
    let _lock = if args.apply {
        Some(urzua_io::FixLock::acquire(&repo_root).map_err(refuse)?)
    } else {
        None
    };

    // `--ids` names the records, with or without `--force` (which lifts only
    // the `max_batch` cap and the need to name one).
    let (subset, failed) = select_subset(&records, &opts, &args.ids);
    let now = urzua_io::now_unix();
    let evidence = gather_evidence(&repo_root, &config, &discovered, &records, &subset, now)?;
    let texts = crate::file_texts::FileTexts::new(repo_root.clone(), &config);
    let read =
        |l: &urzua_core::locator::Locator| crate::pins::read_pin(&repo_root, &config, &texts, l);
    let scan = ReviewScan {
        repo_root: &repo_root,
        config: &config,
        records: &records,
        opts: &opts,
        now,
        measures: &evidence.measures,
        drift: &evidence.drift,
        back_pointed: &evidence.back_pointed,
        disagreeing: &evidence.disagreeing,
        read: &read,
        actor: args.actor,
        pin_only: args.pin_only,
    };
    let mut reviewed: Vec<Reviewed> = Vec::new();
    let population = census(PopulationUnit::Record, subset.iter().collect(), |record| {
        reviewed.push(scan.review(record));
        Outcome::Examined
    });

    retain_listed(&mut reviewed, !args.ids.is_empty(), args.pin_only);

    let batch = Batch {
        reviewed,
        population,
        failed,
    };
    let Some(identity) = identity else {
        return Ok(emit(&detect_report(args.pin_only, batch, &evidence)));
    };
    let write = RenewalWrite {
        repo_root: &repo_root,
        records: &records,
        config: &config,
        opts: &opts,
        pin_only: args.pin_only,
        reason: &reason,
    };
    apply_batch(&write, args.force, batch, identity, &evidence)
}

/// What the records under review are narrowed to.
fn retain_listed(reviewed: &mut Vec<Reviewed>, by_name: bool, pin_only: bool) {
    // What the mode lists: with names, those; otherwise what it finds.
    reviewed.retain(|r| {
        if by_name {
            true
        } else if pin_only {
            r.review
                .locators
                .iter()
                .any(|l| l.check == LocatorCheck::WillPin)
        } else {
            r.review.finding.is_some()
        }
    });
}

/// What the claim measures, drift and back-pointer scan found for a review.
struct Evidence {
    measures: ClaimMeasures,
    drift: DriftedRecords,
    back_pointed: rules::BackPointerEvidence,
    disagreeing: HashSet<PathBuf>,
}

fn gather_evidence(
    repo_root: &Path,
    config: &Config,
    discovered: &urzua_io::DiscoveredFiles,
    records: &[Record],
    subset: &[Record],
    now: i64,
) -> Result<Evidence, Refusal> {
    let (measures, drift) =
        crate::discovery::measure_claims_and_drift(repo_root, subset, config, now);
    let back_pointed =
        back_pointer_evidence(repo_root, discovered, config, records).map_err(refuse)?;
    let disagreeing = if rule_enabled(config, rules::RULE_EMBODIMENT_CONSISTENCY) {
        let judge = |drifted: &HashSet<PathBuf>, fresh: &HashSet<PathBuf>| {
            embodiment_disagreements(
                repo_root,
                config,
                records,
                subset,
                discovered,
                (drifted, fresh),
                &back_pointed,
            )
        };
        let mut disagreeing = judge(&drift.drifted, &drift.fresh);
        // Implements: ADR-97 (drift-policy amendment, consistency): a record
        // whose only drift is in categories that acknowledge it is renewable
        // although `embodiment.consistency` reports it now, because the
        // renewal clears that drift. It is refused only for a disagreement
        // that would remain without the acknowledged drift.
        let acknowledged = acknowledged_only(&drift, config);
        if !disagreeing.is_empty() && !acknowledged.is_empty() {
            let drifted = drift.drifted.difference(&acknowledged).cloned().collect();
            let fresh = drift.fresh.difference(&acknowledged).cloned().collect();
            let remaining = judge(&drifted, &fresh);
            disagreeing.retain(|path| !acknowledged.contains(path) || remaining.contains(path));
        }
        disagreeing
    } else {
        HashSet::new()
    };
    Ok(Evidence {
        measures,
        drift,
        back_pointed,
        disagreeing,
    })
}

/// The records under review, the population they came from and the failures so
/// far.
struct Batch {
    reviewed: Vec<Reviewed>,
    population: Population,
    failed: Vec<(PathBuf, String)>,
}

/// The detect-mode report: what a reviewer reads before `--apply`.
fn detect_report(pin_only: bool, batch: Batch, evidence: &Evidence) -> ReverifyReport {
    let Batch {
        reviewed,
        population,
        failed,
    } = batch;
    let status = if !failed.is_empty() {
        FixStatus::PartialFailure
    } else if reviewed.iter().any(|r| r.review.refused.is_empty()) {
        FixStatus::RepairsAvailable
    } else {
        FixStatus::Ok
    };
    ReverifyReport {
        as_of: crate::as_of(),
        status,
        mode: "detect",
        pin_only,
        population,
        reason: None,
        records: reviewed.into_iter().map(|r| r.review).collect(),
        failed: into_failures(failed),
        identity: None,
        notices: drift_notices(
            &evidence.drift,
            &evidence.measures.problems,
            &evidence.back_pointed,
        ),
    }
}

/// `reverify --apply`: write each reviewed record that nothing refused.
fn apply_batch(
    write: &RenewalWrite<'_>,
    force: bool,
    batch: Batch,
    identity: ResolvedAttestation,
    evidence: &Evidence,
) -> Result<ExitCode, Refusal> {
    let Batch {
        reviewed,
        population,
        mut failed,
    } = batch;
    if force && reviewed.len() > 1 {
        return Err(refuse(format!(
            "urzua reverify --force is refused when it selects more than one record ({} selected): it lifts max_batch for a single record and never batches",
            reviewed.len()
        )));
    }
    let mut written: Vec<RecordReview> = Vec::new();
    for item in reviewed {
        match write.apply(&item, &identity.attestation) {
            Ok(()) => written.push(item.review),
            Err(e) => failed.push((item.review.record.clone(), e)),
        }
    }
    Ok(emit(&apply_report(
        ApplyOutcome {
            written,
            failed,
            population,
            pin_only: write.pin_only,
            reason: write.reason.to_string(),
            identity,
        },
        drift_notices(
            &evidence.drift,
            &evidence.measures.problems,
            &evidence.back_pointed,
        ),
    )))
}

/// The usage errors of the command line that need no repository, in the order
/// they are reported; the validated `--reason` (empty when none applies).
fn check_usage(args: &ReverifyArgs) -> Result<String, Refusal> {
    if args.apply && args.ids.is_empty() && !args.force {
        return Err(refuse(
            "urzua reverify --apply: pass --ids <record,...> or --force (never the default)",
        ));
    }
    if !args.apply
        && (args.force
            || args.by.is_some()
            || args.reason.is_some()
            || args.actor != ActorKind::Unspecified)
    {
        return Err(refuse(
            "urzua reverify --force, --by, --actor and --reason belong to --apply: without it nothing is written, so they would be ignored (--ids alone narrows the review)",
        ));
    }
    validate_reason(
        args.apply,
        args.reason.as_deref(),
        "urzua reverify --apply: pass --reason <what you checked>; a renewal with no reason is a rubber stamp",
    )
    .map_err(refuse)
}

/// The records a review covers, and a failure for each `--ids` entry that names
/// no record of a type the expiry rule lists.
fn select_subset(
    records: &[Record],
    opts: &ClaimExpiredOpts,
    ids: &[String],
) -> (Vec<Record>, Vec<(PathBuf, String)>) {
    let listed: Vec<&Record> = records
        .iter()
        .filter(|r| opts.periods.iter().any(|p| p.record_type == r.record_type))
        .collect();
    let mut failed: Vec<(PathBuf, String)> = Vec::new();
    if ids.is_empty() {
        return (listed.iter().map(|r| (*r).clone()).collect(), failed);
    }
    for id in ids {
        if !listed.iter().any(|r| r.path.display().to_string() == *id) {
            failed.push((
                PathBuf::from(id),
                format!(
                    "--ids entry '{id}' matched no record of a type {} lists (an entry is a record's repo-relative path)",
                    rules::RULE_EMBODIMENT_CLAIM_EXPIRED
                ),
            ));
        }
    }
    let subset = listed
        .iter()
        .filter(|r| ids.contains(&r.path.display().to_string()))
        .map(|r| (*r).clone())
        .collect();
    (subset, failed)
}

/// Everything the review of one record reads.
struct ReviewScan<'a> {
    repo_root: &'a Path,
    config: &'a Config,
    records: &'a [Record],
    opts: &'a ClaimExpiredOpts,
    now: i64,
    measures: &'a ClaimMeasures,
    drift: &'a DriftedRecords,
    back_pointed: &'a rules::BackPointerEvidence,
    disagreeing: &'a HashSet<PathBuf>,
    read: &'a dyn Fn(&Locator) -> PinRead,
    actor: ActorKind,
    pin_only: bool,
}

/// What the expiry rule says about a record's claim.
#[derive(Default)]
struct Standing {
    status: String,
    finding: Option<String>,
    age_days: Option<i64>,
    period_days: Option<i64>,
    renewal: Option<(i64, u32)>,
}

impl ReviewScan<'_> {
    /// One record under review: its locators, its claim's standing and every
    /// reason a write would be refused.
    fn review(&self, record: &Record) -> Reviewed {
        let config = self.config;
        let locators = review_locators(record, config, &self.opts.require_pin, self.read);
        let tier = rules::computed_tier(record, config, self.back_pointed);
        let mut refused: Vec<String> = Vec::new();
        let standing = self.standing(record, &tier, &mut refused);
        locator_refusals(&locators, self.opts, &mut refused);
        let pins = FirstPin::from_review(&locators.locators);
        let (acknowledged, repins) = self.mode_refusals(record, &pins, &locators, &mut refused);
        self.log_refusals(record, &mut refused);
        let mut commits_since = Vec::new();
        if self.pin_only && !pins.is_empty() {
            commits_since = commits_since_line(self.repo_root, record, &locators, &pins, config);
        }
        Reviewed {
            review: RecordReview {
                record: record.path.clone(),
                status: standing.status,
                tier,
                finding: standing.finding,
                age_days: standing.age_days,
                period_days: standing.period_days,
                locators: locators.locators.clone(),
                commits_since,
                refused,
                drift_acknowledged: acknowledged.clone(),
            },
            locators,
            pins,
            repins,
            renewal: standing.renewal,
            acknowledged,
        }
    }

    /// The claim's standing under the expiry rule, or why the rule does not
    /// judge it.
    fn standing(&self, record: &Record, tier: &str, refused: &mut Vec<String>) -> Standing {
        let judged = rules::claim_standing(
            record,
            self.config,
            self.opts,
            tier,
            self.now,
            self.measures.line_times.get(&record.path).copied(),
            self.measures.seeds.get(&record.path).copied(),
        );
        let mut out = Standing::default();
        match &judged {
            Ok(s) => {
                out.status.clone_from(&s.status);
                out.age_days = Some(s.age_days);
                out.period_days = Some(s.limit);
                if s.expired() {
                    out.finding = Some(s.message(&record.record_type));
                }
                out.renewal = Some((s.age_days, s.days));
                if s.level == RuleLevel::Error && self.actor != ActorKind::Human {
                    refused.push("a human actor is required where the period's level is error: pass --actor human (--actor agent and an unspecified actor are refused for this record)".to_string());
                }
            }
            Err(ClaimSkip::Unexamined(why)) => {
                refused.push(format!(
                    "the expiry rule does not judge this record ({why})"
                ));
            }
            Err(ClaimSkip::NoAnchor) => {
                refused.push("the claim cannot be dated: no valid anchor date and no commit for the first evidence line, the Realized-by line or the first Done-means item evidence clause (an untracked record?)".to_string());
            }
            Err(ClaimSkip::Unreadable) => refused.push("the header cannot be read".to_string()),
        }
        out
    }

    /// The refusals that depend on the mode: `--pin-only` needs something to
    /// pin; a renewal needs an anchor field and locators whose drift the
    /// per-category policy lets through (`ADR-97`, amendment of 2026-11-09).
    /// Returns the drift a renewal would acknowledge in its row and the
    /// pins it would move.
    fn mode_refusals(
        &self,
        record: &Record,
        pins: &[FirstPin],
        locators: &LocatorsReview,
        refused: &mut Vec<String>,
    ) -> (Vec<String>, Vec<Repin>) {
        if self.pin_only {
            if pins.is_empty() {
                refused.push("no unpinned locator of a required category to pin".to_string());
            }
            return (Vec::new(), Vec::new());
        }
        if self.opts.anchor_field.is_none() {
            refused.push(format!(
                "{} declares no anchor_field: there is nowhere to record the date, so the claim cannot be renewed",
                rules::RULE_EMBODIMENT_CLAIM_EXPIRED
            ));
        }
        let mut acknowledged = self.drift_refusals(record, locators, refused);
        // A pinned locator that moved is re-pinned with the renewal, so the
        // pin does not stay stale; the row names the move.
        let repins = Repin::from_review(&locators.locators, self.opts);
        let line_commit = self.line_commit(record, locators);
        for l in locators.locators.iter().filter(|l| {
            repins
                .iter()
                .any(|r| r.display == l.locator && l.recorded_hash.as_deref() == Some(&r.old_hash))
        }) {
            let text = self.describe_drift(l, line_commit.as_deref());
            if !acknowledged.contains(&text) {
                acknowledged.push(text);
            }
        }
        if self.disagreeing.contains(&record.path) {
            refused.push("the stated Embodiment disagrees with the one its locators compute to (embodiment.consistency would report it): run `urzua fix` first".to_string());
        }
        (acknowledged, repins)
    }

    /// The commit that last touched the record's `Realized-by` line, when git
    /// can say: the point a drifted file's later commits are counted from.
    fn line_commit(&self, record: &Record, locators: &LocatorsReview) -> Option<String> {
        if locators.field_line == 0 {
            return None;
        }
        urzua_io::commit_for_line(self.repo_root, &record.path, locators.field_line)
            .ok()
            .flatten()
    }

    /// The drift policy applied to the record's drifted locators: what blocks
    /// the renewal is refused, what is acknowledged is returned for the row.
    fn drift_refusals(
        &self,
        record: &Record,
        locators: &LocatorsReview,
        refused: &mut Vec<String>,
    ) -> Vec<String> {
        let drifted =
            self.drift.drifted.contains(&record.path) || self.drift.fresh.contains(&record.path);
        if !drifted {
            return Vec::new();
        }
        let Some(found) = self.drift.locators.get(&record.path) else {
            refused.push(REPIN_DRIFT_REFUSAL.to_string());
            return Vec::new();
        };
        let line_commit = self.line_commit(record, locators);
        let verdict = judge_drift(found, locators, self.opts, &|l| {
            self.describe_drift(l, line_commit.as_deref())
        });
        refused.extend(verdict.refusals);
        verdict.acknowledged
    }

    /// How the revision row names an acknowledged locator: a cited record by
    /// its id and its newest revision-log row; any other file by its path, the
    /// prefix of the hash the reviewer's tool read and the commit that last
    /// changed it since the `Realized-by` line (`line_commit`). A pinned
    /// locator that the renewal re-pins shows its old and new hash prefixes.
    fn describe_drift(&self, l: &LocatorReview, line_commit: Option<&str>) -> String {
        let short = |h: &str| h.get(..12).unwrap_or(h).to_string();
        let moved = match (
            l.check,
            l.recorded_hash.as_deref(),
            l.current_hash.as_deref(),
        ) {
            (LocatorCheck::Changed, Some(old), Some(new)) => {
                Some(format!("{} -> {}", short(old), short(new)))
            }
            _ => None,
        };
        let cited = self
            .records
            .iter()
            .find(|r| r.path.to_str() == Some(l.path.as_str()));
        let Some(cited) = cited else {
            let hash = match (&moved, l.read_hash.as_deref()) {
                (Some(moved), _) => format!("@sha256:{moved}"),
                (None, Some(h)) => format!("@sha256:{}", short(h)),
                (None, None) => String::new(),
            };
            let commit = self
                .last_change(&l.path, line_commit)
                .map_or(String::new(), |c| format!(" (last changed in {c})"));
            return format!("{}:{}{hash}{commit}", l.category, l.locator);
        };
        let id = rules::record_id(cited).unwrap_or_else(|| l.path.clone());
        let latest = urzua_io::read_to_string(&self.repo_root.join(&cited.path))
            .ok()
            .and_then(|content| {
                latest_revision_date(
                    &content,
                    &self.config.revision_log_id(&cited.record_type),
                    &self.config.revision_log_shape(&cited.record_type),
                )
            });
        let revision = latest.map_or("no dated revision row".to_string(), |date| {
            format!("latest revision {date}")
        });
        match moved {
            Some(moved) => format!("{}:{id} ({revision}; pin {moved})", l.category),
            None => format!("{}:{id} ({revision})", l.category),
        }
    }

    /// The newest commit that changed `path` after the `Realized-by` line was
    /// last touched (its short hash), when git can say.
    fn last_change(&self, path: &str, line_commit: Option<&str>) -> Option<String> {
        let line_commit = line_commit?;
        let commits = urzua_io::commits_touching_since(self.repo_root, line_commit, path).ok()?;
        commits
            .first()
            .and_then(|c| c.split_whitespace().next())
            .map(str::to_string)
    }

    /// Detect never offers a write `--apply` would refuse: both modes append
    /// an attested row to the record's revision log.
    fn log_refusals(&self, record: &Record, refused: &mut Vec<String>) {
        let log_id = self.config.revision_log_id(&record.record_type);
        let log_shape = self.config.revision_log_shape(&record.record_type);
        match urzua_io::read_to_string(&self.repo_root.join(&record.path)) {
            Ok(content) => {
                refused.extend(revision_log_refusal(&content, &log_id, &log_shape, "write"))
            }
            Err(e) => refused.push(format!("could not read the record: {e}")),
        }
    }
}

/// The refusals for locators that do not resolve or no longer match their pin.
/// A pin that moved is drift, so a category whose drift policy is not `block`
/// is judged by that policy instead (`ADR-97`, amendment of 2026-11-09).
fn locator_refusals(locators: &LocatorsReview, opts: &ClaimExpiredOpts, refused: &mut Vec<String>) {
    for l in &locators.locators {
        match l.check {
            LocatorCheck::Unresolved => refused.push(format!(
                "locator '{}' does not resolve: {}",
                l.locator,
                l.reason.as_deref().unwrap_or("not found")
            )),
            LocatorCheck::Changed if opts.drift_policy(&l.category) == DriftPolicy::Block => {
                refused.push(format!(
                    "pinned locator '{}' no longer matches its pin: run `urzua fix --rebaseline` first, with its own reason",
                    l.locator
                ))
            }
            _ => {}
        }
    }
}

/// What `--apply` writes a reviewed record with.
struct RenewalWrite<'a> {
    repo_root: &'a Path,
    records: &'a [Record],
    config: &'a Config,
    opts: &'a ClaimExpiredOpts,
    pin_only: bool,
    reason: &'a str,
}

impl RenewalWrite<'_> {
    /// Writes one reviewed record; the error is the failure's message.
    fn apply(&self, item: &Reviewed, attestation: &Attestation) -> Result<(), String> {
        let review = &item.review;
        if !review.refused.is_empty() {
            return Err(format!("refused: {}", review.refused.join("; ")));
        }
        let full = self.repo_root.join(&review.record);
        let fresh =
            urzua_io::read_to_string(&full).map_err(|e| format!("could not re-read: {e}"))?;
        let record_type = self
            .records
            .iter()
            .find(|r| r.path == review.record)
            .map(|r| r.record_type.as_str());
        let log_id = record_type
            .map(|t| self.config.revision_log_id(t))
            .unwrap_or_default();
        let log_shape = record_type
            .map(|t| self.config.revision_log_shape(t))
            .unwrap_or_default();
        let renewal = if self.pin_only {
            None
        } else {
            match (self.opts.anchor_field.as_deref(), item.renewal) {
                (Some(anchor_field), Some((age_days, days))) => Some(Renewal {
                    anchor_field,
                    age_days,
                    days,
                    require_pin_empty: self.opts.require_pin.is_empty(),
                    acknowledged: &item.acknowledged,
                }),
                _ => None,
            }
        };
        let plan = ReverifyPlan {
            review: &item.locators,
            pins: &item.pins,
            repins: if self.pin_only { &[] } else { &item.repins },
            renewal,
        };
        let new = apply_reverify(
            &fresh,
            &plan,
            &urzua_io::today(),
            attestation,
            self.reason,
            &log_id,
            &log_shape,
        )?;
        urzua_io::write_atomic_in(self.repo_root, &full, &new)
            .map_err(|e| format!("could not write: {e}"))
    }
}

/// What an apply run produced.
struct ApplyOutcome {
    written: Vec<RecordReview>,
    failed: Vec<(PathBuf, String)>,
    population: Population,
    pin_only: bool,
    reason: String,
    identity: ResolvedAttestation,
}

fn apply_report(outcome: ApplyOutcome, drift_notices: Vec<Notice>) -> ReverifyReport {
    let ApplyOutcome {
        written,
        failed,
        population,
        pin_only,
        reason,
        identity,
    } = outcome;
    let status = if !failed.is_empty() {
        FixStatus::PartialFailure
    } else {
        FixStatus::Applied
    };
    let mut notices: Vec<Notice> = identity_notices(Some(&identity));
    notices.extend(drift_notices);
    ReverifyReport {
        as_of: crate::as_of(),
        status,
        mode: "apply",
        pin_only,
        population,
        reason: Some(reason),
        records: written,
        failed: into_failures(failed),
        identity: Some(identity.report),
        notices,
    }
}

fn drift_notices(
    drift: &crate::discovery::DriftedRecords,
    problems: &[String],
    back_pointed: &rules::BackPointerEvidence,
) -> Vec<Notice> {
    let mut all = drift.git_problems.clone();
    for p in problems {
        if !all.contains(p) {
            all.push(p.clone());
        }
    }
    let mut notices = crate::discovery::drift_notices(drift.shallow, &all);
    notices.extend(excused_evidence_notice(back_pointed));
    notices
}

/// The commits that touched an unpinned locator's file since the record's
/// `Realized-by` line was last touched, for the reviewer to read (`ADR-97`).
fn commits_since_line(
    repo_root: &std::path::Path,
    record: &Record,
    locators: &LocatorsReview,
    pins: &[FirstPin],
    _config: &Config,
) -> Vec<String> {
    if locators.field_line == 0 {
        return Vec::new();
    }
    let Ok(Some(line_commit)) =
        urzua_io::commit_for_line(repo_root, &record.path, locators.field_line)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pin in pins {
        let path = pin.display.split('#').next().unwrap_or(&pin.display);
        if let Ok(commits) = urzua_io::commits_touching_since(repo_root, &line_commit, path) {
            for c in commits {
                let entry = format!("{path}: {c}");
                if !out.contains(&entry) {
                    out.push(entry);
                }
            }
        }
    }
    out
}

/// The records whose stated `Embodiment` `embodiment.consistency` would
/// report: the same computation `urzua fix` repairs from.
/// The records whose every drifted locator is in a category whose drift
/// policy is `acknowledge`: the renewal reads and attests all of it.
fn acknowledged_only(
    drift: &crate::discovery::DriftedRecords,
    config: &Config,
) -> HashSet<PathBuf> {
    let opts: rules::ClaimExpiredOpts = config.rule_opts(rules::RULE_EMBODIMENT_CLAIM_EXPIRED);
    drift
        .locators
        .iter()
        .filter(|(path, found)| {
            (drift.drifted.contains(*path) || drift.fresh.contains(*path))
                && !found.is_empty()
                && found.iter().all(|l| {
                    opts.drift_policy(&l.category) == urzua_core::config::DriftPolicy::Acknowledge
                })
        })
        .map(|(path, _)| path.clone())
        .collect()
}

fn embodiment_disagreements(
    repo_root: &std::path::Path,
    config: &Config,
    records: &[Record],
    subset: &[Record],
    discovered: &urzua_io::DiscoveredFiles,
    (drifted, fresh): (&HashSet<PathBuf>, &HashSet<PathBuf>),
    back_pointed: &rules::BackPointerEvidence,
) -> HashSet<PathBuf> {
    let blocked = if subset.iter().any(|r| r.items.is_some()) {
        rules::derived_blocked_records(records, config)
    } else {
        Default::default()
    };
    crate::file_texts::with_locator_env(repo_root, config, &discovered.paths, records, |env| {
        let ctx = rules::DeliveryContext {
            back_pointed,
            env,
            blocked: &blocked,
        };
        let (_, repairs) = urzua_core::fix::detect_repairs(subset, config, drifted, fresh, &ctx);
        repairs
            .into_iter()
            .filter(|r| {
                records
                    .iter()
                    .find(|x| x.path == r.record)
                    .is_some_and(|x| {
                        r.field
                            == rules::relation_field_name(
                                config,
                                &x.record_type,
                                urzua_core::config::RelationRole::EmbodimentState,
                            )
                    })
            })
            .map(|r| r.record)
            .collect()
    })
}
