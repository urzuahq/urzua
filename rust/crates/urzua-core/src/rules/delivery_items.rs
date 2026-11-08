//! Done-means items as delivery evidence (`ADR-96`, `MILE-214`): reading the
//! declared section into a record, resolving each item against the working
//! tree, the standing that follows, and the two rules that hold the grammar
//! (`status.done-items-shape`) and the waivers (`status.waived-item-stale`).

use super::*;
use crate::done_items::{self, DoneItems, Item, ItemsDecl};
use crate::embodiment_model::EmbodimentDecl;
use crate::status_model::{DeliveryDecl, StatusDecl};
use std::borrow::Cow;

// Implements: ADR-96 (a milestone's status is computed from its Done-means items)

/// What `attach_done_items` needs of a type: the declaration and the required
/// section the items live in.
struct Spec {
    decl: ItemsDecl,
    required: crate::document_model::RequiredSection,
    depth: u8,
}

fn spec_for(
    config: &Config,
    declared: &HashMap<String, StatusDecl>,
    record_type: &str,
) -> Option<Spec> {
    let decl = declared
        .get(record_type)?
        .delivery
        .as_ref()?
        .items
        .clone()?;
    let rt = config.record_types.get(record_type)?;
    let sections = crate::document_model::declared_model(record_type, rt)
        .ok()
        .flatten()?
        .sections?;
    let required = sections
        .required
        .iter()
        .find(|r| r.name == decl.section)?
        .clone();
    Some(Spec {
        decl,
        required,
        depth: sections.depth,
    })
}

fn items_of(record: &Record, text: &str, spec: &Spec) -> DoneItems {
    let sections = super::section::record_section_tree(record, text, spec.depth);
    match super::section::find_section(&sections, &spec.required) {
        Some((section, _)) => {
            done_items::parse_items(&section.body, section.line + 1, section.line, &spec.decl)
        }
        None => DoneItems::default(),
    }
}

/// Reads each opted-in record's declared section into `record.items`. The core
/// opens no file, so the caller hands the text in, as it does for `section.*`.
/// A type that declares no `statuses.delivery.items` is left as it was.
pub fn attach_done_items(
    records: &mut [Record],
    config: &Config,
    full_text: &HashMap<PathBuf, String>,
) {
    let declared = statuses_by_type(config);
    let mut specs: HashMap<String, Option<Spec>> = HashMap::new();
    for record in records.iter_mut() {
        let spec = specs
            .entry(record.record_type.clone())
            .or_insert_with(|| spec_for(config, &declared, &record.record_type));
        let (Some(spec), Some(text)) = (spec.as_ref(), full_text.get(&record.path)) else {
            continue;
        };
        record.items = Some(items_of(record, text, spec));
    }
}

/// [`attach_done_items`] for one record, whose text is a proposal
/// (`check --stdin-path`).
pub fn attach_done_items_to(record: &mut Record, config: &Config, text: &str) {
    let declared = statuses_by_type(config);
    if let Some(spec) = spec_for(config, &declared, &record.record_type) {
        record.items = Some(items_of(record, text, &spec));
    }
}

/// The evidence field's text: `Realized-by` plus the locators of every
/// Done-means item clause (`ADR-96` section 5). `Embodiment`, drift,
/// `embodiment.locator-exists` and `fix --rebaseline` read this one function,
/// so an item locator is evidence for them as a header locator is.
pub fn evidence_text(record: &Record, key: &str) -> Option<String> {
    let items = record.items.as_ref().and_then(DoneItems::locator_text);
    match (record.header.get(key), items) {
        (Some(v), None) => Some(v.to_string()),
        (Some(v), Some(extra)) if v.trim().is_empty() => Some(extra),
        (Some(v), Some(extra)) => Some(format!("{}, {extra}", v.trim())),
        (None, extra) => extra,
    }
}

