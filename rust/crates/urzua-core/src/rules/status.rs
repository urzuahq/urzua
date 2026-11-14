use super::*;
use crate::status_model::StatusDecl;

/// A type's validated `statuses` declaration, by type name, only for a type
/// that declares one (`MILE-132`): an undeclared type must stay absent from
/// the map so every rule reading it keeps its prior behaviour.
pub fn statuses_by_type(config: &Config) -> HashMap<String, StatusDecl> {
    config
        .record_types
        .iter()
        .filter_map(|(name, cfg)| cfg.status_decl().map(|d| (name.clone(), d)))
        .collect()
}

/// Whether `status` finishes a record of `record_type`: the declared success
/// and failure terminals when the type declares them, otherwise the rule's own
/// `fallback` list (`BUG-178`).
pub(super) fn is_finished(
    declared: &HashMap<String, StatusDecl>,
    record_type: &str,
    status: &str,
    fallback: &[String],
) -> bool {
    match declared.get(record_type) {
        Some(decl) => decl.is_terminal(status),
        None => fallback.iter().any(|s| s == status),
    }
}

/// What `judge` is handed for one candidate record.
struct Judged<'a> {
    record: &'a Record,
    decl: &'a StatusDecl,
    record_type: &'a str,
    status_field: &'a str,
    status: &'a str,
}

fn run_status_rule(
    records: &[Record],
    config: &Config,
    rule_id: &'static str,
    variant: &'static str,
    eligible: impl Fn(&StatusDecl) -> bool,
    judge: impl Fn(&Judged) -> Vec<(Option<String>, String)>,
) -> (RuleExecution, Vec<Finding>) {
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| declared.get(&r.record_type).is_some_and(&eligible))
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let decl = &declared[&record.record_type];
            let status_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Status,
            );
            let status = match declared_value(record, status_field) {
                Ok(v) if !v.trim().is_empty() => v,
                Ok(_) => return Outcome::Absent,
                Err(outcome) => return outcome,
            };
            let judged = Judged {
                record,
                decl,
                record_type: &record.record_type,
                status_field,
                status,
            };
            for (field, message) in judge(&judged) {
                let declared_severity = field
                    .as_deref()
                    .and_then(|f| declared_field_severity(config, rule_id, f));
                // The edit lands on the field the finding names; one that names
                // none (an undeclared status, a forbidden field) is about the
                // status field or the field the message quotes.
                let action_field = field
                    .clone()
                    .unwrap_or_else(|| judged.status_field.to_string());
                findings.push(
                    Finding::new(
                        rule_id.to_string(),
                        declared_severity.unwrap_or(FindingSeverity::Error),
                        record.path.clone(),
                        ActionSource::new(variant).field(action_field),
                        message,
                    )
                    .declared_opt(declared_severity),
                );
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(rule_id.to_string(), population, examined_records),
        findings,
    )
}

/// How one record's `Status` stands against its delivery evidence
/// (`MILE-183`), shared by `status.delivery-agreement` and `urzua fix`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryJudgement {
    pub status_field: String,
    pub status: String,
    /// The tier computed from the record's evidence (drift is not read: it says
    /// the evidence is stale, not that nothing was delivered). For a type that
    /// declares `delivery.items` it is the lowest tier among the resolved items.
    pub tier: String,
    /// What the evidence was, in words a repair row can cite.
    pub evidence: String,
    pub disagreement: Option<DeliveryDisagreement>,
    /// The per-item judgement, for a type that declares `delivery.items`
    /// (`ADR-96`).
    pub items: Option<ItemsJudgement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryDisagreement {
    /// The evidence says delivered while the status is declared not done. The
    /// one correct value is the type's `set_status`.
    DeliveredButNotDone { set_status: String },
    /// A success status with evidence that says nothing was delivered. No
    /// single correct value exists: the repair is adding evidence.
    SuccessWithoutEvidence,
    /// Some items resolve while the status is a not-done status short of
    /// `partial_status` (`ADR-96`). `blocked` records that the record is
    /// derived-blocked, where `fix` writes nothing.
    PartialButNotStarted {
        partial_status: String,
        blocked: bool,
    },
    /// A `success` status while an item is unresolved (`ADR-96`): a guaranteed
    /// error. `reopen_to` is the status an edit would reopen the record to.
    DoneWithUnresolvedItems {
        unresolved: Vec<UnresolvedItem>,
        reopen_to: String,
    },
}

