//! `urzua fix` detect mode (ADR-0015 §3): read-only, emits one entry per
//! repairable fact. Apply mode (`--ids`, `--by`, `--force`) is not built
//! yet -- it needs real file mutation, identity resolution, and a
//! revision-log write-path, each its own piece of work. Detect mode alone
//! is "a real Phase 1 feature" per ADR-0015, not a stub.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::{Config, RelationRole};
use crate::line_edit::{table_row_prefix, LineDoc};
use crate::record::Record;
use crate::report::{census, Outcome, Population, PopulationUnit};
use crate::rules::{
    compute_embodiment, evidence_text, find_revision_log_entries, parse_realized_by,
    relation_field_name, DeliveryContext,
};

/// One mechanically-detected disagreement between a record's stated value
/// and what the tool computes -- the same shape RFC-0008 §3 specifies for
/// `urzua fix`'s detect-mode output.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Repair {
    pub record: std::path::PathBuf,
    pub field: String,
    /// The 1-based line of the stated field in the content examined (the file,
    /// or the proposed content under `--stdin-path`), so an editor hook can
    /// place the repair without parsing the record (`MILE-159`).
    pub line: usize,
    pub current_value: String,
    pub computed_value: String,
    /// RFC-0008 §2's repair tiers. Only Tier 1 (recompute in place) exists
    /// in this MVP.
    pub tier: u8,
    /// The evidence the computation read, named in the record it produces
    /// (RFC-0008 §6) -- a reader can disagree with the derivation, not just
    /// trust it.
    pub evidence: String,
}

/// Tier 1 repairs for `Embodiment`: the only tool-writable field this MVP
/// knows about (ADR-0018). Every other field stays permanently
/// hand-authored (RFC-0008 §1's eligibility test).
///
/// The candidate is the *pair*: a repair needs both `Embodiment` and
/// `Realized-by`, so a record carrying only one of them is handed to this
/// function and reaches no verdict -- `Absent`, not a candidate that never
/// existed. Every loaded record is eligible; `fix` has no config-declared
/// field set to filter against the way `check`'s rules do, so unlike
/// `embodiment_consistency` this reads every record rather than a declared
/// slot.
///
/// Returns the population alongside the repairs found, so a caller can
/// distinguish "examined some, found nothing to repair" from "examined
/// nothing" -- the same no-silent-no-op discipline `check` follows.
pub fn detect_repairs(
    records: &[Record],
    config: &Config,
    drifted: &HashSet<PathBuf>,
    fresh_drift: &HashSet<PathBuf>,
    ctx: &DeliveryContext<'_>,
) -> (Population, Vec<Repair>) {
    let back_pointed = ctx.back_pointed;
    let mut repairs = Vec::new();
    let sticky = crate::rules::sticky_states(config);

    let embodiment_outcome = |record: &Record, repairs: &mut Vec<Repair>| -> Outcome {
        let state_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentState);
        let locator_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
        let Some(stated) = record.header.get(state_field) else {
            return Outcome::Absent;
        };
        // A sticky state is a human's; there is nothing to recompute (`MILE-148`).
        if sticky.contains(stated.trim()) {
            return Outcome::Examined;
        }
        let cited_by_code = back_pointed.code.contains(&record.path);
        let cited_by_test = back_pointed.test.contains(&record.path);
        // No `Realized-by` is repaired only when a back-pointer cites the
        // record, as `embodiment.consistency` judges it (`MILE-149`).
        // The union of `Realized-by` and the Done-means item clauses
        // (`ADR-96` section 5).
        let realized_by_text = evidence_text(record, locator_field);
        let realized_by_value = realized_by_text.as_deref();
        if realized_by_value.is_none() && !(cited_by_code || cited_by_test) {
            return Outcome::Absent;
        }

        // Drift (ADR-32) is computed by the caller from git history and
        // handed in as plain data, the same shape `embodiment_consistency`
        // takes, so this function stays pure (MILE-15).
        let computed = compute_embodiment(
            &config.embodiment,
            &realized_by_value.map(parse_realized_by).unwrap_or_default(),
            drifted.contains(&record.path),
            cited_by_code,
            cited_by_test,
        );
        let stated = stated.trim();
        // Fresh drift (`drift_grace_days`) never flips a stated `Drift detected`.
        let keeps_stated_drift =
            fresh_drift.contains(&record.path) && config.embodiment.is_drift(stated);
        if stated != computed && !keeps_stated_drift {
            repairs.push(Repair {
                record: record.path.clone(),
                field: state_field.to_string(),
                line: record
                    .header
                    .fields
                    .iter()
                    .find(|f| f.key.as_str() == state_field)
                    .map_or(0, |f| f.line),
                current_value: stated.to_string(),
                computed_value: computed.to_string(),
                tier: 1,
                evidence: match realized_by_value {
                    Some(value) => format!("{locator_field}: {value}"),
                    None => format!(
                        "no {locator_field} field; cited by {} back-pointers",
                        if cited_by_test { "test" } else { "source" }
                    ),
                },
            });
        }
        Outcome::Examined
    };

    let delivery_types = crate::rules::statuses_by_type(config);
    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        let embodiment = embodiment_outcome(record, &mut repairs);
        // `Status` against delivery evidence (`MILE-183`): the one case with a
        // single correct value is repaired; the reverse is `unevidenced_successes`.
        let status = match delivery_types.get(&record.record_type) {
            Some(decl) if decl.delivery.is_some() => {
                match crate::rules::judge_delivery(record, config, decl, ctx) {
                    Ok(judged) => {
                        // `fix` never downgrades a `success` status
                        // (`DoneWithUnresolvedItems` is reported, not written),
                        // and writes `partial_status` only to a record that is
                        // not derived-blocked (`ADR-96` sections 2 and 7).
                        let repaired = match judged.disagreement.clone() {
                            Some(crate::rules::DeliveryDisagreement::DeliveredButNotDone {
                                set_status,
                            }) => Some(set_status),
                            Some(crate::rules::DeliveryDisagreement::PartialButNotStarted {
                                partial_status,
                                blocked: false,
                            }) => Some(partial_status),
                            _ => None,
                        };
                        if let Some(set_status) = repaired {
                            repairs.push(Repair {
                                record: record.path.clone(),
                                field: judged.status_field.clone(),
                                line: record
                                    .header
                                    .fields
                                    .iter()
                                    .find(|f| f.key.as_str() == judged.status_field)
                                    .map_or(0, |f| f.line),
                                current_value: judged.status.clone(),
                                computed_value: set_status,
                                tier: 1,
                                evidence: format!(
                                    "{} computes to {}",
                                    judged.evidence, judged.tier
                                ),
                            });
                        }
                        Outcome::Examined
                    }
                    Err(outcome) => outcome,
                }
            }
            _ => Outcome::Absent,
        };
        if embodiment == Outcome::Examined || status == Outcome::Examined {
            Outcome::Examined
        } else if embodiment != Outcome::Absent {
            embodiment
        } else {
            status
        }
    });

    (population, repairs)
}