/// [`declared_value`], reading [`evidence_text`]: a record with item clauses
/// and no `Realized-by` is still a record with evidence.
pub(super) fn evidence_value<'a>(record: &'a Record, key: &str) -> Result<Cow<'a, str>, Outcome> {
    let items = record.items.as_ref().and_then(DoneItems::locator_text);
    match (declared_value(record, key), items) {
        (Ok(v), None) => Ok(Cow::Borrowed(v)),
        (Ok(v), Some(extra)) if v.trim().is_empty() => Ok(Cow::Owned(extra)),
        (Ok(v), Some(extra)) => Ok(Cow::Owned(format!("{}, {extra}", v.trim()))),
        (Err(Outcome::Absent), Some(extra)) => Ok(Cow::Owned(extra)),
        (Err(outcome), _) => Err(outcome),
    }
}

/// How one item's own evidence stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceStanding {
    /// The item has no evidence clause.
    None,
    /// Every locator exists and the tier they compute is in `resolving_tiers`.
    Resolves { tier: String },
    /// A clause that fails, and why (the message contract's `<cause>`).
    Fails { cause: String },
}

/// One item as the engine reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStanding {
    pub n: usize,
    pub line: usize,
    pub text: String,
    pub evidence: EvidenceStanding,
    pub waived: Option<done_items::Waiver>,
}

impl ItemStanding {
    pub fn resolved(&self) -> bool {
        matches!(self.evidence, EvidenceStanding::Resolves { .. })
    }

    /// Waived is the item excused while it does not resolve.
    pub fn is_waived(&self) -> bool {
        !self.resolved() && self.waived.is_some()
    }

    pub fn is_unresolved(&self) -> bool {
        !self.resolved() && self.waived.is_none()
    }

    /// The cause an unresolved item is reported with.
    pub fn cause(&self) -> String {
        match &self.evidence {
            EvidenceStanding::None => "no evidence clause".to_string(),
            EvidenceStanding::Fails { cause } => cause.clone(),
            EvidenceStanding::Resolves { .. } => String::new(),
        }
    }
}

/// One unresolved item, as `DoneWithUnresolvedItems` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedItem {
    pub n: usize,
    pub line: usize,
    pub text: String,
    pub cause: String,
}

/// The computed standing of a record's items (`ADR-96` section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Delivered,
    Partial,
    SpecifiedOnly,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemsJudgement {
    pub items: Vec<ItemStanding>,
    pub resolved: usize,
    /// Resolved at a `delivered_tiers` tier, not merely a `resolving_tiers` one.
    pub resolved_delivered: usize,
    pub waived: usize,
    pub unresolved: Vec<UnresolvedItem>,
    /// The lowest tier among the resolved items, else the declared `none`.
    pub tier: String,
    pub standing: Standing,
    /// The unfinished dependencies that lower a `Delivered` standing to
    /// `Partial` (`ADR-96`, amendment of 2026-11-07): the computed status is
    /// the lower of the item result and the dependency result. Empty when no
    /// dependency lowered anything.
    pub capped_by: Vec<String>,
}

impl ItemsJudgement {
    pub fn total(&self) -> usize {
        self.items.len()
    }

    /// "3 of 5 items resolved", with the waived ones counted aside.
    pub fn counts(&self) -> String {
        let mut text = format!("{} of {} items resolved", self.resolved, self.total());
        if self.waived > 0 {
            text.push_str(&format!(", {} waived", self.waived));
        }
        text
    }
}

fn tier_rank(embodiment: &EmbodimentDecl, tier: &str) -> usize {
    embodiment
        .tiers
        .iter()
        .position(|t| t.state == tier)
        .unwrap_or(embodiment.tiers.len())
}

/// How a record is judged, by its kind (`ADR-96`, amendment of 2026-11-07).
struct KindRule<'a> {
    /// The tiers at which an item counts as delivered.
    delivered_tiers: &'a [String],
    /// A locator that resolves to a record counts only while the record has
    /// reached a `success` status of its own type.
    cited_must_be_decided: bool,
}

/// The kind a record declares, as its header holds it: `None` when the type
/// declares no kind, or the record names none, or names one outside the
/// vocabulary (`status.done-items-shape` reports that one).
pub fn declared_kind<'a>(
    record: &Record,
    decl: &'a ItemsDecl,
) -> Option<&'a done_items::KindEntry> {
    let kind = decl.kind.as_ref()?;
    let value = record.header.get(kind.field.as_str())?.trim();
    decl.kind_entry(value)
}