/// What a judgement reads beyond the record: the back-pointers that cite it,
/// the working tree its item locators resolve in, and the records that are
/// derived-blocked (`ADR-96` section 7).
pub struct DeliveryContext<'a> {
    pub back_pointed: &'a BackPointerEvidence,
    pub env: &'a LocatorEnv<'a>,
    pub blocked: &'a HashMap<PathBuf, Vec<String>>,
}

/// Judges one record of a type that declares `statuses.delivery`. `Err` is the
/// census outcome for a record that cannot be judged (no status, an unreadable
/// header).
pub fn judge_delivery(
    record: &Record,
    config: &Config,
    decl: &StatusDecl,
    ctx: &DeliveryContext<'_>,
) -> Result<DeliveryJudgement, Outcome> {
    use crate::config::RelationRole::Status;
    let delivery = decl.delivery.as_ref().ok_or(Outcome::Absent)?;
    let status_field = relation_field_name(config, &record.record_type, Status);
    let status = match declared_value(record, status_field) {
        Ok(v) if !v.trim().is_empty() => v.trim(),
        Ok(_) => return Err(Outcome::Absent),
        Err(outcome) => return Err(outcome),
    };
    let input = DeliveryInput {
        record,
        config,
        decl,
        delivery,
        ctx,
        status_field,
        status,
    };
    // A type that declares `delivery.items` is judged from the items alone: a
    // record-level locator never makes a milestone delivered (`ADR-96`
    // section 5).
    if delivery.items.is_some() {
        return Ok(judge_delivery_items(&input));
    }
    judge_delivery_evidence(&input)
}

/// What one delivery judgement reads.
struct DeliveryInput<'a> {
    record: &'a Record,
    config: &'a Config,
    decl: &'a StatusDecl,
    delivery: &'a crate::status_model::DeliveryDecl,
    ctx: &'a DeliveryContext<'a>,
    status_field: &'a str,
    status: &'a str,
}

fn in_list(list: &[String], v: &str) -> bool {
    list.iter().any(|e| e == v)
}

/// A type that declares `delivery.items`, judged from its Done means items.
fn judge_delivery_items(input: &DeliveryInput<'_>) -> DeliveryJudgement {
    let DeliveryInput {
        record,
        config,
        delivery,
        ctx,
        status_field,
        status,
        ..
    } = *input;
    let empty = crate::done_items::DoneItems::default();
    let mut judged = judge_items(
        record,
        record.items.as_ref().unwrap_or(&empty),
        config,
        delivery,
        ctx.env,
    );
    let open = ctx.blocked.get(&record.path);
    let blocked = open.is_some();
    // Dependencies cap the status (`ADR-96`, amendment of 2026-11-07): the
    // computed status is the lower of the item result and the dependency
    // result, so items that all resolve beside an unfinished dependency are
    // partial work, not delivered. It lowers `Delivered` only, and only
    // where items and dependencies are both declared (`blocked` is empty
    // without a sequencing pair).
    if let Some(open) = open {
        if judged.standing == Standing::Delivered {
            judged.standing = Standing::Partial;
            judged.capped_by = open.clone();
        }
    }
    let disagreement = items_disagreement(input, &judged, blocked);
    DeliveryJudgement {
        status_field: status_field.to_string(),
        status: status.to_string(),
        tier: judged.tier.clone(),
        evidence: format!(
            "the {} items ({})",
            delivery
                .items
                .as_ref()
                .expect("checked by the caller")
                .section,
            judged.counts()
        ),
        disagreement,
        items: Some(judged),
    }
}

