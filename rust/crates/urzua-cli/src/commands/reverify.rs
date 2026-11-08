//! `urzua reverify` (`ADR-97` section 4, `MILE-129`): the one attributed way to
//! renew a claim. Review first (read only), then `--apply`. It never moves an
//! existing pin: a changed pin is `fix --rebaseline`'s. It writes the first pin
//! on a `code:` or `test:` locator that has none, because a claim is not renewed
//! over evidence only git history vouches for; `--pin-only` writes those pins
//! and renews nothing.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use urzua_core::config::{Config, RuleLevel};
use urzua_core::fix::{revision_log_refusal, ActorKind};
use urzua_core::record::Record;
use urzua_core::report::{census, CouldNotRun, FixStatus, Notice, Outcome, PopulationUnit};
use urzua_core::reverify::{
    apply_reverify, review_locators, FirstPin, LocatorCheck, LocatorsReview, RecordReview, Renewal,
    ReverifyPlan, ReverifyReport,
};
use urzua_core::rules::{self, ClaimExpiredOpts, ClaimSkip};

use crate::commands::fix::{
    back_pointer_evidence, identity_notices, into_failures, resolve_attestation, validate_reason,
    ResolvedAttestation,
};
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
    let ReverifyArgs {
        ids,
        apply,
        pin_only,
        by,
        actor,
        force,
        reason,
    } = args;
    if apply && ids.is_empty() && !force {
        return emit(&CouldNotRun::from(
            "urzua reverify --apply: pass --ids <record,...> or --force (never the default)",
        ));
    }
    if !apply && (force || by.is_some() || reason.is_some() || actor != ActorKind::Unspecified) {
        return emit(&CouldNotRun::from(
            "urzua reverify --force, --by, --actor and --reason belong to --apply: without it nothing is written, so they would be ignored (--ids alone narrows the review)",
        ));
    }
    let reason = match validate_reason(
        apply,
        reason.as_deref(),
        "urzua reverify --apply: pass --reason <what you checked>; a renewal with no reason is a rubber stamp",
    ) {
        Ok(r) => r,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let session = match Session::open(config_path) {
        Ok(s) => s,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let Session {
        repo_root,
        config,
        discovered,
        records,
        ..
    } = session;
    let enabled = |rule: &str| {
        config
            .rules
            .get(rule)
            .is_some_and(|s| s.level != RuleLevel::Off)
    };
    if !enabled(rules::RULE_EMBODIMENT_CLAIM_EXPIRED) {
        return emit(&CouldNotRun::from(
            "urzua reverify renews the claims that embodiment.claim-expired judges, and that rule is off or undeclared in the configuration: declare it (ADR-97) first",
        ));
    }
    let opts: ClaimExpiredOpts = config.rule_opts(rules::RULE_EMBODIMENT_CLAIM_EXPIRED);
    if apply && !force && ids.len() > opts.max_batch {
        return emit(&CouldNotRun::from(format!(
            "urzua reverify --apply names {} records, more than max_batch of {}: name at most {} and run another batch with its own reason",
            ids.len(),
            opts.max_batch,
            opts.max_batch
        )));
    }
    let identity = if apply {
        match resolve_attestation(by.as_deref(), actor, &repo_root, &config) {
            Ok(id) => Some(id),
            Err(e) => return emit(&CouldNotRun::from(e)),
        }
    } else {
        None
    };
    // The pin hashes written come from this review, so the lock is taken first
    // (as `fix --rebaseline` does): evidence edited between a review and the
    // lock would otherwise be hashed stale and attested as current.
    let _lock = if apply {
        match urzua_io::FixLock::acquire(&repo_root) {
            Ok(lock) => Some(lock),
            Err(e) => return emit(&CouldNotRun::from(e)),
        }
    } else {
        None
    };

    let listed: Vec<&Record> = records
        .iter()
        .filter(|r| opts.periods.iter().any(|p| p.record_type == r.record_type))
        .collect();
    let mut failed: Vec<(PathBuf, String)> = Vec::new();
    // `--ids` names the records, with or without `--force` (which lifts only
    // the `max_batch` cap and the need to name one).
    let by_name = !ids.is_empty();
    let subset: Vec<Record> = if by_name {
        for id in &ids {
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
        listed
            .iter()
            .filter(|r| ids.contains(&r.path.display().to_string()))
            .map(|r| (*r).clone())
            .collect()
    } else {
        listed.iter().map(|r| (*r).clone()).collect()
    };

    let now = urzua_io::now_unix();
    let measures = crate::discovery::compute_claim_measures(&repo_root, &subset, &config, now);
    let drift = crate::discovery::compute_drifted_records(&repo_root, &subset, &config);
    let back_pointed = match back_pointer_evidence(&repo_root, &discovered, &config, &records) {
        Ok(evidence) => evidence,
        Err(e) => return emit(&CouldNotRun::from(e)),
    };
    let disagreeing = if enabled(rules::RULE_EMBODIMENT_CONSISTENCY) {
        embodiment_disagreements(
            &repo_root,
            &config,
            &records,
            &subset,
            &discovered,
            &drift,
            &back_pointed,
        )
    } else {
        HashSet::new()
    };

    let texts = crate::file_texts::FileTexts::new(repo_root.clone(), &config);
    let read =
        |l: &urzua_core::locator::Locator| crate::pins::read_pin(&repo_root, &config, &texts, l);
    let mut reviewed: Vec<Reviewed> = Vec::new();
    let population = census(PopulationUnit::Record, subset.iter().collect(), |record| {
        let locators = review_locators(record, &config, &opts.require_pin, &read);
        let tier = rules::computed_tier(record, &config, &back_pointed);
        let standing = rules::claim_standing(
            record,
            &config,
            &opts,
            &tier,
            now,
            measures.line_times.get(&record.path).copied(),
            measures.seeds.get(&record.path).copied(),
        );
        let mut refused: Vec<String> = Vec::new();
        let mut finding = None;
        let mut age = None;
        let mut limit = None;
        let mut renewal = None;
        let mut status = String::new();
        match &standing {
            Ok(s) => {
                status.clone_from(&s.status);
                age = Some(s.age_days);
                limit = Some(s.limit);
                if s.expired() {
                    finding = Some(s.message(&record.record_type));
                }
                renewal = Some((s.age_days, s.days));
                if s.level == RuleLevel::Error && actor != ActorKind::Human {
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
        let pins = FirstPin::from_review(&locators.locators);
        if pin_only {
            if pins.is_empty() {
                refused.push("no unpinned locator of a required category to pin".to_string());
            }
        } else {
            if opts.anchor_field.is_none() {
                refused.push(format!(
                    "{} declares no anchor_field: there is nowhere to record the date, so the claim cannot be renewed",
                    rules::RULE_EMBODIMENT_CLAIM_EXPIRED
                ));
            }
            if drift.drifted.contains(&record.path) || drift.fresh.contains(&record.path) {
                refused.push("a locator has drifted since the Realized-by line was last touched, even if inside drift_grace_days: re-pin an unpinned one with `urzua reverify --pin-only` (or a pinned one with `urzua fix --rebaseline`), then renew".to_string());
            }
            if disagreeing.contains(&record.path) {
                refused.push("the stated Embodiment disagrees with the one its locators compute to (embodiment.consistency would report it): run `urzua fix` first".to_string());
            }
        }
        // Detect never offers a write `--apply` would refuse: both modes append
        // an attested row to the record's revision log.
        let log_id = config.revision_log_id(&record.record_type);
        let log_shape = config.revision_log_shape(&record.record_type);
        match urzua_io::read_to_string(&repo_root.join(&record.path)) {
            Ok(content) => {
                refused.extend(revision_log_refusal(&content, &log_id, &log_shape, "write"))
            }
            Err(e) => refused.push(format!("could not read the record: {e}")),
        }
        let mut commits_since = Vec::new();
        if pin_only && !pins.is_empty() {
            commits_since = commits_since_line(&repo_root, record, &locators, &pins, &config);
        }
        reviewed.push(Reviewed {
            review: RecordReview {
                record: record.path.clone(),
                status,
                tier,
                finding,
                age_days: age,
                period_days: limit,
                locators: locators.locators.clone(),
                commits_since,
                refused,
            },
            locators,
            pins,
            renewal,
        });
        Outcome::Examined
    });

    // What the mode lists: with names, those; otherwise what it finds.
    let select = |r: &Reviewed| {
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
    };
    reviewed.retain(select);

    let Some(identity) = identity else {
        let status = if !failed.is_empty() {
            FixStatus::PartialFailure
        } else if reviewed.iter().any(|r| r.review.refused.is_empty()) {
            FixStatus::RepairsAvailable
        } else {
            FixStatus::Ok
        };
        return emit(&ReverifyReport {
            as_of: crate::as_of(),
            status,
            mode: "detect",
            pin_only,
            population,
            reason: None,
            records: reviewed.into_iter().map(|r| r.review).collect(),
            failed: into_failures(failed),
            identity: None,
            notices: drift_notices(&drift, &measures.problems),
        });
    };

    if force && reviewed.len() > 1 {
        return emit(&CouldNotRun::from(format!(
            "urzua reverify --force is refused when it selects more than one record ({} selected): it lifts max_batch for a single record and never batches",
            reviewed.len()
        )));
    }
    let today = urzua_io::today();
    let mut written: Vec<RecordReview> = Vec::new();
    for item in reviewed {
        let Reviewed {
            review,
            locators,
            pins,
            renewal,
        } = item;
        if !review.refused.is_empty() {
            failed.push((
                review.record.clone(),
                format!("refused: {}", review.refused.join("; ")),
            ));
            continue;
        }
        let full = repo_root.join(&review.record);
        let fresh = match urzua_io::read_to_string(&full) {
            Ok(c) => c,
            Err(e) => {
                failed.push((review.record.clone(), format!("could not re-read: {e}")));
                continue;
            }
        };
        let record_type = records
            .iter()
            .find(|r| r.path == review.record)
            .map(|r| r.record_type.as_str());
        let log_id = record_type
            .map(|t| config.revision_log_id(t))
            .unwrap_or_default();
        let log_shape = record_type
            .map(|t| config.revision_log_shape(t))
            .unwrap_or_default();
        let renewal = if pin_only {
            None
        } else {
            match (opts.anchor_field.as_deref(), renewal) {
                (Some(anchor_field), Some((age_days, days))) => Some(Renewal {
                    anchor_field,
                    age_days,
                    days,
                    require_pin_empty: opts.require_pin.is_empty(),
                }),
                _ => None,
            }
        };
        let plan = ReverifyPlan {
            review: &locators,
            pins: &pins,
            renewal,
        };
        match apply_reverify(
            &fresh,
            &plan,
            &today,
            &identity.attestation,
            &reason,
            &log_id,
            &log_shape,
        ) {
            Ok(new) => match urzua_io::write_atomic_in(&repo_root, &full, &new) {
                Ok(()) => written.push(review),
                Err(e) => failed.push((review.record.clone(), format!("could not write: {e}"))),
            },
            Err(e) => failed.push((review.record.clone(), e)),
        }
    }
    emit(&apply_report(
        written,
        failed,
        population,
        pin_only,
        reason,
        identity,
        &drift,
        &measures.problems,
    ))
}

#[allow(clippy::too_many_arguments)]
fn apply_report(
    written: Vec<RecordReview>,
    failed: Vec<(PathBuf, String)>,
    population: urzua_core::report::Population,
    pin_only: bool,
    reason: String,
    identity: ResolvedAttestation,
    drift: &crate::discovery::DriftedRecords,
    problems: &[String],
) -> ReverifyReport {
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