fn kind_rule<'a>(record: &Record, delivery: &'a DeliveryDecl, decl: &'a ItemsDecl) -> KindRule<'a> {
    let entry = declared_kind(record, decl);
    KindRule {
        delivered_tiers: entry
            .and_then(|k| k.delivered_tiers.as_deref())
            .unwrap_or(&delivery.delivered_tiers),
        cited_must_be_decided: entry
            .is_some_and(|k| k.cited_records == Some(done_items::CitedRecords::Success)),
    }
}

/// The first locator of a clause that resolves to a record which has not
/// reached a `success` status of its own type, with the cause: `ADR-2 is
/// Proposed`. A locator that is not a record is not judged here.
fn undecided_record(
    entries: &[crate::locator::LocatorEntry],
    env: &LocatorEnv<'_>,
) -> Option<String> {
    let declared = statuses_by_type(env.config);
    for entry in entries {
        let path = entry
            .parsed
            .as_ref()
            .map_or_else(|_| entry.raw.clone(), |l| l.path.clone());
        let Some(cited) = env.records.iter().find(|r| r.path == Path::new(&path)) else {
            continue;
        };
        let field = relation_field_name(
            env.config,
            &cited.record_type,
            crate::config::RelationRole::Status,
        );
        let status = cited.header.get(field).map(str::trim).unwrap_or("");
        let decided = declared
            .get(&cited.record_type)
            .is_some_and(|d| d.is_success(status));
        if !decided {
            return Some(format!(
                "locator {path} is {} at status {}, which its type does not declare a success status",
                record_label(cited),
                if status.is_empty() { "(none)" } else { status }
            ));
        }
    }
    None
}

fn evidence_standing(
    item: &Item,
    config: &Config,
    kind: &KindRule<'_>,
    delivery: &DeliveryDecl,
    env: &LocatorEnv<'_>,
) -> EvidenceStanding {
    let Some(clause) = &item.evidence else {
        return EvidenceStanding::None;
    };
    if !clause.valid {
        return EvidenceStanding::Fails {
            cause: "the evidence clause is not entirely locators (see status.done-items-shape)"
                .to_string(),
        };
    }
    let entries = crate::locator::realized_by_locators(&clause.text);
    for entry in &entries {
        if locator_defect(entry, "the evidence clause", env).is_some() {
            let path = entry
                .parsed
                .as_ref()
                .map_or_else(|_| entry.raw.clone(), |l| l.path.clone());
            return EvidenceStanding::Fails {
                cause: format!("locator {path} does not exist"),
            };
        }
    }
    if kind.cited_must_be_decided {
        if let Some(cause) = undecided_record(&entries, env) {
            return EvidenceStanding::Fails { cause };
        }
    }
    let tier = compute_embodiment(
        &config.embodiment,
        &parse_realized_by(&clause.text),
        false,
        false,
        false,
    );
    if delivery.resolving_tiers().iter().any(|t| t == tier) {
        EvidenceStanding::Resolves {
            tier: tier.to_string(),
        }
    } else {
        let paths: Vec<String> = entries
            .iter()
            .map(|e| {
                e.parsed
                    .as_ref()
                    .map_or_else(|_| e.raw.clone(), |l| l.path.clone())
            })
            .collect();
        EvidenceStanding::Fails {
            cause: format!(
                "locator {} resolves only to {tier}, which is not in resolving_tiers",
                paths.join(", ")
            ),
        }
    }
}

