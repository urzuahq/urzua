//! `urzua reverify` (`ADR-97` section 4, `MILE-129`): the one attributed way to
//! renew a claim. Review first (read only), then `--apply`. It never moves an
//! existing pin: a changed pin is `fix --rebaseline`'s. It writes the first pin
//! on a `code:` or `test:` locator that has none, because a claim is not renewed
//! over evidence only git history vouches for; `--pin-only` writes those pins
//! and renews nothing.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use urzua_core::config::{Config, RuleLevel};
use urzua_core::fix::{revision_log_refusal, ActorKind, Attestation, PinRead};
use urzua_core::locator::Locator;
use urzua_core::record::Record;
use urzua_core::report::{
    census, CouldNotRun, FixStatus, Notice, Outcome, Population, PopulationUnit,
};
use urzua_core::reverify::{
    apply_reverify, review_locators, FirstPin, LocatorCheck, LocatorsReview, RecordReview, Renewal,
    ReverifyPlan, ReverifyReport,
};
use urzua_core::rules::{self, ClaimExpiredOpts, ClaimSkip};

use crate::commands::fix::{
    back_pointer_evidence, identity_notices, into_failures, refuse, resolve_attestation,
    rule_enabled, validate_reason, Refusal, ResolvedAttestation,
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
    renewal: Option<(i64, u32)>,
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

    // What the mode lists: with names, those; otherwise what it finds.
    let by_name = !args.ids.is_empty();
    reviewed.retain(|r| {
        if by_name {
            true
        } else if args.pin_only {
            r.review
                .locators
                .iter()
                .any(|l| l.check == LocatorCheck::WillPin)
        } else {
            r.review.finding.is_some()
        }
    });

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
        embodiment_disagreements(
            repo_root,
            config,
            records,
            subset,
            discovered,
            &drift,
            &back_pointed,
        )
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
        notices: drift_notices(&evidence.drift, &evidence.measures.problems),
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
        &evidence.drift,
        &evidence.measures.problems,
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
        locator_refusals(&locators, &mut refused);
        let pins = FirstPin::from_review(&locators.locators);
        self.mode_refusals(record, &pins, &mut refused);
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
            },
            locators,
            pins,
            renewal: standing.renewal,
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
    /// pin; a renewal needs an anchor field and locators that have not drifted.
    fn mode_refusals(&self, record: &Record, pins: &[FirstPin], refused: &mut Vec<String>) {
        if self.pin_only {
            if pins.is_empty() {
                refused.push("no unpinned locator of a required category to pin".to_string());
            }
            return;
        }
        if self.opts.anchor_field.is_none() {
            refused.push(format!(
                "{} declares no anchor_field: there is nowhere to record the date, so the claim cannot be renewed",
                rules::RULE_EMBODIMENT_CLAIM_EXPIRED
            ));
        }
        if self.drift.drifted.contains(&record.path) || self.drift.fresh.contains(&record.path) {
            refused.push("a locator has drifted since the Realized-by line was last touched, even if inside drift_grace_days: re-pin an unpinned one with `urzua reverify --pin-only` (or a pinned one with `urzua fix --rebaseline`), then renew".to_string());
        }
        if self.disagreeing.contains(&record.path) {
            refused.push("the stated Embodiment disagrees with the one its locators compute to (embodiment.consistency would report it): run `urzua fix` first".to_string());
        }
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
fn locator_refusals(locators: &LocatorsReview, refused: &mut Vec<String>) {
    for l in &locators.locators {
        match l.check {
            LocatorCheck::Unresolved => refused.push(format!(
                "locator '{}' does not resolve: {}",
                l.locator,
                l.reason.as_deref().unwrap_or("not found")
            )),
            LocatorCheck::Changed => refused.push(format!(
                "pinned locator '{}' no longer matches its pin: run `urzua fix --rebaseline` first, with its own reason",
                l.locator
            )),
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
                }),
                _ => None,
            }
        };
        let plan = ReverifyPlan {
            review: &item.locators,
            pins: &item.pins,
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

fn apply_report(
    outcome: ApplyOutcome,
    drift: &DriftedRecords,
    problems: &[String],
) -> ReverifyReport {
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
    notices.extend(drift_notices(drift, problems));
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

fn drift_notices(drift: &crate::discovery::DriftedRecords, problems: &[String]) -> Vec<Notice> {
    let mut all = drift.git_problems.clone();
    for p in problems {
        if !all.contains(p) {
            all.push(p.clone());
        }
    }
    crate::discovery::drift_notices(drift.shallow, &all)
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
fn embodiment_disagreements(
    repo_root: &std::path::Path,
    config: &Config,
    records: &[Record],
    subset: &[Record],
    discovered: &urzua_io::DiscoveredFiles,
    drift: &crate::discovery::DriftedRecords,
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
        let (_, repairs) =
            urzua_core::fix::detect_repairs(subset, config, &drift.drifted, &drift.fresh, &ctx);
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