/// How the status stands against the judged items (`ADR-96`).
fn items_disagreement(
    input: &DeliveryInput<'_>,
    judged: &ItemsJudgement,
    blocked: bool,
) -> Option<DeliveryDisagreement> {
    let DeliveryInput {
        decl,
        delivery,
        status,
        ..
    } = *input;
    let items_decl = delivery.items.as_ref().expect("checked by the caller");
    let unresolved = !judged.unresolved.is_empty();
    if in_list(&decl.success, status) && unresolved {
        let to_partial = judged.resolved_delivered >= 1
            && !(blocked && decl.is_progress_status(&items_decl.partial_status));
        let reopen_to = if to_partial {
            items_decl.partial_status.clone()
        } else {
            first_non_progress_status(decl)
                .unwrap_or(&items_decl.partial_status)
                .to_string()
        };
        Some(DeliveryDisagreement::DoneWithUnresolvedItems {
            unresolved: judged.unresolved.clone(),
            reopen_to,
        })
    } else if in_list(&delivery.not_done, status) && judged.standing == Standing::Delivered {
        Some(DeliveryDisagreement::DeliveredButNotDone {
            set_status: delivery.set_status.clone(),
        })
    } else if in_list(&delivery.not_done, status)
        && status != items_decl.partial_status
        && judged.standing == Standing::Partial
    {
        Some(DeliveryDisagreement::PartialButNotStarted {
            partial_status: items_decl.partial_status.clone(),
            // `fix` holds back only a status the derived blocked value
            // would refuse for the record. `sequence.status-agreement`
            // accepts `partial_status` on a capped record (every item
            // resolves, only a dependency is open), so a capped record is
            // not held back; one with unresolved items is.
            blocked: blocked
                && judged.capped_by.is_empty()
                && decl.is_progress_status(&items_decl.partial_status),
        })
    } else if in_list(&decl.success, status) && in_list(&delivery.unevidenced_tiers, &judged.tier) {
        Some(DeliveryDisagreement::SuccessWithoutEvidence)
    } else {
        None
    }
}

/// A type judged from its evidence field and the back-pointers that cite it.
fn judge_delivery_evidence(input: &DeliveryInput<'_>) -> Result<DeliveryJudgement, Outcome> {
    use crate::config::RelationRole::EmbodimentLocator;
    let DeliveryInput {
        record,
        config,
        decl,
        delivery,
        ctx,
        status_field,
        status,
    } = *input;
    let locator_field = relation_field_name(config, &record.record_type, EmbodimentLocator);
    // A record with no evidence field is the common case, not an absence.
    let realized_by = match declared_value(record, locator_field) {
        Ok(v) => Some(v),
        Err(Outcome::Absent) => None,
        Err(outcome) => return Err(outcome),
    };
    // `back_pointer_evidence: false` (`ADR-84`) holds the type to its declared
    // evidence field: a back-pointer that cites the record is then not read.
    let cited_by_code =
        delivery.back_pointer_evidence && ctx.back_pointed.code.contains(&record.path);
    let cited_by_test =
        delivery.back_pointer_evidence && ctx.back_pointed.test.contains(&record.path);
    // Only locators that resolve are evidence (`embodiment.locator-exists`).
    let resolving = match realized_by {
        Some(value) => super::resolving_realized_by(value, locator_field, ctx.env).0,
        None => Default::default(),
    };
    let tier = compute_embodiment(
        &config.embodiment,
        &resolving,
        false,
        cited_by_code,
        cited_by_test,
    );
    let evidence = evidence_description(
        locator_field,
        realized_by.is_some(),
        cited_by_code,
        cited_by_test,
    );
    let disagreement =
        if in_list(&delivery.not_done, status) && in_list(&delivery.delivered_tiers, tier) {
            Some(DeliveryDisagreement::DeliveredButNotDone {
                set_status: delivery.set_status.clone(),
            })
        } else if in_list(&decl.success, status) && in_list(&delivery.unevidenced_tiers, tier) {
            Some(DeliveryDisagreement::SuccessWithoutEvidence)
        } else {
            None
        };
    Ok(DeliveryJudgement {
        status_field: status_field.to_string(),
        status: status.to_string(),
        tier: tier.to_string(),
        evidence,
        disagreement,
        items: None,
    })
}