/// Judges every item of a record against the tree (`ADR-96` section 2).
///
/// `record` supplies the kind it declares: for a `decision` kind the tiers that
/// count as delivered, and whether the records its items cite must be decided.
pub fn judge_items(
    record: &Record,
    items: &DoneItems,
    config: &Config,
    delivery: &DeliveryDecl,
    env: &LocatorEnv<'_>,
) -> ItemsJudgement {
    let kind = match &delivery.items {
        Some(decl) => kind_rule(record, delivery, decl),
        None => KindRule {
            delivered_tiers: &delivery.delivered_tiers,
            cited_must_be_decided: false,
        },
    };
    let standings: Vec<ItemStanding> = items
        .items
        .iter()
        .map(|item| ItemStanding {
            n: item.n,
            line: item.line,
            text: item.text.clone(),
            evidence: evidence_standing(item, config, &kind, delivery, env),
            waived: item.waiver.clone(),
        })
        .collect();
    let in_list = |list: &[String], v: &str| list.iter().any(|e| e == v);
    let resolved: Vec<&ItemStanding> = standings.iter().filter(|s| s.resolved()).collect();
    let tier_of = |s: &ItemStanding| match &s.evidence {
        EvidenceStanding::Resolves { tier } => tier.clone(),
        _ => String::new(),
    };
    let resolved_delivered = resolved
        .iter()
        .filter(|s| in_list(kind.delivered_tiers, &tier_of(s)))
        .count();
    let unresolved: Vec<UnresolvedItem> = standings
        .iter()
        .filter(|s| s.is_unresolved())
        .map(|s| UnresolvedItem {
            n: s.n,
            line: s.line,
            text: s.text.clone(),
            cause: s.cause(),
        })
        .collect();
    let waived = standings.iter().filter(|s| s.is_waived()).count();
    let tier = resolved
        .iter()
        .map(|s| tier_of(s))
        .max_by_key(|t| tier_rank(&config.embodiment, t))
        .unwrap_or_else(|| config.embodiment.none.clone());
    let standing = if resolved_delivered >= 1 && unresolved.is_empty() {
        Standing::Delivered
    } else if resolved_delivered >= 1 {
        Standing::Partial
    } else if !resolved.is_empty() {
        Standing::SpecifiedOnly
    } else {
        Standing::None
    };
    ItemsJudgement {
        resolved: resolved.len(),
        resolved_delivered,
        waived,
        unresolved,
        tier,
        standing,
        capped_by: Vec::new(),
        items: standings,
    }
}

/// Runs `f` with the working-tree lookups every locator check needs, built the
/// way `embodiment.locator-exists` builds them: a path is present only when git
/// tracks it and it is on disk.
pub fn with_locator_env<R>(ctx: &RuleCtx<'_>, f: impl FnOnce(&LocatorEnv<'_>) -> R) -> R {
    let tracked: HashSet<&Path> = ctx.tracked_paths.iter().map(|p| p.as_path()).collect();
    let present = |p: &str| {
        let path = Path::new(p);
        tracked.contains(path) && (ctx.may_exist)(path)
    };
    let untracked = |p: &str| {
        let path = Path::new(p);
        !tracked.contains(path) && (ctx.may_exist)(path)
    };
    let env = LocatorEnv {
        config: ctx.config,
        opts: ctx.config.rule_opts(RULE_EMBODIMENT_LOCATOR_EXISTS),
        tracked: ctx.tracked_paths,
        present: &present,
        read: ctx.read_text,
        untracked_on_disk: &untracked,
        records: ctx.records,
    };
    f(&env)
}

/// The first declared status that is neither a `success`, a `failure` nor a
/// `progress` status: where a derived-blocked record can be repaired to
/// (`sequence.status-agreement` refuses a `progress` status for it).
pub fn first_non_progress_status(decl: &StatusDecl) -> Option<&str> {
    decl.values
        .iter()
        .find(|v| !decl.is_success(v) && !decl.failure.contains(v) && !decl.is_progress_status(v))
        .map(String::as_str)
}