/// Records whose success `Status` has no delivery evidence (`MILE-183`). `fix`
/// does not downgrade them -- the repair is adding evidence, which only a person
/// or the delivering change can supply -- so they are reported, never written.
pub fn unevidenced_successes(
    records: &[Record],
    config: &Config,
    ctx: &DeliveryContext<'_>,
) -> Vec<crate::report::Notice> {
    let declared = crate::rules::statuses_by_type(config);
    records
        .iter()
        .filter_map(|record| {
            let decl = declared.get(&record.record_type)?;
            let judged = crate::rules::judge_delivery(record, config, decl, ctx).ok()?;
            // A record whose items all resolve while a dependency is unfinished
            // computes to `partial_status` (`ADR-96`, amendment of 2026-11-07):
            // the notice names the dependency that caps it, and a finished
            // status above the cap is not downgraded.
            if let (None, Some(items)) = (&judged.disagreement, &judged.items) {
                if !items.capped_by.is_empty() {
                    let capped_at = decl
                        .delivery
                        .as_ref()
                        .and_then(|d| d.items.as_ref())
                        .map_or("", |i| i.partial_status.as_str());
                    let finished = decl.is_success(&judged.status);
                    let message = if finished {
                        format!(
                            "{} is '{}' but every item resolves and {} is unfinished, so the status computes to '{capped_at}'; not downgraded -- finish the dependency, or reopen it to '{capped_at}' by hand",
                            judged.status_field,
                            judged.status,
                            items.capped_by.join(", ")
                        )
                    } else {
                        format!(
                            "{} '{}' agrees with the items: every item resolves, but {} is unfinished, which caps the status at '{capped_at}'",
                            judged.status_field,
                            judged.status,
                            items.capped_by.join(", ")
                        )
                    };
                    return Some(crate::report::Notice {
                        severity: if finished {
                            crate::report::NoticeSeverity::Warning
                        } else {
                            crate::report::NoticeSeverity::Info
                        },
                        subject: record.path.display().to_string(),
                        message,
                        detail: None,
                    });
                }
            }
            let message = match &judged.disagreement {
                Some(crate::rules::DeliveryDisagreement::SuccessWithoutEvidence) => format!(
                    "{} is '{}' with no delivery evidence (computes to '{}'); not downgraded -- add the evidence that delivered it",
                    judged.status_field, judged.status, judged.tier
                ),
                // `ADR-96`: `fix` never reopens a finished record; it names
                // what keeps the status from being right.
                Some(crate::rules::DeliveryDisagreement::DoneWithUnresolvedItems {
                    unresolved,
                    reopen_to,
                }) => format!(
                    "{} is '{}' but {} item(s) are unresolved ({}); not downgraded -- add the evidence, waive the item, or reopen it to '{reopen_to}' by hand",
                    judged.status_field,
                    judged.status,
                    unresolved.len(),
                    unresolved
                        .iter()
                        .map(|u| format!("item {} \"{}\": {}", u.n, u.text, u.cause))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                _ => return None,
            };
            Some(crate::report::Notice {
                severity: crate::report::NoticeSeverity::Warning,
                subject: record.path.display().to_string(),
                message,
                detail: None,
            })
        })
        .collect()
}

/// How a stated value reads in prose: the value in code spans, or "no value"
/// when the field was empty, `null` or `~`.
fn describe_stated(value: &str) -> String {
    if value.is_empty() {
        "no value".to_string()
    } else {
        format!("`{value}`")
    }
}

/// A YAML value that states nothing: empty, `~` or `null` in any case.
fn is_empty_yaml_value(value: &str) -> bool {
    let v = value.trim();
    v.is_empty() || v == "~" || v.eq_ignore_ascii_case("null")
}

/// Replaces the value after `field:` on `line`, keeping everything before the
/// value (a `> ` prefix, the key and the spacing) byte for byte. `None` when the
/// line is not that field's own line or its value is not the stated one. A
/// stated value of `""` matches an empty, `~` or `null` value.
fn replace_field_value(line: &str, field: &str, stated: &str, computed: &str) -> Option<String> {
    let body = line.trim_start_matches(['>', ' ', '\t']);
    let rest = body.strip_prefix(field)?.strip_prefix(':')?;
    let matches = if stated.is_empty() {
        is_empty_yaml_value(rest)
    } else {
        rest.trim() == stated
    };
    if !matches {
        return None;
    }
    let value_start = line.len() - rest.len();
    let gap: &str = &rest[..rest.len() - rest.trim_start().len()];
    let gap = if gap.is_empty() { " " } else { gap };
    Some(format!("{}{gap}{computed}", &line[..value_start]))
}

/// Where an attestation's identity came from (`ADR-89` section 5): a verified
/// `gh` login, a `--by` taken as given, or `git config user.name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentitySource {
    GhVerified,
    Asserted,
    GitConfig,
}

impl IdentitySource {
    pub fn as_str(self) -> &'static str {
        match self {
            IdentitySource::GhVerified => "gh-verified",
            IdentitySource::Asserted => "asserted",
            IdentitySource::GitConfig => "git-config",
        }
    }
}

/// Whether an agent or a person ran the command (`ADR-89` section 5). It is
/// the caller's claim and not proof, with the same standing as `--by`; a run
/// that does not say is `unspecified`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorKind {
    Agent,
    Human,
    #[default]
    Unspecified,
}

impl ActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActorKind::Agent => "agent",
            ActorKind::Human => "human",
            ActorKind::Unspecified => "unspecified",
        }
    }
}

/// Who a tool-authored revision row names, and on what footing (`ADR-89`
/// section 5): the name, where it came from, the caller's claim of who held
/// the command, and the verified login recorded beside an `asserted` name when
/// `gh` answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation {
    pub by: String,
    pub source: IdentitySource,
    pub actor: ActorKind,
    pub verified_login: Option<String>,
}

impl Attestation {
    pub fn new(by: &str, source: IdentitySource, actor: ActorKind) -> Self {
        Attestation {
            by: by.to_string(),
            source,
            actor,
            verified_login: None,
        }
    }

    /// The text inside a row's parentheses after `by` or `attested by`: the
    /// name, the identity source, the actor kind and, beside an `asserted`
    /// name, the login `gh` verified (`ADR-89` section 5).
    pub fn label(&self) -> String {
        let mut label = format!(
            "{}, identity: {}, actor: {}",
            self.by,
            self.source.as_str(),
            self.actor.as_str()
        );
        if let Some(login) = &self.verified_login {
            label.push_str(&format!(", verified login: {login}"));
        }
        label
    }
}