/// A record's `Status` against its delivery evidence, both ways (`MILE-183`):
/// the evidence computes to a declared delivered tier while the status is
/// declared not done, and a success status whose evidence computes to a
/// declared unevidenced tier. The mapping is the type's `statuses.delivery`;
/// the rule hardcodes no status or type. A type declaring none is not examined.
///
/// A type that declares `delivery.items` (`ADR-96`) is judged from its Done
/// means items, and gains three disagreements: a `success` status while an item
/// is unresolved (`DoneWithUnresolvedItems`, a guaranteed error), partial work
/// under a status short of `partial_status`, and the others as before.
pub fn status_delivery_agreement(
    records: &[Record],
    config: &Config,
    back_pointed: &BackPointerEvidence,
    env: &LocatorEnv<'_>,
) -> (RuleExecution, Vec<Finding>) {
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| {
            declared
                .get(&r.record_type)
                .is_some_and(|d| d.delivery.is_some())
        })
        .collect();
    let any_items = candidates.iter().any(|r| r.items.is_some());
    let blocked = if any_items {
        derived_blocked_records(records, config)
    } else {
        HashMap::new()
    };
    let ctx = DeliveryContext {
        back_pointed,
        env,
        blocked: &blocked,
    };
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let decl = &declared[&record.record_type];
            let judged = match judge_delivery(record, config, decl, &ctx) {
                Ok(j) => j,
                Err(outcome) => return outcome,
            };
            findings.extend(delivery_finding(record, config, decl, &judged));
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_STATUS_DELIVERY_AGREEMENT, population, examined_records),
        findings,
    )
}

/// The finding for a judgement that disagrees; `None` when it agrees.
fn delivery_finding(
    record: &Record,
    config: &Config,
    decl: &StatusDecl,
    judged: &DeliveryJudgement,
) -> Option<Finding> {
    let mut line = None;
    let (message, source) = match judged.disagreement.as_ref()? {
        // `urzua fix --apply` writes this repair (`SPEC-8`), so the
        // command is offered.
        DeliveryDisagreement::DeliveredButNotDone { set_status } => (
            format!(
                "{} is '{}', but the evidence computes to '{}', from {}: the work reads as delivered; set {} to '{set_status}' (`urzua fix --apply` does)",
                judged.status_field, judged.status, judged.tier, judged.evidence, judged.status_field
            ),
            ActionSource::new("delivery-not-done").argv([
                "urzua",
                "fix",
                "--apply",
                "--ids",
                &record.path.display().to_string(),
            ]),
        ),
        // No single correct value exists and `fix` writes nothing for it,
        // so no command is offered.
        DeliveryDisagreement::SuccessWithoutEvidence => (
            format!(
                "{} is '{}', but the evidence computes to '{}', from {}: add the evidence that delivered it",
                judged.status_field, judged.status, judged.tier, judged.evidence
            ),
            ActionSource::new("delivery-without-evidence"),
        ),
        DeliveryDisagreement::PartialButNotStarted { partial_status, blocked: false } => (
            format!(
                "{} is '{}', but {} resolve, from {}: the work has begun{}; set {} to '{partial_status}' (`urzua fix --apply` does)",
                judged.status_field, judged.status, judged.evidence_counts(), judged.evidence, judged.cap_note(partial_status), judged.status_field
            ),
            ActionSource::new("delivery-partial-not-started").argv([
                "urzua",
                "fix",
                "--apply",
                "--ids",
                &record.path.display().to_string(),
            ]),
        ),
        // A derived-blocked record is not written a `progress` status
        // (`sequence.status-agreement` would refuse it): the finding
        // stays and says so.
        DeliveryDisagreement::PartialButNotStarted { partial_status, blocked: true } => (
            format!(
                "{} is '{}', but {} resolve, from {}: evidence has landed ahead of a dependency that is still open, so `urzua fix` does not write '{partial_status}' (it would contradict the derived blocked value); finish the open dependency",
                judged.status_field, judged.status, judged.evidence_counts(), judged.evidence
            ),
            ActionSource::new("delivery-partial-blocked"),
        ),
        DeliveryDisagreement::DoneWithUnresolvedItems { unresolved, reopen_to } => {
            line = unresolved.first().map(|u| u.line);
            unresolved_items_report(record, config, decl, judged, unresolved, reopen_to)
        }
    };
    Some(
        Finding::new(
            RULE_STATUS_DELIVERY_AGREEMENT,
            FindingSeverity::Error,
            record.path.clone(),
            source,
            message,
        )
        .at_opt(line),
    )
}