/// The records whose dependencies are not all finished (`ADR-83`): derived, the
/// way `sequence.status-agreement` derives it, each with the dependencies that
/// are still open, shown as `MILE-171 (InProgress)`. A dependency is unfinished
/// when it is not in a status its own type declares `success`, so one in a
/// `failure` status, one with no `Status` and one whose type declares no
/// `success` all count. Empty without a sequencing pair.
pub fn derived_blocked_records(
    records: &[Record],
    config: &Config,
) -> HashMap<PathBuf, Vec<String>> {
    let mut blocked = HashMap::new();
    if config.roadmap.pairs.is_empty() {
        return blocked;
    }
    let index = build_normalized_index(records);
    let declared = statuses_by_type(config);
    let slots = pair_slots(records, config, |p| config.roadmap.pairs.contains(&p.name));
    let SequenceGraphs { graphs, .. } = sequence_graphs(&slots, &index);
    for record in records {
        let Some(id) = record_id(record) else {
            continue;
        };
        let id = crate::values::RecordId::new(&id);
        let mut open: Vec<String> = Vec::new();
        for graph in graphs.values() {
            for dependency in graph.get(&id).into_iter().flatten() {
                let Some(target) = index.get(dependency) else {
                    continue;
                };
                let field = relation_field_name(
                    config,
                    &target.record_type,
                    crate::config::RelationRole::Status,
                );
                let status = target.header.get(field).map(str::trim);
                let finished = status.is_some_and(|s| {
                    declared
                        .get(&target.record_type)
                        .is_some_and(|d| d.is_success(s))
                });
                if !finished {
                    let shown = format!(
                        "{dependency} ({})",
                        status.filter(|s| !s.is_empty()).unwrap_or("no status")
                    );
                    if !open.contains(&shown) {
                        open.push(shown);
                    }
                }
            }
        }
        if !open.is_empty() {
            blocked.insert(record.path.clone(), open);
        }
    }
    blocked
}

/// `status.done-items-shape`: every deviation from the item grammar of
/// `ADR-96` section 1, on a type that declares `statuses.delivery.items`. A
/// clause that does not parse is a finding, never skipped.
pub fn status_done_items_shape(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_STATUS_DONE_ITEMS_SHAPE;
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| opted_in(&declared, &r.record_type))
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let Some(items) = &record.items else {
                return Outcome::Absent;
            };
            // A record without the section is `section.required`'s to report.
            if items.section_line.is_none() {
                return Outcome::Absent;
            }
            let section = declared[&record.record_type]
                .delivery
                .as_ref()
                .and_then(|d| d.items.as_ref())
                .map(|i| i.section.clone())
                .unwrap_or_default();
            // A kind outside the declared vocabulary judges the record as one
            // with no kind; say so instead of leaving the label inert.
            if let Some(kind) = declared[&record.record_type]
                .delivery
                .as_ref()
                .and_then(|d| d.items.as_ref())
                .and_then(|i| i.kind.as_ref())
            {
                let value = record.header.get(kind.field.as_str()).map(str::trim);
                if let Some(value) = value.filter(|v| !v.is_empty()) {
                    if !kind.kinds.iter().any(|k| k.name == value) {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Error,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new("done-items-shape")
                                .section(section.clone()),
                            message: format!(
                                "{} '{value}' is not a declared kind (one of: {}); the record is judged as one with no kind",
                                kind.field,
                                kind.kinds
                                    .iter()
                                    .map(|k| k.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        });
                    }
                }
            }
            for issue in &items.issues {
                let place = issue.item.map_or(String::new(), |n| format!("item {n}: "));
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: Some(issue.line),
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new("done-items-shape")
                        .section(section.clone())
                        .line(issue.line),
                    message: format!("{place}{}", issue.message),
                });
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// The Done-means section name of a type that declares `delivery.items`, else
/// empty.
pub fn items_section(config: &Config, record_type: &str) -> String {
    config
        .record_types
        .get(record_type)
        .and_then(|t| t.status_decl())
        .and_then(|d| d.delivery)
        .and_then(|d| d.items)
        .map(|i| i.section)
        .unwrap_or_default()
}

fn opted_in(declared: &HashMap<String, StatusDecl>, record_type: &str) -> bool {
    declared
        .get(record_type)
        .and_then(|d| d.delivery.as_ref())
        .is_some_and(|d| d.items.is_some())
}