/// Computes the new file content for applying one Tier-1 repair. Pure --
/// returns the new text; the caller performs the actual write, the identity
/// resolution, and the re-verification-before-write RFC-0008 §4 requires.
///
/// When the type requires a revision log (the default), refuses rather than
/// silently skipping the requirement if the record has no revision-log section
/// (its type's declared identity) to append to: ADR-0015's reversibility clause
/// depends on that entry existing. A type declared `revision_log: {required:
/// false}` (`MILE-159`) takes the field edit alone, with no row.
pub fn apply_repair(
    content: &str,
    repair: &Repair,
    today: &str,
    attestation: &Attestation,
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
    log_required: bool,
) -> Result<String, String> {
    let mut doc = LineDoc::parse(content);

    // Edit by field position: the value after `Field:` on the field's own line,
    // never a substring match (an empty or null stated value is a substring of
    // everything and would be inserted at column 0).
    let (field_line, edited_line) = (1..)
        .map_while(|n| doc.line(n).map(|l| (n, l)))
        .find_map(|(n, line)| {
            replace_field_value(line, &repair.field, &repair.current_value, &repair.computed_value)
                .map(|edited| (n, edited))
        })
        .ok_or_else(|| {
            format!(
                "could not find a '{}:' line stating {} to edit -- the record may have changed since detect ran",
                repair.field,
                describe_stated(&repair.current_value)
            )
        })?;
    doc.replace_line(field_line, edited_line);

    if !log_required {
        return Ok(doc.to_string());
    }
    let edited_content = doc.to_string();
    let log = find_revision_log_entries(&edited_content, log_id, shape).ok_or_else(|| {
        format!(
            "record has no Revision log section (looked for {}) -- refusing to apply without one to record the change in (ADR-0015's reversibility requirement); declare revision_log: {{required: false}} on the type to apply without a log",
            log_id.describe()
        )
    })?;
    let log = log.map_err(|problem| problem.message)?;
    let Some(last_entry) = log.entries.last() else {
        return Err(
            "record's Revision log section has no existing rows to append after".to_string(),
        );
    };

    // The new row repeats the existing last row's own prefix, so it lands
    // inside the table whether the log is a blockquote table or a plain one.
    let prefix = doc
        .line(last_entry.line)
        .and_then(table_row_prefix)
        .unwrap_or("> ")
        .to_string();
    let narrative = format!(
        "Tool-authored (by {}): recomputed `{}` from `{}` -- stated {} disagreed with the computed value `{}`.",
        attestation.label(),
        repair.field,
        repair.evidence,
        describe_stated(&repair.current_value),
        repair.computed_value
    );
    let new_row = log_row(&prefix, log.layout, shape, today, &narrative)?;
    let last_line = last_entry.line;
    doc.insert_after(last_line, new_row);

    Ok(doc.to_string())
}

/// The lines a scoped pin resolved to, shown to the reviewer so they diff
/// exactly those lines and see that the resolver found the right item
/// (`ADR-74` point 7).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// What reading a pinned locator now gives (`ADR-74` point 9): the hash of
/// what it pins, with the span for a scoped pin; a scope that does not resolve
/// and why; or nothing, when the file cannot be read (a missing file is
/// `embodiment.locator-exists`' finding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinRead {
    /// `files` is the files a directory or glob pin covers (`BUG-176`); empty
    /// for a file or a scope.
    Current {
        hash: String,
        span: Option<PinSpan>,
        files: Vec<String>,
    },
    Failed(String),
    Unreadable,
}

/// One pinned locator whose current content no longer matches its pin
/// (`MILE-181`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PinDrift {
    pub locator: String,
    pub recorded_hash: String,
    pub current_hash: String,
    /// The resolved span of a scoped pin; absent for a file-level one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<PinSpan>,
    /// The files a directory or glob pin covers now, for the reviewer; absent
    /// for a file or a scope.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    /// The locator as written before its pin, which the re-pin edits in place.
    #[serde(skip)]
    pub spelling: String,
}

/// A scoped pin that does not resolve, so cannot be re-pinned: re-hashing
/// "whatever is at the range now" would turn a deleted symbol into a silent
/// pass. Per pin, not per record; a rename is repaired by editing the scope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FailedPin {
    pub record: PathBuf,
    pub locator: String,
    pub reason: String,
}

/// A record whose `Realized-by` carries one or more stale `@sha256:` pins: the
/// unit `urzua fix --rebaseline` lists and, on review, re-pins (`MILE-181`).
/// A record is the selector, as in `--ids`; all its stale pins move together
/// and one attributed revision row records them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rebaseline {
    pub record: PathBuf,
    pub field: String,
    /// The 1-based line of the evidence field; `0` when the record has none and
    /// its pins sit only in Done-means item clauses (`ADR-96`).
    pub line: usize,
    pub pins: Vec<PinDrift>,
    /// The lines of the Done-means item clauses a stale pin sits on.
    #[serde(skip)]
    pub item_lines: Vec<usize>,
    /// The type's declared item section name, for messages (`ADR-96`).
    #[serde(skip)]
    pub items_section: String,
}

// Implements: ADR-67 (attributed re-baselining of a pin)
// Implements: ADR-74 (a scoped pin is re-pinned from its resolved span, and refused when it does not resolve)
/// Lists the records whose pinned locators changed. Pure: `read` is the CLI's
/// lookup of what a pinned locator hashes to now. A locator whose file cannot
/// be read is not a stale pin (`embodiment.locator-exists` reports it), an
/// unpinned locator has nothing to re-pin, and a scoped pin that no longer
/// resolves is listed as a failed pin with its reason, never re-pinned.
pub fn detect_rebaselines(
    records: &[Record],
    config: &Config,
    read: &dyn Fn(&crate::locator::Locator) -> PinRead,
) -> (Population, Vec<Rebaseline>, Vec<FailedPin>) {
    let mut found = Vec::new();
    let mut failed = Vec::new();
    let population = census(PopulationUnit::Record, records.iter().collect(), |record| {
        let locator_field =
            relation_field_name(config, &record.record_type, RelationRole::EmbodimentLocator);
        // `Realized-by` plus the Done-means item clauses (`ADR-96`): a pin in an
        // item clause is re-pinned as one in the header is.
        let Some(value) = evidence_text(record, locator_field) else {
            return Outcome::Absent;
        };
        let mut pins: Vec<PinDrift> = Vec::new();
        for entry in crate::locator::realized_by_locators(&value) {
            let Ok(locator) = entry.parsed else { continue };
            let Some(crate::locator::Anchor::Pin(recorded)) = locator.anchor.clone() else {
                continue;
            };
            match read(&locator) {
                // A scoped pin on a file that cannot be read cannot be
                // re-pinned either: list it rather than skip it silently.
                PinRead::Unreadable if locator.scope.is_some() => {
                    let reason = format!("file not found or unreadable: {}", locator.path);
                    note_failed_pin(&mut failed, &record.path, locator.display(), reason);
                }
                PinRead::Unreadable => {}
                PinRead::Failed(reason) => {
                    note_failed_pin(&mut failed, &record.path, locator.display(), reason);
                }
                PinRead::Current { hash, span, files } => {
                    if hash != recorded
                        && !pins
                            .iter()
                            .any(|p| p.spelling == locator.spelling && p.recorded_hash == recorded)
                    {
                        pins.push(PinDrift {
                            locator: locator.display(),
                            recorded_hash: recorded,
                            current_hash: hash,
                            span,
                            files,
                            spelling: locator.spelling.clone(),
                        });
                    }
                }
            }
        }
        if !pins.is_empty() {
            let item_lines: Vec<usize> = record
                .items
                .iter()
                .flat_map(|i| i.items.iter())
                .filter_map(|i| i.evidence.as_ref())
                .filter(|c| c.valid)
                .filter(|c| {
                    pins.iter().any(|p| {
                        c.text
                            .contains(&format!("{}@sha256:{}", p.spelling, p.recorded_hash))
                    })
                })
                .map(|c| c.line)
                .collect();
            found.push(Rebaseline {
                item_lines,
                items_section: crate::rules::items_section(config, &record.record_type),
                record: record.path.clone(),
                field: locator_field.to_string(),
                line: record
                    .header
                    .fields
                    .iter()
                    .find(|f| f.key.as_str() == locator_field)
                    .map_or(0, |f| f.line),
                pins,
            });
        }
        Outcome::Examined
    });
    (population, found, failed)
}