/// The message and action for a `success` status with unresolved Done means
/// items (`ADR-96`).
fn unresolved_items_report(
    record: &Record,
    config: &Config,
    decl: &StatusDecl,
    judged: &DeliveryJudgement,
    unresolved: &[UnresolvedItem],
    reopen_to: &str,
) -> (String, ActionSource) {
    let id = record_id(record).unwrap_or_else(|| record.path.display().to_string());
    let line = unresolved.first().map(|u| u.line);
    let clauses: Vec<String> = unresolved
        .iter()
        .map(|u| {
            format!(
                "{id} is {} but {} item {} \"{}\" is unresolved: {}",
                judged.status,
                items_section(config, &record.record_type),
                u.n,
                u.text,
                u.cause
            )
        })
        .collect();
    // Two actions are declared (`ADR-96`) and a finding carries one
    // (`ADR-73`): the reopen when the lifecycle lets the record move
    // there, else the edit that adds the evidence.
    let reopen_allowed = !decl.has_transitions() || decl.allows(&judged.status, reopen_to);
    let source = if reopen_allowed {
        ActionSource::new("done-with-unresolved-items")
            .field(judged.status_field.clone())
            .text(reopen_to.to_string())
    } else {
        ActionSource::new("done-with-unresolved-items-evidence")
            .section(items_section(config, &record.record_type))
            .line(line.unwrap_or_default())
    };
    (clauses.join("; "), source)
}

impl DeliveryJudgement {
    /// For a record whose items all resolve but whose dependency is unfinished,
    /// the clause naming the dependency that caps the status at `capped_at`;
    /// else empty (`ADR-96`, amendment of 2026-11-07).
    pub fn cap_note(&self, capped_at: &str) -> String {
        match self.items.as_ref().map(|i| i.capped_by.as_slice()) {
            Some(open) if !open.is_empty() => format!(
                " (every item resolves, but {} is unfinished, which caps the status at '{capped_at}')",
                open.join(", ")
            ),
            _ => String::new(),
        }
    }

    /// "3 of 5 items" for an item judgement, else the evidence tier.
    fn evidence_counts(&self) -> String {
        match &self.items {
            Some(i) => format!("{} of {} items", i.resolved, i.total()),
            None => self.tier.clone(),
        }
    }
}

/// A record's `Status` outside the values its type declares.
pub fn status_undeclared_value(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    run_status_rule(
        records,
        config,
        RULE_STATUS_UNDECLARED_VALUE,
        "status-undeclared-value",
        |_| true,
        |j| {
            if j.decl.declares(j.status) {
                return Vec::new();
            }
            vec![(
                None,
                format!(
                    "{} '{}' is not declared for type '{}' ({})",
                    j.status_field,
                    j.status,
                    j.record_type,
                    j.decl.declared_summary()
                ),
            )]
        },
    )
}