/// `status.waived-item-stale`: a waived item whose evidence now resolves, whose
/// cited record is missing (or, for a deferral, finished), or whose expiry date
/// has come (`ADR-96` section 3). `today` is empty when the caller has none, and
/// no expiry is then judged.
pub fn status_waived_item_stale(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
    env: &LocatorEnv<'_>,
    today: &str,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_STATUS_WAIVED_ITEM_STALE;
    let declared = statuses_by_type(config);
    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| opted_in(&declared, &r.record_type))
        .collect();
    let mut findings = Vec::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let Some(items) = &record.items else {
                return Outcome::Absent;
            };
            let delivery = declared[&record.record_type]
                .delivery
                .as_ref()
                .expect("opted in");
            let decl = delivery.items.as_ref().expect("opted in");
            let judged = judge_items(record, items, config, delivery, env);
            for standing in &judged.items {
                let Some(waiver) = &standing.waived else {
                    continue;
                };
                let category = decl.category(&waiver.category);
                let mut push = |message: String| {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Warning,
                        file: record.path.clone(),
                        line: Some(waiver.line),
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("waived-item-stale")
                            .section(decl.section.clone())
                            .line(waiver.line),
                        message: format!(
                            "{} {} item {} \"{}\" is waived as {}, and {message}",
                            record_label(record),
                            decl.section,
                            standing.n,
                            standing.text,
                            waiver.category
                        ),
                    });
                };
                if standing.resolved() {
                    push("its evidence now resolves: remove the waiver".to_string());
                }
                if let (Some(cite), Some(category)) = (&waiver.cite, category) {
                    let target = index.get(&crate::values::RecordId::new(cite));
                    match target {
                        None => push(format!("the cited record {cite} does not exist")),
                        Some(target) if category.expires_required() => {
                            let finished = declared.get(&target.record_type).is_some_and(|d| {
                                let field = relation_field_name(
                                    config,
                                    &target.record_type,
                                    crate::config::RelationRole::Status,
                                );
                                target
                                    .header
                                    .get(field)
                                    .map(str::trim)
                                    .is_some_and(|s| d.is_terminal(s))
                            });
                            if finished {
                                push(format!(
                                    "the deferral target {cite} is finished: evidence the item here, or re-point the cite"
                                ));
                            }
                        }
                        Some(_) => {}
                    }
                }
                if let Some(date) = &waiver.date {
                    if !today.is_empty() && date.as_str() <= today {
                        push(format!(
                            "its expiry {date} has come: extend it with a new date, or resolve the item"
                        ));
                    }
                }
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution {
            rule: RULE_ID.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

fn record_label(record: &Record) -> String {
    record_id(record).unwrap_or_else(|| record.path.display().to_string())
}

/// One informational notice per waived item, in record order, and one summary
/// by category: a waived item is never omitted from the report (`ADR-96`
/// section 3).
pub fn waived_item_notices(records: &[Record], config: &Config) -> Vec<Notice> {
    let declared = statuses_by_type(config);
    let mut notices = Vec::new();
    let mut by_category: std::collections::BTreeMap<String, usize> = Default::default();
    let mut sections: std::collections::BTreeSet<String> = Default::default();
    for record in records
        .iter()
        .filter(|r| opted_in(&declared, &r.record_type))
    {
        let Some(items) = &record.items else { continue };
        for item in &items.items {
            let Some(w) = &item.waiver else { continue };
            *by_category.entry(w.category.clone()).or_default() += 1;
            sections.insert(items_section(config, &record.record_type));
            notices.push(Notice {
                severity: NoticeSeverity::Info,
                subject: record.path.display().to_string(),
                message: format!(
                    "{} item {} \"{}\" is waived: {}{} -- {}",
                    items_section(config, &record.record_type),
                    item.n,
                    item.text,
                    w.category,
                    match (&w.cite, &w.date) {
                        (Some(c), Some(d)) => format!(" ({c}, {d})"),
                        (Some(c), None) => format!(" ({c})"),
                        _ => String::new(),
                    },
                    w.reason
                ),
            });
        }
    }
    if !by_category.is_empty() {
        let total: usize = by_category.values().sum();
        notices.push(Notice {
            severity: NoticeSeverity::Info,
            subject: RULE_STATUS_WAIVED_ITEM_STALE.to_string(),
            message: format!(
                "{total} waived {} item(s) by category: {}",
                sections.iter().cloned().collect::<Vec<_>>().join("/"),
                by_category
                    .iter()
                    .map(|(c, n)| format!("{c} {n}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    notices
}