/// Lists a pin that could not be read once per record and locator.
fn note_failed_pin(failed: &mut Vec<FailedPin>, record: &Path, locator: String, reason: String) {
    if !failed
        .iter()
        .any(|f: &FailedPin| f.record == record && f.locator == locator)
    {
        failed.push(FailedPin {
            record: record.to_path_buf(),
            locator,
            reason,
        });
    }
}

/// `line` with every whole-token occurrence of `old` (`spelling@sha256:hash`)
/// replaced by `new`, or `None` when there is none. A match must start at a
/// token boundary, so the pin on `a.rs` does not rewrite the pin on `lib/a.rs`
/// that carries the same hash, and must not run into a longer hash.
fn replace_exact_pin(line: &str, old: &str, new: &str) -> Option<String> {
    let path_char = |c: char| c.is_alphanumeric() || "_./-~@+%".contains(c);
    let mut out = String::with_capacity(line.len());
    let mut rest = 0;
    let mut found = false;
    for (at, _) in line.match_indices(old) {
        if at < rest {
            continue;
        }
        let before_ok = line[..at].chars().next_back().is_none_or(|c| !path_char(c));
        let after_ok = line[at + old.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_hexdigit());
        if before_ok && after_ok {
            out.push_str(&line[rest..at]);
            out.push_str(new);
            rest = at + old.len();
            found = true;
        }
    }
    out.push_str(&line[rest..]);
    found.then_some(out)
}