/// What the merge base says of a changed record's `Status` (`ADR-72`). The
/// caller reads it from git; the core never opens a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommittedStatus {
    /// The record is not in the base: it starts at its current status.
    New,
    /// The status the base committed.
    Status(String),
    /// The base has the record but no readable status.
    Unreadable,
}

/// A changed record whose `Status` moved along an edge its type's declared
/// `transitions` does not allow, or a new record that starts somewhere other
/// than `initial` (`MILE-144`, `ADR-72`). `committed` is `None` without
/// `--base`: the rule is then not applicable, not a pass. A record the diff did
/// not change is eligible and not examined.
pub fn status_transition_allowed(
    records: &[Record],
    config: &Config,
    committed: Option<&HashMap<PathBuf, CommittedStatus>>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_STATUS_TRANSITION_ALLOWED;
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| {
            declared
                .get(&r.record_type)
                .is_some_and(|d| d.has_transitions())
        })
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let decl = &declared[&record.record_type];
            let status_field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Status,
            );
            let status = match declared_value(record, status_field) {
                Ok(v) if !v.trim().is_empty() => v.trim(),
                Ok(_) => return Outcome::Absent,
                Err(outcome) => return outcome,
            };
            // Without `--base` there is no committed side: every record is
            // eligible and none examined, and the rule says it was not applicable.
            let Some(before) = committed.and_then(|c| c.get(&record.path)) else {
                return Outcome::Absent;
            };
            // A status the type does not declare is `status.undeclared-value`'s
            // finding; the graph has no edge to judge it by.
            if !decl.declares(status) {
                return Outcome::Absent;
            }
            let message = match before {
                CommittedStatus::Unreadable => return Outcome::Unreadable,
                CommittedStatus::New if decl.is_initial(status) => None,
                CommittedStatus::New => Some(format!(
                    "a new record starts at {status_field} '{status}', which is not an initial status (initial: {})",
                    decl.initial.join(", ")
                )),
                CommittedStatus::Status(old) if old == status => None,
                CommittedStatus::Status(old) if !decl.declares(old) => return Outcome::Unreadable,
                CommittedStatus::Status(old) if decl.allows(old, status) => None,
                CommittedStatus::Status(old) => {
                    let next = decl.transitions.get(old.as_str()).map(Vec::as_slice).unwrap_or(&[]);
                    Some(format!(
                        "{status_field} changed from '{old}' to '{status}', which '{}' does not allow (from '{old}': {})",
                        record.record_type,
                        if next.is_empty() { "nothing, it is terminal".to_string() } else { next.join(", ") }
                    ))
                }
            };
            if let Some(message) = message {
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Error,
                    record.path.clone(),
                    ActionSource::new("transition-not-allowed").field(status_field),
                    message,
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            status: if committed.is_some() {
                RuleStatus::Ran
            } else {
                RuleStatus::NotApplicable
            },
            ..RuleExecution::ran(RULE_ID, population, examined_records)
        },
        findings,
    )
}

/// A declared status no path of its type's `transitions` reaches from `initial`
/// (`MILE-144`, `ADR-72`). Judges the declaration, so it needs no `--base`. A
/// type declaring no graph is eligible and not examined.
pub fn status_unreachable(
    config: &Config,
    config_path: &std::path::Path,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_STATUS_UNREACHABLE;
    let mut findings = Vec::new();
    let population = census(
        PopulationUnit::RecordType,
        config.sorted_type_names(),
        |type_name| {
            let Some(decl) = config.record_types[*type_name].status_decl() else {
                return Outcome::Absent;
            };
            if !decl.has_transitions() {
                return Outcome::Absent;
            }
            for status in decl.unreachable() {
                findings.push(Finding::new(
                    RULE_ID,
                    FindingSeverity::Warning,
                    config_path.to_path_buf(),
                    ActionSource::new("status-unreachable")
                        .key_path(["record_types", type_name, "statuses", "transitions"]),
                    format!(
                        "record type '{type_name}' declares status '{status}' but no path of its transitions reaches it from initial ({})",
                        decl.initial.join(", ")
                    ),
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, Vec::new()),
        findings,
    )
}

/// A field the record's status requires, blank, unedited template text or a
/// value from the type's declared `pending` vocabulary.
pub fn status_required_field_missing(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    run_status_rule(
        records,
        config,
        RULE_STATUS_REQUIRED_FIELD_MISSING,
        "status-required-field-missing",
        |d| !d.require.is_empty(),
        |j| {
            j.decl
                .required_at(j.status)
                .into_iter()
                .filter_map(|field| {
                    // A declared pending vocabulary (MILE-153) is the type's own
                    // placeholder list: its whole values are missing here too. A
                    // type declaring none keeps the old reading, so the compiled-in
                    // `contains("pending")` match never reaches this rule.
                    let state = classify_with(j.record.header.get(field), Some(j.decl));
                    let declared_pending =
                        state == FieldState::Pending && j.decl.pending.is_some();
                    (matches!(state, FieldState::Blank | FieldState::Placeholder)
                        || declared_pending)
                        .then(|| {
                        (
                            Some(field.to_string()),
                            format!(
                                "{} is {}, which requires '{field}', but it is {state:?} -- not a real, present value",
                                j.status_field, j.status
                            ),
                        )
                    })
                })
                .collect()
        },
    )
}

/// A field the record's status forbids, carrying a value.
pub fn status_forbidden_field_present(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    run_status_rule(
        records,
        config,
        RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
        "status-forbidden-field-present",
        |d| !d.forbid.is_empty(),
        |j| {
            j.decl
                .forbidden_at(j.status)
                .into_iter()
                .filter(|field| classify(j.record.header.get(field)) != FieldState::Blank)
                .map(|field| {
                    (
                        // This rule takes no `field_levels`, so naming the field
                        // sets no severity; it tells the action which to remove.
                        Some(field.to_string()),
                        format!(
                            "{} is {}, which forbids '{field}', but the record carries it",
                            j.status_field, j.status
                        ),
                    )
                })
                .collect()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    fn typed(statuses: &str) -> Config {
        let yaml = format!(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Deciders, Accepted-on]\n    statuses:\n{statuses}\nrules:\n  identity.collision: error\n"
        );
        crate::config::parse(&yaml).expect("config loads")
    }

    fn adr(status: &str, extra: &str) -> Record {
        Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/adr/ADR-1-x.md"),
            "adr".to_string(),
            &format!("---\nStatus: {status}\n{extra}---\n# 1 - X\n"),
            crate::header::HeaderShape::YamlFrontmatter,
            "ADR".to_string(),
        )
    }

    const VOCAB: &str = "      values: [Proposed, Accepted, Rejected]\n      success: [Accepted]\n      failure: [Rejected]\n      require:\n        - {when: [Accepted], fields: [Deciders]}\n      forbid:\n        - {when: [Proposed, Rejected], fields: [Accepted-on]}\n";

    #[test]
    fn a_status_outside_the_declared_set_is_reported_observed_failing() {
        let config = typed(VOCAB);
        let (exec, findings) = status_undeclared_value(&[adr("Acceptd", "")], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Acceptd"), "{findings:?}");
        assert!(findings[0].message.contains("Proposed"), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_declared_status_is_not_reported() {
        let config = typed(VOCAB);
        let (exec, findings) = status_undeclared_value(&[adr("Accepted", "")], &config);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_type_declaring_no_statuses_is_not_examined() {
        let config = config_for_required(vec![("adr", vec!["Status"])]);
        let (exec, findings) = status_undeclared_value(&[adr("Whatever", "")], &config);
        assert!(findings.is_empty());
        let p = exec.population.unwrap();
        assert_eq!((p.eligible(), p.examined()), (0, 0));
    }

    #[test]
    fn an_accepted_record_without_deciders_fails_and_a_proposed_one_passes_observed_failing() {
        let config = typed(VOCAB);
        let (_, findings) = status_required_field_missing(&[adr("Accepted", "")], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Deciders"), "{findings:?}");
        let (_, findings) = status_required_field_missing(&[adr("Proposed", "")], &config);
        assert!(findings.is_empty(), "{findings:?}");
        let (_, findings) =
            status_required_field_missing(&[adr("Accepted", "Deciders: Ana\n")], &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_blank_or_template_value_does_not_satisfy_a_requirement() {
        let config = typed(VOCAB);
        let (_, findings) =
            status_required_field_missing(&[adr("Accepted", "Deciders: name\n")], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_declared_pending_value_does_not_satisfy_a_requirement_observed_failing() {
        let with_pending = format!(
            "{VOCAB}      pending: [\"(pending review)\", \"none recorded\", \"—\", Unknown]\n"
        );
        let config = typed(&with_pending);
        for v in [
            "(pending review)",
            "none recorded",
            "—",
            "Unknown",
            "unknown",
        ] {
            let extra = format!("Deciders: '{v}'\n");
            let (_, findings) = status_required_field_missing(&[adr("Accepted", &extra)], &config);
            assert_eq!(findings.len(), 1, "{v}: {findings:?}");
            assert!(findings[0].message.contains("Deciders"), "{findings:?}");
        }
        // A real value that merely contains the word is not reported.
        for v in ["Unknown Person", "Ana (pending nothing)", "Ana"] {
            let extra = format!("Deciders: '{v}'\n");
            let (_, findings) = status_required_field_missing(&[adr("Accepted", &extra)], &config);
            assert!(findings.is_empty(), "{v}: {findings:?}");
        }
    }

    #[test]
    fn without_a_declared_pending_vocabulary_a_dash_or_pending_text_still_satisfies() {
        let config = typed(VOCAB);
        for v in ["—", "Unknown", "pending review"] {
            let extra = format!("Deciders: '{v}'\n");
            let (_, findings) = status_required_field_missing(&[adr("Accepted", &extra)], &config);
            assert!(findings.is_empty(), "{v}: {findings:?}");
        }
    }

    #[test]
    fn a_forbidden_field_on_the_wrong_status_is_reported_observed_failing() {
        let config = typed(VOCAB);
        let carrying = "Accepted-on: 2026-01-01\n";
        let (_, findings) = status_forbidden_field_present(&[adr("Proposed", carrying)], &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Accepted-on"), "{findings:?}");
        let (_, findings) = status_forbidden_field_present(&[adr("Accepted", carrying)], &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn the_declared_terminals_replace_the_fallback_list_and_an_undeclared_type_keeps_it() {
        let config = typed(VOCAB);
        let declared = statuses_by_type(&config);
        let fallback = vec!["Done".to_string()];
        assert!(is_finished(&declared, "adr", "Rejected", &fallback));
        assert!(!is_finished(&declared, "adr", "Done", &fallback));
        assert!(!is_finished(&declared, "adr", "Proposed", &fallback));
        assert!(is_finished(&declared, "other", "Done", &fallback));
    }

    #[test]
    fn a_missing_required_field_reports_at_its_own_declared_level_observed_failing() {
        let yaml = "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    header_shape: yaml-frontmatter\n    required_fields: [Status]\n    known_fields: [Deciders, Reviewers]\n    statuses:\n      values: [Proposed, Accepted]\n      success: [Accepted]\n      require:\n        - {when: [Accepted], fields: [Deciders, Reviewers]}\nrules:\n  status.required-field-missing:\n    level: warn\n    field_levels: {Deciders: error}\n";
        let config = crate::config::parse(yaml).expect("config loads");
        let (_, findings) = status_required_field_missing(&[adr("Accepted", "")], &config);
        let d = findings
            .iter()
            .find(|x| x.message.contains("'Deciders'"))
            .unwrap();
        let r = findings
            .iter()
            .find(|x| x.message.contains("'Reviewers'"))
            .unwrap();
        assert_eq!(d.declared_severity, Some(FindingSeverity::Error));
        assert_eq!(r.declared_severity, None);
    }
}