// Implements: ADR-67 (attributed re-baselining of a pin)
/// New file content for re-pinning one record: each stale pin on the
/// `Realized-by` line becomes the current hash and nothing else on the line
/// moves, then one revision-log row names who attested and why. Pure. Fails
/// when a pin is no longer on the line (a concurrent edit since detect) or the
/// record has no revision log to record the attestation in.
pub fn apply_rebaseline(
    content: &str,
    rebaseline: &Rebaseline,
    today: &str,
    attestation: &Attestation,
    reason: &str,
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
) -> Result<String, String> {
    let mut doc = LineDoc::parse(content);
    let lines = evidence_edit_lines(
        &doc,
        &rebaseline.field,
        rebaseline.line,
        &rebaseline.item_lines,
        "detect ran",
    )?;
    for pin in &rebaseline.pins {
        let old = format!("{}@sha256:{}", pin.spelling, pin.recorded_hash);
        let new = format!("{}@sha256:{}", pin.spelling, pin.current_hash);
        let mut replaced_anywhere = false;
        for &at in &lines {
            let line = doc.line(at).unwrap_or_default().to_string();
            if let Some(replaced) = replace_exact_pin(&line, &old, &new) {
                doc.replace_line(at, replaced);
                replaced_anywhere = true;
            }
        }
        if !replaced_anywhere {
            return Err(format!(
                "the pin on '{}' is no longer on the '{}' line or in a {} item clause -- the record may have changed since detect ran",
                pin.locator, rebaseline.field, rebaseline.items_section
            ));
        }
    }

    let moved = rebaseline
        .pins
        .iter()
        .map(|p| {
            let lines = p
                .span
                .as_ref()
                .map(|s| format!(" (lines {}-{})", s.start, s.end))
                .unwrap_or_default();
            format!(
                "`{}`{lines} {} -> {}",
                p.locator,
                short(&p.recorded_hash),
                short(&p.current_hash)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let narrative = format!(
        "Re-baselined (attested by {}): re-pinned {moved}. Reason: {reason} **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds.",
        attestation.label()
    );
    append_log_rows(&mut doc, log_id, shape, today, &[narrative], "re-baseline")?;
    Ok(doc.to_string())
}

/// The 1-based lines a pin edit may land on: the evidence field's line when the
/// record has one (or has no item clauses to hold the evidence instead), then
/// the Done-means item clause lines (`ADR-96`). `changed_since` finishes the
/// "record may have changed since ..." refusal. Shared by `apply_rebaseline`
/// and `reverify::apply_reverify`.
pub(crate) fn evidence_edit_lines(
    doc: &LineDoc,
    field: &str,
    field_line: usize,
    item_lines: &[usize],
    changed_since: &str,
) -> Result<Vec<usize>, String> {
    let mut lines: Vec<usize> = Vec::new();
    if field_line != 0 || item_lines.is_empty() {
        let key = format!("{field}:");
        let at = doc
            .find_line(|l| l.trim_start().starts_with(&key))
            .ok_or_else(|| {
                format!(
                    "could not find the '{field}' line to edit -- the record may have changed since {changed_since}"
                )
            })?;
        lines.push(at);
    }
    lines.extend(item_lines);
    Ok(lines)
}

/// Why `content` has no revision log a tool row can be appended to, or `None`
/// when it has one. `verb` finishes "refusing to ... without one to record the
/// attestation in". `urzua reverify`'s detect asks this too, so it never offers
/// a write that `--apply` would refuse.
pub fn revision_log_refusal(
    content: &str,
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
    verb: &str,
) -> Option<String> {
    locate_revision_log(content, log_id, shape, verb).err()
}

fn locate_revision_log(
    content: &str,
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
    verb: &str,
) -> Result<crate::rules::RevisionLog, String> {
    let log = find_revision_log_entries(content, log_id, shape).ok_or_else(|| {
        format!("record has no Revision log section -- refusing to {verb} without one to record the attestation in")
    })?;
    let log = log.map_err(|problem| problem.message)?;
    if log.entries.is_empty() {
        return Err(
            "record's Revision log section has no existing rows to append after".to_string(),
        );
    }
    Ok(log)
}

/// Appends one tool-authored row per narrative after the revision log's last
/// row, in the log's own layout. Fails, changing nothing, when the log is
/// missing, unreadable or has no row to append after.
pub(crate) fn append_log_rows(
    doc: &mut LineDoc,
    log_id: &crate::config::RevisionLogId,
    shape: &crate::config::RevisionLogShape,
    today: &str,
    narratives: &[String],
    verb: &str,
) -> Result<(), String> {
    let log = locate_revision_log(&doc.to_string(), log_id, shape, verb)?;
    let last_line = log.entries.last().map(|e| e.line).unwrap_or_default();
    let prefix = doc
        .line(last_line)
        .and_then(table_row_prefix)
        .unwrap_or("> ")
        .to_string();
    let mut rows = Vec::new();
    for narrative in narratives {
        rows.push(log_row(&prefix, log.layout, shape, today, narrative)?);
    }
    let mut after = last_line;
    for row in rows {
        doc.insert_after(after, row);
        after += 1;
    }
    Ok(())
}

/// A tool-authored revision-log row, laid out by the log's own header
/// (`ADR-94`): the date in the date column, the type's `tool_class` in the
/// class column, the narrative in the first remaining column, any other cell
/// empty. A log with no such column left is refused, as one with no rows to
/// append after is.
pub(crate) fn log_row(
    prefix: &str,
    layout: crate::rules::LogLayout,
    shape: &crate::config::RevisionLogShape,
    today: &str,
    narrative: &str,
) -> Result<String, String> {
    if layout.date == layout.class {
        return Err(
            "record's Revision log has its date and class in the same column -- refusing to write a row there".to_string(),
        );
    }
    let Some(narrative_at) = (0..layout.width).find(|i| *i != layout.date && *i != layout.class)
    else {
        return Err(
            "record's Revision log table has no column left for the narrative besides the date and class columns -- refusing to write a row there".to_string(),
        );
    };
    let mut cells = vec![String::new(); layout.width];
    cells[layout.date] = today.to_string();
    cells[layout.class] = format!("**{}**", shape.tool_class);
    cells[narrative_at] = narrative.to_string();
    Ok(format!("{prefix}| {} |", cells.join(" | ")))
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// `detect_repairs` over a tree with no files, no blocked record and the
    /// given back-pointers.
    fn detect(
        records: &[Record],
        config: &Config,
        drifted: &HashSet<PathBuf>,
        fresh: &HashSet<PathBuf>,
        back_pointed: &crate::rules::BackPointerEvidence,
    ) -> (Population, Vec<Repair>) {
        crate::rules::LocatorEnv::without_files(config, |env| {
            let blocked = std::collections::HashMap::new();
            let ctx = DeliveryContext {
                back_pointed,
                env,
                blocked: &blocked,
            };
            detect_repairs(records, config, drifted, fresh, &ctx)
        })
    }

    fn record(path: &str, content: &str) -> Record {
        Record::parse(PathBuf::from(path), "adr".to_string(), content)
    }

    fn attest(name: &str) -> Attestation {
        Attestation::new(name, IdentitySource::GhVerified, ActorKind::Unspecified)
    }

    /// `ADR-89` section 5 pins the row text: the identity source and the actor
    /// kind follow the name, in both tool-authored row kinds.
    #[test]
    fn the_attestation_label_names_the_source_and_the_actor_kind() {
        let verified = Attestation::new("ana", IdentitySource::GhVerified, ActorKind::Agent);
        assert_eq!(verified.label(), "ana, identity: gh-verified, actor: agent");
        let git = Attestation::new("ana", IdentitySource::GitConfig, ActorKind::Human);
        assert_eq!(git.label(), "ana, identity: git-config, actor: human");
        let mut asserted =
            Attestation::new("ana", IdentitySource::Asserted, ActorKind::Unspecified);
        assert_eq!(
            asserted.label(),
            "ana, identity: asserted, actor: unspecified"
        );
        asserted.verified_login = Some("octo".to_string());
        assert_eq!(
            asserted.label(),
            "ana, identity: asserted, actor: unspecified, verified login: octo"
        );
    }

    #[test]
    fn a_rebaseline_row_carries_the_source_and_the_actor_kind() {
        let content = "---\nEmbodiment: Implemented\nRealized-by: 'code:src/a.rs@sha256:aaaaaaaaaaaaaaaaaaaaaaaa'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n";
        let rb = Rebaseline {
            item_lines: Vec::new(),
            items_section: String::new(),
            record: PathBuf::from("docs/adr/ADR-1-a.md"),
            field: "Realized-by".to_string(),
            line: 3,
            pins: vec![PinDrift {
                locator: "code:src/a.rs".to_string(),
                recorded_hash: "aaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                current_hash: "bbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                span: None,
                files: Vec::new(),
                spelling: "code:src/a.rs".to_string(),
            }],
        };
        let who = Attestation::new("ana", IdentitySource::Asserted, ActorKind::Agent);
        let out = apply_rebaseline(
            content,
            &rb,
            "2026-11-06",
            &who,
            "Reason.",
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert!(
            out.contains(
                "| 2026-11-06 | Re-baselined (attested by ana, identity: asserted, actor: agent): re-pinned "
            ),
            "{out}"
        );
        // The existing row is untouched.
        assert!(out.contains("| 2026-01-01 | Initial. | **structural** |"));
    }

    #[test]
    fn a_rebaseline_rewrites_the_exact_pin_and_not_one_whose_spelling_ends_with_it() {
        let h = "aaaaaaaaaaaaaaaaaaaaaaaa";
        let content = format!("---\nRealized-by: 'lib/a.rs@sha256:{h}, a.rs@sha256:{h}'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n");
        let rb = Rebaseline {
            item_lines: Vec::new(),
            items_section: String::new(),
            record: PathBuf::from("docs/adr/ADR-1-a.md"),
            field: "Realized-by".to_string(),
            line: 2,
            pins: vec![PinDrift {
                locator: "a.rs".to_string(),
                recorded_hash: h.to_string(),
                current_hash: "bbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                span: None,
                files: Vec::new(),
                spelling: "a.rs".to_string(),
            }],
        };
        let who = Attestation::new("ana", IdentitySource::Asserted, ActorKind::Agent);
        let out = apply_rebaseline(
            &content,
            &rb,
            "2026-11-06",
            &who,
            "R.",
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert!(
            out.contains(&format!(
                "lib/a.rs@sha256:{h}, a.rs@sha256:bbbbbbbbbbbbbbbbbbbbbbbb"
            )),
            "{out}"
        );
    }

    #[test]
    fn a_rebaseline_refusal_names_the_declared_item_section() {
        let h = "aaaaaaaaaaaaaaaaaaaaaaaa";
        let content = "---\nRealized-by: 'src/b.rs'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n";
        let rb = Rebaseline {
            item_lines: Vec::new(),
            items_section: "Done when".to_string(),
            record: PathBuf::from("docs/adr/ADR-1-a.md"),
            field: "Realized-by".to_string(),
            line: 2,
            pins: vec![PinDrift {
                locator: "a.rs".to_string(),
                recorded_hash: h.to_string(),
                current_hash: "bbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                span: None,
                files: Vec::new(),
                spelling: "a.rs".to_string(),
            }],
        };
        let who = Attestation::new("ana", IdentitySource::Asserted, ActorKind::Agent);
        let err = apply_rebaseline(
            content,
            &rb,
            "2026-11-06",
            &who,
            "R.",
            &Default::default(),
            &Default::default(),
        )
        .unwrap_err();
        assert!(err.contains("in a Done when item clause"), "{err}");
        assert!(!err.contains("Done means"), "{err}");
    }

    fn no_relation_field_overrides() -> Config {
        Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            roadmap: Default::default(),
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            locator_resolvers: Default::default(),
            upgraded_from: None,
            override_log: None,
            block_on: None,
            attestation: Default::default(),
            new: Default::default(),
            discovery: Default::default(),
            harness: None,
            authority: None,
            rules: std::collections::BTreeMap::new(),
            record_types: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn a_sticky_state_is_never_repaired_and_a_neighbour_still_is() {
        let mut config = no_relation_field_overrides();
        config.rules.insert(
            crate::rules::RULE_EMBODIMENT_CONSISTENCY.to_string(),
            yaml_serde::from_str("{level: warn, sticky_states: [Deprecated]}").unwrap(),
        );
        let sticky = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Deprecated\nRealized-by: spec:docs/x.md\n---\n",
        );
        let neighbour = record(
            "docs/adr/0002-y.md",
            "---\nEmbodiment: Verified\nRealized-by: spec:docs/x.md\n---\n",
        );
        let (population, repairs) = detect(
            &[sticky, neighbour],
            &config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!(population.examined(), 2);
        assert_eq!(repairs.len(), 1);
        assert!(repairs[0].record.ends_with("0002-y.md"));
    }

    #[test]
    fn a_back_pointed_record_repairs_to_implemented_not_specified_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Specified\nRealized-by: spec:docs/x.md\n---\n",
        );
        let cited: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &crate::rules::BackPointerEvidence {
                code: cited.clone(),
                ..Default::default()
            },
        );
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].computed_value, "Implemented");
    }

    #[test]
    fn a_mismatched_embodiment_is_a_tier_1_repair_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Not started\nRealized-by: code:src/lib.rs\n---\n",
        );
        let (population, repairs) = detect(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].field, "Embodiment");
        assert_eq!(repairs[0].current_value, "Not started");
        assert_eq!(repairs[0].computed_value, "Implemented");
        assert_eq!(repairs[0].tier, 1);
    }

    #[test]
    fn a_drifted_record_is_repaired_to_drift_detected_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n---\n",
        );
        let drifted: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect(
            &[r],
            &no_relation_field_overrides(),
            &drifted,
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!(repairs.len(), 1);
        assert_eq!(repairs[0].computed_value, "Drift detected");
    }

    #[test]
    fn a_matching_embodiment_is_not_a_repair() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Implemented\nRealized-by: code:src/lib.rs\n---\n",
        );
        let (population, repairs) = detect(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert!(repairs.is_empty());
    }

    #[test]
    fn a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing()
    {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nPhase: Not started\nEvidence: code:src/lib.rs\n---\n",
        );
        let config = Config {
            schema_version: crate::config::CURRENT_SCHEMA_VERSION,
            roadmap: Default::default(),
            relation_pairs: Vec::new(),
            fail_on: Default::default(),
            embodiment: Default::default(),
            locator_resolvers: Default::default(),
            upgraded_from: None,
            override_log: None,
            block_on: None,
            attestation: Default::default(),
            new: Default::default(),
            discovery: Default::default(),
            harness: None,
            authority: None,
            rules: std::collections::BTreeMap::new(),
            record_types: [(
                "adr".to_string(),
                crate::config::RecordTypeConfig {
                    exclude_files: Vec::new(),
                    dir: "docs/adr".to_string(),
                    extra_dirs: Vec::new(),
                    identity: None,
                    fields: None,
                    sections: None,
                    statuses: None,
                    required_fields: vec![],
                    header_shape: crate::header::HeaderShape::default(),
                    prefix: None,
                    known_fields: Some(vec!["Phase".to_string(), "Evidence".to_string()]),
                    pointer_fields: None,
                    narrative_fields: None,
                    spec: None,
                    companion_fields: None,
                    relation_fields: Some(crate::config::RelationFields {
                        embodiment_state: Some("Phase".to_string()),
                        embodiment_locator: Some("Evidence".to_string()),
                        ..Default::default()
                    }),
                    revision_log: None,
                    role: None,
                    effective_requires: None,
                    permanent: None,
                },
            )]
            .into_iter()
            .collect(),
        };
        let (population, repairs) = detect(
            &[r],
            &config,
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 1));
        assert_eq!(repairs.len(), 1, "{repairs:?}");
        assert_eq!(repairs[0].field, "Phase");
        assert_eq!(repairs[0].computed_value, "Implemented");
    }

    #[test]
    fn no_realized_by_leaves_the_record_eligible_but_unexamined() {
        // Absent, not out of population: the record was handed to this
        // function and reached no verdict, which is what `eligible() > 0`
        // says and a bare examined-count could not.
        let r = record("docs/adr/0001-x.md", "---\nEmbodiment: Not started\n---\n");
        let (population, repairs) = detect(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &HashSet::new(),
            &Default::default(),
        );
        assert_eq!((population.eligible(), population.examined()), (1, 0));
        assert!(repairs.is_empty());
    }

    fn sample_repair() -> Repair {
        Repair {
            record: PathBuf::from("docs/adr/0001-x.md"),
            field: "Embodiment".to_string(),
            line: 2,
            current_value: "Not started".to_string(),
            computed_value: "Verified".to_string(),
            tier: 1,
            evidence: "Realized-by: code:src/lib.rs, test:tests/it.rs".to_string(),
        }
    }

    #[test]
    fn apply_repair_edits_the_field_and_appends_a_revision_log_row_observed_failing() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            true,
        )
        .unwrap();
        assert!(result.contains("Embodiment: Verified"));
        assert!(!result.contains("Embodiment: Not started"));
        assert!(result.contains(
            "Tool-authored (by beau, identity: gh-verified, actor: unspecified): recomputed"
        ));
        assert!(result.contains("**structural**"));
        // the original row survives untouched
        assert!(result.contains("| 2026-08-20 | Initial. | **structural** |"));
    }

    fn empty_value_repair() -> Repair {
        Repair {
            current_value: String::new(),
            computed_value: "Implemented".to_string(),
            ..sample_repair()
        }
    }

    #[test]
    fn apply_repair_fills_a_null_tilde_or_empty_value_by_position_observed_failing() {
        for raw in ["null", "~", "", "Null", "NULL"] {
            let content = format!(
                "---\nEmbodiment: {raw}\nRealized-by: code:src/lib.rs\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n"
            );
            let result = apply_repair(
                &content,
                &empty_value_repair(),
                "2026-09-05",
                &attest("beau"),
                &Default::default(),
                &Default::default(),
                true,
            )
            .unwrap();
            assert!(
                result.contains("\nEmbodiment: Implemented\n"),
                "{raw:?}: {result}"
            );
            assert!(!result.contains("ImplementedEmbodiment"), "{raw:?}");
            assert!(result.contains("stated no value disagreed"), "{raw:?}");
            assert!(!result.contains("stated `` disagreed"), "{raw:?}");
        }
    }

    #[test]
    fn apply_repair_does_not_touch_a_substring_of_the_field_name_or_prose_observed_failing() {
        // The stated value "Done" also occurs inside "Done-when:" and prose; only the
        // value after `Status:` may change.
        let repair = Repair {
            field: "Status".to_string(),
            current_value: "Done".to_string(),
            computed_value: "Planned".to_string(),
            ..sample_repair()
        };
        let content = "---\nDone-when: Done\nStatus: Done\n---\n# Done\n";
        let result = apply_repair(
            content,
            &repair,
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            "---\nDone-when: Done\nStatus: Planned\n---\n# Done\n"
        );
    }

    #[test]
    fn apply_repair_refuses_an_empty_stated_value_when_the_field_now_has_one() {
        let content = "---\nEmbodiment: Verified\n---\n# 0001 — X\n";
        let err = apply_repair(
            content,
            &empty_value_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            false,
        )
        .unwrap_err();
        assert!(err.contains("could not find"), "{err}");
    }

    #[test]
    fn apply_repair_refuses_without_a_revision_log_section() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n";
        let err = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            true,
        )
        .unwrap_err();
        assert!(err.contains("Revision log"));
    }

    #[test]
    fn apply_repair_without_a_required_log_edits_only_the_field() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            false,
        )
        .unwrap();
        assert_eq!(result, "---\nEmbodiment: Verified\n---\n# 0001 — X\n");
    }

    #[test]
    fn apply_repair_refuses_when_the_stated_value_has_already_changed() {
        // The record changed since detect ran (e.g. a concurrent edit) --
        // RFC-0008 §4's re-verify-before-write requirement, exercised at the
        // level this pure function can enforce: it can't find what it was
        // told to replace.
        let content = "---\nEmbodiment: Implemented\n---\n# 0001 — X\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Initial. | **structural** |\n";
        let err = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            true,
        )
        .unwrap_err();
        assert!(err.contains("could not find"));
    }

    #[test]
    fn apply_repair_appends_a_plain_table_row_inside_the_plain_table_observed_failing() {
        let content = "---\nEmbodiment: Not started\n---\n# 0001 — X\n\n**Revision log**\n\n| Date | Change | Class |\n|---|---|---|\n| 2026-08-20 | Initial. | **structural** |\n";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            true,
        )
        .unwrap();
        let last = result.lines().last().unwrap();
        assert!(last.starts_with("| 2026-09-05 |"), "{last}");
        assert!(!result.contains("> | 2026-09-05"));
    }

    #[test]
    fn apply_repair_on_a_crlf_file_changes_only_the_intended_lines_observed_failing() {
        let content = "# 0001 — X\r\n\r\n> Embodiment: Not started\r\n\r\n> **Revision log**\r\n>\r\n> | Date | Change | Class |\r\n> |---|---|---|\r\n> | 2026-08-20 | Initial. | **structural** |";
        let result = apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            true,
        )
        .unwrap();
        assert!(result.starts_with(
            "# 0001 — X\r\n\r\n> Embodiment: Verified\r\n\r\n> **Revision log**\r\n>\r\n"
        ));
        assert!(!result.replace("\r\n", "").contains('\n'), "bare LF");
        assert!(!result.ends_with('\n'), "trailing newline was added");
        assert_eq!(result.lines().count(), content.lines().count() + 1);
    }

    #[test]
    fn fresh_drift_leaves_a_stated_drift_detected_alone_observed_failing() {
        let r = record(
            "docs/adr/0001-x.md",
            "---\nEmbodiment: Drift detected\nRealized-by: code:src/lib.rs\n---\n",
        );
        let fresh: HashSet<PathBuf> = [PathBuf::from("docs/adr/0001-x.md")].into();
        let (_, repairs) = detect(
            &[r],
            &no_relation_field_overrides(),
            &HashSet::new(),
            &fresh,
            &Default::default(),
        );
        assert!(repairs.is_empty(), "{repairs:?}");
    }

    const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn pinned(path: &str, hash: &str) -> Record {
        record(
            path,
            &format!(
                "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{hash}, test:t.rs'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
            ),
        )
    }

    #[test]
    fn a_stale_pin_is_listed_with_both_hashes_and_a_current_one_is_not_observed_failing() {
        let stale = pinned("docs/adr/0001-x.md", H1);
        let current = pinned("docs/adr/0002-y.md", H2);
        let lookup = |l: &crate::locator::Locator| {
            if l.path == "src/a.rs" {
                PinRead::Current {
                    hash: H2.to_string(),
                    span: None,
                    files: Vec::new(),
                }
            } else {
                PinRead::Unreadable
            }
        };
        let (population, found, failed) =
            detect_rebaselines(&[stale, current], &no_relation_field_overrides(), &lookup);
        assert!(failed.is_empty());
        assert_eq!(population.examined(), 2);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pins[0].recorded_hash, H1);
        assert_eq!(found[0].pins[0].current_hash, H2);
    }

    #[test]
    fn re_pinning_moves_only_the_hash_and_logs_who_and_why_observed_failing() {
        let r = pinned("docs/adr/0001-x.md", H1);
        let lookup = |_: &crate::locator::Locator| PinRead::Current {
            hash: H2.to_string(),
            span: None,
            files: Vec::new(),
        };
        let (_, found, _) = detect_rebaselines(&[r], &no_relation_field_overrides(), &lookup);
        let content = format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{H1}, test:t.rs'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
        );
        let out = apply_rebaseline(
            &content,
            &found[0],
            "2026-11-04",
            &attest("ana"),
            "Read the diff; the claim holds.",
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert!(out.contains(&format!(
            "Realized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{H2}, test:t.rs'"
        )));
        assert!(out.contains("attested by ana"));
        assert!(out.contains("Read the diff; the claim holds."));
        // Applying the same rebaseline again finds no pin to move.
        assert!(apply_rebaseline(
            &out,
            &found[0],
            "d",
            &attest("ana"),
            "r",
            &Default::default(),
            &Default::default()
        )
        .is_err());
    }

    fn scoped(path: &str, spec: &str, hash: &str) -> Record {
        record(
            path,
            &format!(
                "---\nEmbodiment: Implemented\nRealized-by: 'code:src/a.rs{spec}@sha256:{hash}, code:src/b.rs#Gone@sha256:{hash}'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
            ),
        )
    }

    #[test]
    fn a_scoped_pin_is_re_pinned_in_its_own_spelling_and_an_unresolving_one_is_a_failed_pin() {
        let r = scoped("docs/adr/0001-x.md", "#Foo::bar", H1);
        let lookup = |l: &crate::locator::Locator| match l.path.as_str() {
            "src/a.rs" => PinRead::Current {
                hash: H2.to_string(),
                span: Some(PinSpan {
                    start: 10,
                    end: 14,
                    text: "fn bar() {}".to_string(),
                }),
                files: Vec::new(),
            },
            _ => PinRead::Failed("'Gone' matches nothing".to_string()),
        };
        let (_, found, failed) = detect_rebaselines(
            std::slice::from_ref(&r),
            &no_relation_field_overrides(),
            &lookup,
        );
        // Per pin, not per record: the resolvable one is listed, the other fails.
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pins.len(), 1);
        assert_eq!(found[0].pins[0].locator, "src/a.rs#Foo::bar");
        assert_eq!(found[0].pins[0].span.as_ref().map(|s| s.start), Some(10));
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].locator, "src/b.rs#Gone");
        assert!(failed[0].reason.contains("matches nothing"));

        let content = format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'code:src/a.rs#Foo::bar@sha256:{H1}, code:src/b.rs#Gone@sha256:{H1}'\n---\n# 1\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-01-01 | Initial. | **structural** |\n"
        );
        let out = apply_rebaseline(
            &content,
            &found[0],
            "2026-11-04",
            &attest("ana"),
            "ok.",
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        assert!(out.contains(&format!(
            "code:src/a.rs#Foo::bar@sha256:{H2}, code:src/b.rs#Gone@sha256:{H1}"
        )));
        assert!(out.contains("(lines 10-14)"));
    }

    #[test]
    fn a_status_repair_edits_the_status_line_not_a_title_that_mentions_both_words() {
        let content = "---\nStatus: Planned\n---\n# Planned work whose Status is open\n\n> **Revision log**\n>\n> | Date | Change | Class |\n> |---|---|---|\n> | 2026-08-20 | Filed. | **structural** |\n";
        let repair = Repair {
            record: PathBuf::from("docs/milestones/MILE-1-x.md"),
            field: "Status".to_string(),
            line: 2,
            current_value: "Planned".to_string(),
            computed_value: "Done".to_string(),
            tier: 1,
            evidence: "Realized-by computes to Implemented".to_string(),
        };
        let result = apply_repair(
            content,
            &repair,
            "2026-11-04",
            &attest("beau"),
            &Default::default(),
            &Default::default(),
            true,
        )
        .unwrap();
        assert!(result.contains("\nStatus: Done\n"), "{result}");
        assert!(
            result.contains("# Planned work whose Status is open"),
            "{result}"
        );
        assert!(
            result.contains("disagreed with the computed value `Done`"),
            "{result}"
        );
    }

    // ---- ADR-94: the tool writes the type's tool_class, laid out by the log's header ----

    fn shape(
        classes: Option<&[&str]>,
        class_column: Option<&str>,
        date_column: Option<&str>,
        tool_class: &str,
    ) -> crate::config::RevisionLogShape {
        crate::config::RevisionLogShape {
            classes: classes.map_or_else(
                || crate::config::RevisionLogShape::default().classes,
                |c| c.iter().map(|c| c.to_string()).collect(),
            ),
            class_column: class_column.map(str::to_string),
            date_column: date_column.map(str::to_string),
            tool_class: tool_class.to_string(),
        }
    }

    fn repair_with(
        content: &str,
        shape: &crate::config::RevisionLogShape,
    ) -> Result<String, String> {
        apply_repair(
            content,
            &sample_repair(),
            "2026-09-05",
            &attest("beau"),
            &Default::default(),
            shape,
            true,
        )
    }

    fn log_doc(header: &str, sep: &str, row: &str) -> String {
        format!(
            "---\nEmbodiment: Not started\n---\n# 0001 — X\n\n> **Revision log**\n>\n> {header}\n> {sep}\n> {row}\n"
        )
    }

    #[test]
    fn apply_repair_writes_the_declared_tool_class_not_a_literal() {
        let content = log_doc(
            "| Date | Change | Class |",
            "|---|---|---|",
            "| 2026-08-20 | Initial. | **minor** |",
        );
        let out = repair_with(
            &content,
            &shape(Some(&["major", "minor"]), None, None, "minor"),
        )
        .unwrap();
        let row = out.lines().find(|l| l.contains("Tool-authored")).unwrap();
        assert!(row.ends_with("| **minor** |"), "{row}");
        assert!(!out.contains("**structural**"), "{out}");
    }

    #[test]
    fn apply_repair_lays_the_row_out_by_the_declared_header() {
        let content = log_doc(
            "| Class | Date | Change |",
            "|---|---|---|",
            "| **major** | 2026-08-20 | Initial. |",
        );
        let out = repair_with(
            &content,
            &shape(
                Some(&["major", "structural"]),
                Some("Class"),
                Some("Date"),
                "major",
            ),
        )
        .unwrap();
        let row = out.lines().find(|l| l.contains("Tool-authored")).unwrap();
        assert!(
            row.starts_with("> | **major** | 2026-09-05 | Tool-authored"),
            "{row}"
        );
        assert!(row.ends_with(". |"), "{row}");
        assert_eq!(row.matches('|').count(), 4, "{row}");
    }

    #[test]
    fn apply_repair_leaves_other_columns_empty_and_puts_the_narrative_in_the_first_remaining() {
        let content = log_doc(
            "| Date | Change | Notes | Class |",
            "|---|---|---|---|",
            "| 2026-08-20 | Initial. | n | **structural** |",
        );
        let out = repair_with(&content, &Default::default()).unwrap();
        let row = out.lines().find(|l| l.contains("Tool-authored")).unwrap();
        assert!(row.starts_with("> | 2026-09-05 | Tool-authored"), "{row}");
        assert!(row.ends_with("|  | **structural** |"), "{row}");
        assert_eq!(row.matches('|').count(), 5, "{row}");
    }

    #[test]
    fn apply_repair_refuses_a_log_with_no_column_left_for_the_narrative() {
        let content = log_doc(
            "| Date | Class |",
            "|---|---|",
            "| 2026-08-20 | **structural** |",
        );
        let err = repair_with(&content, &Default::default()).unwrap_err();
        assert!(err.contains("narrative"), "{err}");
        // A date column that is the class column leaves none either.
        let content = log_doc(
            "| Date | Change | Class |",
            "|---|---|---|",
            "| 2026-08-20 | A. | **structural** |",
        );
        let err =
            repair_with(&content, &shape(None, Some("Date"), None, "structural")).unwrap_err();
        assert!(err.contains("narrative") || err.contains("same"), "{err}");
    }

    #[test]
    fn apply_repair_refuses_a_log_whose_declared_column_is_missing() {
        let content = log_doc(
            "| Date | Change | Kind |",
            "|---|---|---|",
            "| 2026-08-20 | A. | **structural** |",
        );
        let err =
            repair_with(&content, &shape(None, Some("Class"), None, "structural")).unwrap_err();
        assert!(err.contains("no column named"), "{err}");
        assert!(err.contains("Class"), "{err}");
    }

    #[test]
    fn apply_rebaseline_writes_the_declared_tool_class_laid_out_by_the_header() {
        let r = pinned("docs/adr/0001-x.md", H1);
        let lookup = |_: &crate::locator::Locator| PinRead::Current {
            hash: H2.to_string(),
            span: None,
            files: Vec::new(),
        };
        let (_, found, _) = detect_rebaselines(&[r], &no_relation_field_overrides(), &lookup);
        let content = format!(
            "---\nEmbodiment: Implemented\nRealized-by: 'spec:docs/s.md, code:src/a.rs@sha256:{H1}, test:t.rs'\n---\n# 1\n\n> **Revision log**\n>\n> | Class | Date | Change |\n> |---|---|---|\n> | **major** | 2026-01-01 | Initial. |\n"
        );
        let out = apply_rebaseline(
            &content,
            &found[0],
            "2026-11-04",
            &attest("ana"),
            "Read the diff.",
            &Default::default(),
            &shape(
                Some(&["major", "minor"]),
                Some("Class"),
                Some("Date"),
                "major",
            ),
        )
        .unwrap();
        let row = out.lines().find(|l| l.contains("Re-baselined")).unwrap();
        assert!(
            row.starts_with("> | **major** | 2026-11-04 | Re-baselined (attested by ana"),
            "{row}"
        );
        assert!(!out.contains("**structural**"), "{out}");
    }
}
