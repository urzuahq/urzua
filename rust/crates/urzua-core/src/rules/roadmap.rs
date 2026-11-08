//! `urzua roadmap` (`ADR-82`, `MILE-182`, `RFC-48`): the order of work computed
//! from the records alone. One graph over the sequencing pairs `roadmap.pairs`
//! declares, read through the same `sequence_graphs` the checks use, so the
//! roadmap and `check` cannot disagree about an edge. Pure: records and
//! configuration in, one serializable `Roadmap` out. Nothing is stored.
//!
//! Implements: MILE-182

use super::relation::{pair_carried_by, pair_slot_claims, pair_slots};
use super::sequence::{
    merged_graph, sequence_graphs, strongly_connected, SequenceGraph, SequenceGraphs,
};
use super::*;
use crate::status_model::StatusDecl;
use crate::values::RecordId;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// The output schema version (`ADR-82`). A change to a number's meaning or the
/// shape bumps it; adding a key, a note kind or a reason does not.
pub const ROADMAP_SCHEMA: u32 = 1;

/// The natural order of ids (`ADR-82`): the text before a trailing number is
/// compared as text, then the number as a number, then the whole as text for
/// an id with no number. `MILE-5` sorts before `MILE-12`. Group values use the
/// same order, so `Phase` `2` sorts before `10`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    fn key(s: &str) -> (&str, Option<u64>) {
        let digits = s.len() - s.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        if digits == 0 {
            return (s, None);
        }
        let (prefix, number) = s.split_at(s.len() - digits);
        match number.parse::<u64>() {
            Ok(n) => (prefix, Some(n)),
            Err(_) => (s, None),
        }
    }
    let (pa, na) = key(a);
    let (pb, nb) = key(b);
    pa.cmp(pb).then(na.cmp(&nb)).then(a.cmp(b))
}

/// A JSON object whose keys keep the order they were inserted in. The roadmap
/// keys its `records` by id in natural order, which a `BTreeMap<String, _>`
/// cannot express (it sorts as text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedMap<V>(pub Vec<(String, V)>);

impl<V: Serialize> Serialize for OrderedMap<V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<V> OrderedMap<V> {
    pub fn get(&self, key: &str) -> Option<&V> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// The computed roadmap (`ADR-82`): always an object, versioned by `schema`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Roadmap {
    pub schema: u32,
    /// The declared `roadmap.pairs`, in declaration order.
    pub pairs: Vec<String>,
    /// Layer `n` holds the included records whose deepest open dependency chain
    /// has `n` records above them.
    pub waves: Vec<Vec<String>>,
    pub ready: Vec<String>,
    pub critical_path: Vec<String>,
    pub records: OrderedMap<RoadmapRecord>,
    pub groups: Vec<RoadmapGroup>,
    pub needs_replan: Vec<NeedsReplan>,
    pub cycles: Vec<Vec<String>>,
    pub notes: Vec<RoadmapNote>,
    /// The non-work records not in a `success` status (`ADR-82`, Amendment of
    /// 2026-11-05). Present only when `roadmap.work_types` is declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_on: Option<Vec<WaitingOn>>,
}

/// A record of a type `roadmap.work_types` leaves out, still open or failed:
/// a wait that gates the work records listed in `blocks`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaitingOn {
    pub record: String,
    #[serde(rename = "type")]
    pub record_type: String,
    /// The written value, `null` when the record states none.
    pub status: Option<String>,
    /// The work records that transitively depend on it.
    pub blocks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoadmapRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    /// The written value, `null` when the record states none.
    pub status: Option<String>,
    /// `null` for a record a `needs_replan` reason keeps out of the waves.
    pub wave: Option<usize>,
    pub ready: bool,
    /// The derived value (`ADR-83`): some direct dependency is not in a status
    /// its own type declares `success`.
    pub blocked: bool,
    /// The direct dependencies not in a `success` status.
    pub blocked_by: Vec<String>,
    /// Every open record that transitively depends on this one.
    pub blocks: Vec<String>,
    /// The Done-means items of a record whose type declares
    /// `statuses.delivery.items` (`ADR-96` section 7): "3 of 5" without a status
    /// the engine would have to know. Absent for every other record. A
    /// partially delivered record stays open work, in its wave, and its
    /// dependents stay blocked: blocked is derived from its written `Status`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<RoadmapItems>,
}

/// `RoadmapRecord.items`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoadmapItems {
    pub total: usize,
    pub resolved: usize,
    pub waived: usize,
}

/// Fills `items` on each roadmap record whose type declares
/// `statuses.delivery.items`. The roadmap itself reads no file; the caller
/// supplies the working tree the item locators resolve in.
pub fn add_item_counts(
    roadmap: &mut Roadmap,
    records: &[Record],
    config: &Config,
    env: &LocatorEnv<'_>,
) {
    let declared = statuses_by_type(config);
    let by_id: HashMap<String, &Record> = records
        .iter()
        .filter_map(|r| record_id(r).map(|id| (id, r)))
        .collect();
    for (id, entry) in roadmap.records.0.iter_mut() {
        let Some(record) = by_id.get(id) else {
            continue;
        };
        let Some(delivery) = declared
            .get(&record.record_type)
            .and_then(|d| d.delivery.as_ref())
            .filter(|d| d.items.is_some())
        else {
            continue;
        };
        let empty = crate::done_items::DoneItems::default();
        let judged = judge_items(
            record,
            record.items.as_ref().unwrap_or(&empty),
            config,
            delivery,
            env,
        );
        entry.items = Some(RoadmapItems {
            total: judged.total(),
            resolved: judged.resolved,
            waived: judged.waived,
        });
    }
}

/// One bucket of `roadmap.group_by`. A bucket of a non-final field holds
/// `groups`, the final one holds `records`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoadmapGroup {
    pub field: String,
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<RoadmapGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<String>>,
}

/// Why an open record is kept out of the waves, the ready set and the critical
/// path (`ADR-82`). A closed set; a consumer treats an unknown one as
/// informational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReplanReason {
    /// A direct dependency is in a failure status.
    FailedDependency,
    /// A direct dependency is itself kept out, for a failure further down.
    DependsOnExcluded,
    /// The record is a member of a cycle.
    Cycle,
    /// A direct dependency is kept out because of a cycle.
    DependsOnCycle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NeedsReplan {
    pub record: String,
    pub reason: ReplanReason,
    /// The dependency the reason follows; `null` for `cycle`.
    pub via: Option<String>,
}

/// The kinds of note, in the order notes sort by (`ADR-82`). Closed; a consumer
/// treats an unknown kind as informational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoteKind {
    NoSequencingPair,
    PairMatchesNothing,
    UnresolvedClaim,
    UnreadableRecord,
    NoStatusVocabulary,
    UnstructuredWait,
    GroupFieldUnset,
    NothingOpen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoadmapNote {
    pub kind: NoteKind,
    pub record: Option<String>,
    pub pair: Option<String>,
    pub field: Option<String>,
    pub message: String,
}

/// Where a record stands in its own type's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Open,
    Success,
    Failure,
}

fn empty_roadmap(pairs: &[String]) -> Roadmap {
    Roadmap {
        schema: ROADMAP_SCHEMA,
        pairs: pairs.to_vec(),
        waves: Vec::new(),
        ready: Vec::new(),
        critical_path: Vec::new(),
        records: OrderedMap(Vec::new()),
        groups: Vec::new(),
        needs_replan: Vec::new(),
        cycles: Vec::new(),
        notes: Vec::new(),
        waiting_on: None,
    }
}

fn sorted_naturally<'a>(ids: impl IntoIterator<Item = &'a RecordId>) -> Vec<&'a RecordId> {
    let mut ids: Vec<&RecordId> = ids.into_iter().collect();
    ids.sort_by(|a, b| natural_cmp(a.as_str(), b.as_str()));
    ids
}

fn names(ids: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut out: Vec<String> = ids.into_iter().map(|i| i.as_ref().to_string()).collect();
    out.sort_by(|a, b| natural_cmp(a, b));
    out
}

/// The smallest of `ids` in natural order.
fn smallest<'a>(ids: impl IntoIterator<Item = &'a RecordId>) -> Option<&'a RecordId> {
    sorted_naturally(ids).into_iter().next()
}

fn present(value: Option<&str>) -> Option<&str> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty() && !crate::values::is_no_value_sentinel(v))
}

type Reasons = BTreeMap<RecordId, (ReplanReason, Option<RecordId>)>;
type Dependents<'a> = BTreeMap<&'a RecordId, Vec<&'a RecordId>>;

/// The scoped, classified graph every later roadmap step reads.
struct Order<'a> {
    index: &'a RecordIndex<'a>,
    /// X -> {Y} reads "X depends on Y" over every declared pair.
    depends: SequenceGraph,
    scope: BTreeSet<RecordId>,
    standing: BTreeMap<RecordId, Standing>,
    written: BTreeMap<RecordId, Option<String>>,
    open: BTreeSet<RecordId>,
    open_work: BTreeSet<RecordId>,
    work_types: Option<&'a Vec<String>>,
}

/// The note for a configuration that names no sequencing pair.
fn no_pair_note(config: &Config) -> RoadmapNote {
    let message = if config.relation_pairs.iter().any(|p| p.acyclic) {
        let available: Vec<&str> = config
            .relation_pairs
            .iter()
            .filter(|p| p.acyclic)
            .map(|p| p.name.as_str())
            .collect();
        format!(
            "roadmap.pairs names no pair, so there is no roadmap; acyclic pair(s) available to name: {}",
            available.join(", ")
        )
    } else {
        "no relation pair declares `acyclic: true` and roadmap.pairs names none, so there is no sequencing relation and no roadmap".to_string()
    };
    RoadmapNote {
        kind: NoteKind::NoSequencingPair,
        record: None,
        pair: None,
        field: None,
        message,
    }
}

/// Every record a declared pair can say something about is in scope, linked
/// or not: a record whose type declares a side of a pair, and every record
/// an edge reaches. Each gets an entry in the graph.
fn scoped_graph(
    mut depends: SequenceGraph,
    slots: &[super::relation::PairSlot<'_>],
) -> (SequenceGraph, BTreeSet<RecordId>) {
    let mut scope: BTreeSet<RecordId> = depends.keys().cloned().collect();
    for slot in slots {
        if slot.record.header.is_unreadable() {
            continue;
        }
        if let Some(id) = record_id(slot.record) {
            scope.insert(RecordId::new(&id));
        }
    }
    for id in &scope {
        depends.entry(id.clone()).or_default();
    }
    (depends, scope)
}

/// Where one record stands in its own type's lifecycle: the written status,
/// the standing, and why it is read as open when the type gives no answer.
fn standing_of(
    config: &Config,
    index: &RecordIndex<'_>,
    declared: &HashMap<String, StatusDecl>,
    id: &RecordId,
) -> (Option<String>, Standing, Option<&'static str>) {
    let record = index[id];
    let field = relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::Status,
    );
    let status = present(record.header.get(field)).map(str::to_string);
    let decl = declared.get(&record.record_type);
    match (decl, &status) {
        (None, _) => (
            status,
            Standing::Open,
            Some("declares no status vocabulary"),
        ),
        (Some(_), None) => (status, Standing::Open, Some("states no Status")),
        (Some(d), Some(s)) if d.success.iter().any(|v| v == s) => (status, Standing::Success, None),
        (Some(d), Some(s)) if d.failure.iter().any(|v| v == s) => (status, Standing::Failure, None),
        _ => (status, Standing::Open, None),
    }
}

/// The standing and written status of every in-scope record present in the
/// index, with a note for each type that gives no status vocabulary.
fn read_standings(
    config: &Config,
    index: &RecordIndex<'_>,
    declared: &HashMap<String, StatusDecl>,
    scope: &BTreeSet<RecordId>,
    notes: &mut Vec<RoadmapNote>,
) -> (
    BTreeMap<RecordId, Standing>,
    BTreeMap<RecordId, Option<String>>,
) {
    let mut standing: BTreeMap<RecordId, Standing> = BTreeMap::new();
    let mut written: BTreeMap<RecordId, Option<String>> = BTreeMap::new();
    for id in scope {
        let Some(record) = index.get(id) else {
            continue;
        };
        let (status, state, why) = standing_of(config, index, declared, id);
        if let Some(why) = why {
            let field = relation_field_name(
                config,
                &record.record_type,
                crate::config::RelationRole::Status,
            );
            notes.push(RoadmapNote {
                kind: NoteKind::NoStatusVocabulary,
                record: Some(id.as_str().to_string()),
                pair: None,
                field: Some(field.to_string()),
                message: format!(
                    "{id}: type '{}' {why}, so it is read as open",
                    record.record_type
                ),
            });
        }
        standing.insert(id.clone(), state);
        written.insert(id.clone(), status);
    }
    (standing, written)
}

/// Cycles: the strongly connected components of the whole declared graph that
/// loop, each in natural order, and the set of their members.
fn find_cycles(depends: &SequenceGraph) -> (Vec<Vec<RecordId>>, BTreeSet<RecordId>) {
    let mut cycles: Vec<Vec<RecordId>> = Vec::new();
    let mut in_cycle: BTreeSet<RecordId> = BTreeSet::new();
    for mut component in strongly_connected(depends) {
        let looped = component.len() > 1
            || depends
                .get(&component[0])
                .is_some_and(|next| next.contains(&component[0]));
        if looped {
            component.sort_by(|a, b| natural_cmp(a.as_str(), b.as_str()));
            in_cycle.extend(component.iter().cloned());
            cycles.push(component);
        }
    }
    cycles.sort_by(|a, b| natural_cmp(a[0].as_str(), b[0].as_str()));
    (cycles, in_cycle)
}

impl<'a> Order<'a> {
    fn new(
        config: &'a Config,
        index: &'a RecordIndex<'a>,
        depends: SequenceGraph,
        standing: BTreeMap<RecordId, Standing>,
        written: BTreeMap<RecordId, Option<String>>,
    ) -> Self {
        let scope: BTreeSet<RecordId> = standing.keys().cloned().collect();
        let open: BTreeSet<RecordId> = scope
            .iter()
            .filter(|id| standing.get(*id).copied().unwrap_or(Standing::Success) == Standing::Open)
            .cloned()
            .collect();
        let mut order = Order {
            index,
            depends,
            scope,
            standing,
            written,
            open,
            open_work: BTreeSet::new(),
            work_types: config.roadmap.work_types.as_ref(),
        };
        // The records the roadmap orders (`ADR-82`, Amendment of 2026-11-05):
        // every type when `work_types` is absent. A non-work record stays in the
        // graph and gates its dependents; it is never a subject of the output.
        order.open_work = order
            .open
            .iter()
            .filter(|id| order.is_work(id))
            .cloned()
            .collect();
        order
    }

    fn state(&self, id: &RecordId) -> Standing {
        self.standing.get(id).copied().unwrap_or(Standing::Success)
    }

    fn is_work(&self, id: &RecordId) -> bool {
        self.work_types
            .is_none_or(|types| types.contains(&self.index[id].record_type))
    }

    fn deps_of(&self, id: &RecordId) -> &BTreeSet<RecordId> {
        &self.depends[id]
    }

    /// Exclusion: a cycle, a failed dependency, or a dependency kept out.
    /// Returns the reason for each kept-out open record and the kept-out set.
    fn exclusions(&self, in_cycle: &BTreeSet<RecordId>) -> (Reasons, BTreeSet<RecordId>) {
        let mut reason: Reasons = BTreeMap::new();
        for id in &self.open {
            if in_cycle.contains(id) {
                reason.insert(id.clone(), (ReplanReason::Cycle, None));
            } else if let Some(via) = smallest(
                self.deps_of(id)
                    .iter()
                    .filter(|d| self.state(d) == Standing::Failure),
            ) {
                reason.insert(
                    id.clone(),
                    (ReplanReason::FailedDependency, Some(via.clone())),
                );
            }
        }
        // The least fixed point: an open record with a dependency kept out is kept out.
        let mut excluded: BTreeSet<RecordId> = reason.keys().cloned().collect();
        loop {
            let more: Vec<RecordId> = self
                .open
                .iter()
                .filter(|id| {
                    !excluded.contains(*id) && self.deps_of(id).iter().any(|d| excluded.contains(d))
                })
                .cloned()
                .collect();
            if more.is_empty() {
                break;
            }
            excluded.extend(more);
        }
        self.indirect_reasons(&excluded, &mut reason);
        (reason, excluded)
    }

    /// Reasons for the indirect exclusions, each after every dependency it reads.
    fn indirect_reasons(&self, excluded: &BTreeSet<RecordId>, reason: &mut Reasons) {
        let mut pending: Vec<RecordId> = excluded
            .iter()
            .filter(|id| !reason.contains_key(*id))
            .cloned()
            .collect();
        while !pending.is_empty() {
            let (ready_now, rest): (Vec<RecordId>, Vec<RecordId>) =
                pending.into_iter().partition(|id| {
                    self.deps_of(id)
                        .iter()
                        .filter(|d| excluded.contains(*d))
                        .all(|d| reason.contains_key(d))
                });
            // A dependency graph with no cycle among these cannot stall; if it
            // ever did, stop rather than loop.
            if ready_now.is_empty() {
                for id in rest {
                    let via = smallest(self.deps_of(&id).iter().filter(|d| excluded.contains(*d)))
                        .cloned();
                    reason.insert(id, (ReplanReason::DependsOnExcluded, via));
                }
                break;
            }
            for id in ready_now {
                let kept_out: Vec<&RecordId> = self
                    .deps_of(&id)
                    .iter()
                    .filter(|d| excluded.contains(*d))
                    .collect();
                let cyclic: Vec<&RecordId> = kept_out
                    .iter()
                    .copied()
                    .filter(|d| {
                        matches!(
                            reason[*d].0,
                            ReplanReason::Cycle | ReplanReason::DependsOnCycle
                        )
                    })
                    .collect();
                let entry = if cyclic.is_empty() {
                    (ReplanReason::DependsOnExcluded, smallest(kept_out).cloned())
                } else {
                    (ReplanReason::DependsOnCycle, smallest(cyclic).cloned())
                };
                reason.insert(id, entry);
            }
            pending = rest;
        }
    }

    /// The work records at the end of the chains of non-work records `edges`
    /// leads through. The edges waves and the critical path read: a work
    /// record's dependencies contract through chains of non-work records,
    /// whatever their status, so a non-work record in between does not cut the
    /// ordering. Gating is not contracted: a non-work record's own status is
    /// read from `blocked_by` and `reason`.
    fn reach(&self, from: &RecordId, edges: &dyn Fn(&RecordId) -> Vec<RecordId>) -> Vec<RecordId> {
        let mut found: BTreeSet<RecordId> = BTreeSet::new();
        let mut seen: BTreeSet<RecordId> = BTreeSet::new();
        let mut frontier = vec![from.clone()];
        while let Some(at) = frontier.pop() {
            for next in edges(&at) {
                if next == *from || !seen.insert(next.clone()) {
                    continue;
                }
                if self.is_work(&next) {
                    found.insert(next);
                } else {
                    frontier.push(next);
                }
            }
        }
        found.into_iter().collect()
    }

    fn work_deps_of(&self, id: &RecordId) -> Vec<RecordId> {
        self.reach(id, &|at| self.deps_of(at).iter().cloned().collect())
    }

    /// Waves: 1 + the deepest open dependency; a success dependency is ignored.
    fn assign_waves(&self, included: &BTreeSet<RecordId>) -> BTreeMap<RecordId, usize> {
        let mut wave: BTreeMap<RecordId, usize> = BTreeMap::new();
        let mut waiting: Vec<&RecordId> = included.iter().collect();
        while !waiting.is_empty() {
            let mut rest = Vec::new();
            for id in waiting.iter().copied() {
                // A dependency kept out is not ordered; the record behind it is
                // kept out too unless a non-work record in between is finished.
                let open_deps: Vec<RecordId> = self
                    .work_deps_of(id)
                    .into_iter()
                    .filter(|d| self.state(d) == Standing::Open && included.contains(d))
                    .collect();
                if open_deps.iter().all(|d| wave.contains_key(d)) {
                    let w = open_deps.iter().map(|d| wave[d] + 1).max().unwrap_or(0);
                    wave.insert(id.clone(), w);
                } else {
                    rest.push(id);
                }
            }
            if rest.len() == waiting.len() {
                break;
            }
            waiting = rest;
        }
        wave
    }

    /// Dependents of open records, and of every non-work record (a finished one
    /// still sits between the work records it orders).
    fn dependents(&self) -> Dependents<'_> {
        let mut dependents: Dependents<'_> = BTreeMap::new();
        for id in self
            .scope
            .iter()
            .filter(|id| self.open.contains(*id) || !self.is_work(id))
        {
            for d in self.deps_of(id) {
                dependents.entry(d).or_default().push(id);
            }
        }
        dependents
    }

    /// Every open work record that transitively depends on `id`.
    fn blocked_work(&self, dependents: &Dependents<'_>, id: &RecordId) -> Vec<String> {
        let mut seen: BTreeSet<&RecordId> = BTreeSet::new();
        let mut frontier = vec![id];
        while let Some(at) = frontier.pop() {
            for next in dependents.get(at).into_iter().flatten().copied() {
                if next != id && seen.insert(next) {
                    frontier.push(next);
                }
            }
        }
        names(
            seen.into_iter()
                .filter(|n| self.open_work.contains(*n))
                .map(RecordId::as_str),
        )
    }

    /// Per record: blocked_by, blocks, ready.
    fn record_entries(
        &self,
        included: &BTreeSet<RecordId>,
        wave: &BTreeMap<RecordId, usize>,
        dependents: &Dependents<'_>,
    ) -> (Vec<(String, RoadmapRecord)>, Vec<String>) {
        let mut entries: Vec<(String, RoadmapRecord)> = Vec::new();
        let mut ready: Vec<String> = Vec::new();
        for id in sorted_naturally(self.open_work.iter()) {
            let blocked_by = names(
                self.deps_of(id)
                    .iter()
                    .filter(|d| self.state(d) != Standing::Success)
                    .map(RecordId::as_str),
            );
            let blocked = !blocked_by.is_empty();
            // Ready is wave 0 and unblocked. With every type work that is just
            // unblocked; a work record behind open work through a finished
            // non-work record is unblocked but in a later wave, so not ready.
            let is_ready = included.contains(id) && !blocked && wave.get(id) == Some(&0);
            if is_ready {
                ready.push(id.as_str().to_string());
            }
            entries.push((
                id.as_str().to_string(),
                RoadmapRecord {
                    record_type: self.index[id].record_type.clone(),
                    status: self.written.get(id).cloned().flatten(),
                    wave: wave.get(id).copied(),
                    ready: is_ready,
                    blocked,
                    blocked_by,
                    blocks: self.blocked_work(dependents, id),
                    items: None,
                },
            ));
        }
        (entries, ready)
    }

    /// The included work records that depend on `id`, through any chain of
    /// non-work records.
    fn next_of<'i>(
        &self,
        dependents: &Dependents<'_>,
        included: &'i BTreeSet<RecordId>,
        id: &RecordId,
    ) -> Vec<&'i RecordId> {
        self.reach(id, &|at| {
            dependents
                .get(at)
                .into_iter()
                .flatten()
                .map(|n| (*n).clone())
                .collect()
        })
        .into_iter()
        .filter_map(|n| included.get(&n))
        .collect()
    }

    /// The critical path: the longest chain of included records, and among the
    /// longest the lexicographically smallest id sequence, blocker first. Empty
    /// when nothing has an included dependency (maintainer decision, 2026-11-05).
    fn critical_path(
        &self,
        included: &BTreeSet<RecordId>,
        wave: &BTreeMap<RecordId, usize>,
        dependents: &Dependents<'_>,
    ) -> Vec<String> {
        let mut longest: BTreeMap<&RecordId, usize> = BTreeMap::new();
        let mut by_wave: Vec<&RecordId> = included.iter().collect();
        by_wave.sort_by_key(|id| std::cmp::Reverse(wave[*id]));
        for id in by_wave {
            let tail = self
                .next_of(dependents, included, id)
                .iter()
                .map(|n| longest[n])
                .max()
                .unwrap_or(0);
            longest.insert(id, tail + 1);
        }
        let mut critical_path: Vec<String> = Vec::new();
        // A chain needs an edge: when no included record has an included
        // dependency every record is a chain of one, and naming one of them would
        // present an arbitrary id as critical, so the path is empty.
        if let Some(top) = longest.values().max().copied().filter(|top| *top > 1) {
            let mut at = smallest(
                longest
                    .iter()
                    .filter(|(_, l)| **l == top)
                    .map(|(id, _)| *id),
            );
            while let Some(id) = at {
                critical_path.push(id.as_str().to_string());
                let want = longest[id] - 1;
                at = smallest(
                    self.next_of(dependents, included, id)
                        .into_iter()
                        .filter(|n| longest[n] == want),
                );
            }
        }
        critical_path
    }

    /// The `group_by` buckets of the open work, with a note for each field no
    /// open record carries.
    fn groups(&self, config: &Config, notes: &mut Vec<RoadmapNote>) -> Vec<RoadmapGroup> {
        let fields = &config.roadmap.group_by;
        let value_of = |id: &RecordId, field: &str| -> Option<String> {
            present(self.index[id].header.get(field)).map(str::to_string)
        };
        let all: Vec<&RecordId> = self.open_work.iter().collect();
        let groups = if fields.is_empty() {
            Vec::new()
        } else {
            group(&all, fields, &value_of)
        };
        for field in fields {
            if !self
                .open_work
                .iter()
                .any(|id| value_of(id, field).is_some())
            {
                notes.push(RoadmapNote {
                    kind: NoteKind::GroupFieldUnset,
                    record: None,
                    pair: None,
                    field: Some(field.clone()),
                    message: format!(
                        "no open record carries a value for group_by field '{field}', so every record is in its null bucket"
                    ),
                });
            }
        }
        groups
    }

    /// The open work records kept out of the waves, with why.
    fn needs_replan(&self, reason: &Reasons) -> Vec<NeedsReplan> {
        let mut needs_replan: Vec<NeedsReplan> = reason
            .iter()
            .filter(|(id, _)| self.open_work.contains(*id))
            .map(|(id, (why, via))| NeedsReplan {
                record: id.as_str().to_string(),
                reason: *why,
                via: via.as_ref().map(|v| v.as_str().to_string()),
            })
            .collect();
        needs_replan.sort_by(|a, b| natural_cmp(&a.record, &b.record));
        needs_replan
    }

    /// The waits the work is gated on. Absent unless `work_types` is declared.
    fn waiting_on(&self, dependents: &Dependents<'_>) -> Option<Vec<WaitingOn>> {
        self.work_types?;
        Some(
            sorted_naturally(
                self.scope
                    .iter()
                    .filter(|id| !self.is_work(id) && self.state(id) != Standing::Success),
            )
            .into_iter()
            .map(|id| WaitingOn {
                record: id.as_str().to_string(),
                record_type: self.index[id].record_type.clone(),
                status: self.written.get(id).cloned().flatten(),
                blocks: self.blocked_work(dependents, id),
            })
            .collect(),
        )
    }
}

/// Notes that read the open set, then every note sorted and de-duplicated.
fn finish_notes(
    order: &Order<'_>,
    config: &Config,
    slots: &[super::relation::PairSlot<'_>],
    notes: &mut Vec<RoadmapNote>,
) {
    if order.open_work.is_empty() {
        notes.push(RoadmapNote {
            kind: NoteKind::NothingOpen,
            record: None,
            pair: None,
            field: None,
            message:
                "every record in scope is finished (or none is in scope): there is nothing to order"
                    .to_string(),
        });
    }
    unstructured_waits(&order.open_work, order.index, config, slots, notes);
    notes.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| cmp_opt(&a.record, &b.record, natural_cmp))
            .then_with(|| cmp_opt(&a.pair, &b.pair, |x, y| x.cmp(y)))
            .then_with(|| a.field.cmp(&b.field))
            .then_with(|| a.message.cmp(&b.message))
    });
    notes.dedup();
}

/// Compute the roadmap (`ADR-82`) from the loaded records and configuration.
pub fn compute_roadmap(records: &[Record], config: &Config) -> Roadmap {
    let pairs = config.roadmap.pairs.clone();
    let mut roadmap = empty_roadmap(&pairs);
    if pairs.is_empty() {
        roadmap.notes.push(no_pair_note(config));
        return roadmap;
    }

    let index = build_normalized_index(records);
    let declared = statuses_by_type(config);
    let slots = pair_slots(records, config, |p| pairs.contains(&p.name));
    let SequenceGraphs { graphs, .. } = sequence_graphs(&slots, &index);
    let mut notes: Vec<RoadmapNote> = Vec::new();

    let (depends, scope) = scoped_graph(merged_graph(graphs.values()), &slots);
    let (standing, written) = read_standings(config, &index, &declared, &scope, &mut notes);
    let order = Order::new(config, &index, depends, standing, written);
    notes_about_claims(&slots, &index, &pairs, &mut notes);

    let (cycles, in_cycle) = find_cycles(&order.depends);
    let (reason, excluded) = order.exclusions(&in_cycle);
    let included: BTreeSet<RecordId> = order.open_work.difference(&excluded).cloned().collect();

    let wave = order.assign_waves(&included);
    let depth = wave.values().max().map_or(0, |m| m + 1);
    let mut waves: Vec<Vec<String>> = vec![Vec::new(); depth];
    for (id, w) in &wave {
        waves[*w].push(id.as_str().to_string());
    }
    for layer in &mut waves {
        layer.sort_by(|a, b| natural_cmp(a, b));
    }

    let dependents = order.dependents();
    let (entries, ready) = order.record_entries(&included, &wave, &dependents);
    let critical_path = order.critical_path(&included, &wave, &dependents);
    let groups = order.groups(config, &mut notes);
    finish_notes(&order, config, &slots, &mut notes);

    roadmap.waves = waves;
    roadmap.ready = ready;
    roadmap.critical_path = critical_path;
    roadmap.records = OrderedMap(entries);
    roadmap.groups = groups;
    roadmap.needs_replan = order.needs_replan(&reason);
    roadmap.cycles = cycles
        .into_iter()
        .map(|c| c.iter().map(|i| i.as_str().to_string()).collect())
        .collect();
    roadmap.notes = notes;
    roadmap.waiting_on = order.waiting_on(&dependents);
    roadmap
}

/// `None` sorts first.
fn cmp_opt(
    a: &Option<String>,
    b: &Option<String>,
    by: impl Fn(&str, &str) -> Ordering,
) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(x), Some(y)) => by(x, y),
    }
}

/// Nested buckets in `group_by` order. Values sort naturally, the null bucket
/// last; the final field's bucket lists its records, the others hold groups.
fn group(
    ids: &[&RecordId],
    fields: &[String],
    value_of: &dyn Fn(&RecordId, &str) -> Option<String>,
) -> Vec<RoadmapGroup> {
    let Some((field, rest)) = fields.split_first() else {
        return Vec::new();
    };
    let mut buckets: BTreeMap<Option<String>, Vec<&RecordId>> = BTreeMap::new();
    for id in ids {
        buckets.entry(value_of(id, field)).or_default().push(id);
    }
    let mut keys: Vec<Option<String>> = buckets.keys().cloned().collect();
    keys.sort_by(|a, b| match (a, b) {
        (Some(x), Some(y)) => natural_cmp(x, y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    });
    keys.into_iter()
        .map(|value| {
            let members = &buckets[&value];
            if rest.is_empty() {
                RoadmapGroup {
                    field: field.clone(),
                    value,
                    groups: None,
                    records: Some(names(members.iter().map(|i| i.as_str()))),
                }
            } else {
                RoadmapGroup {
                    field: field.clone(),
                    value,
                    groups: Some(group(members, rest, value_of)),
                    records: None,
                }
            }
        })
        .collect()
}

/// Notes about what a declared pair could not say: a pair no record carries a
/// value for, a claim that resolves to nothing the pair carries, and a claim
/// or a record whose header cannot be read (`ADR-82`). An empty result never
/// stands for "nothing declared matched".
fn notes_about_claims(
    slots: &[super::relation::PairSlot<'_>],
    index: &RecordIndex,
    pairs: &[String],
    notes: &mut Vec<RoadmapNote>,
) {
    let mut valued: BTreeSet<&str> = BTreeSet::new();
    for slot in slots {
        let label =
            record_id(slot.record).unwrap_or_else(|| slot.record.path.display().to_string());
        if slot.record.header.is_unreadable() {
            notes.push(RoadmapNote {
                kind: NoteKind::UnreadableRecord,
                record: Some(label.clone()),
                pair: Some(slot.pair.name.clone()),
                field: Some(slot.own_field().to_string()),
                message: format!(
                    "{label}: the header cannot be read, so its claims and any record that depends on it through '{}' are not in this output",
                    slot.own_field()
                ),
            });
            continue;
        }
        if pair_slot_claims(slot).is_err()
            && present(slot.record.header.get(slot.own_field())).is_none()
        {
            continue;
        }
        let Some(value) = present(slot.record.header.get(slot.own_field())) else {
            continue;
        };
        valued.insert(slot.pair.name.as_str());
        for entry in value.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let token = entry.split_whitespace().next().unwrap_or(entry);
            let message = match index.get(&RecordId::new(token)) {
                None => Some((
                    NoteKind::UnresolvedClaim,
                    format!(
                        "{label}: {} '{entry}' names no record, so it orders nothing",
                        slot.own_field()
                    ),
                )),
                Some(target) if !pair_carried_by(slot.pair, &target.record_type) => Some((
                    NoteKind::UnresolvedClaim,
                    format!(
                        "{label}: {} '{entry}' names a {} record, which pair '{}' does not carry, so it orders nothing",
                        slot.own_field(),
                        target.record_type,
                        slot.pair.name
                    ),
                )),
                Some(target) if target.header.is_unreadable() => Some((
                    NoteKind::UnreadableRecord,
                    format!(
                        "{label}: {} names {token}, whose header cannot be read, so the edge is left out",
                        slot.own_field()
                    ),
                )),
                Some(_) => None,
            };
            if let Some((kind, message)) = message {
                notes.push(RoadmapNote {
                    kind,
                    record: Some(label.clone()),
                    pair: Some(slot.pair.name.clone()),
                    field: Some(slot.own_field().to_string()),
                    message,
                });
            }
        }
    }
    for name in pairs {
        if !valued.contains(name.as_str()) {
            notes.push(RoadmapNote {
                kind: NoteKind::PairMatchesNothing,
                record: None,
                pair: Some(name.clone()),
                field: None,
                message: format!(
                    "no record carries a value for pair '{name}', so it orders nothing"
                ),
            });
        }
    }
}

/// An open record with a narrative wait and no sequencing claim: the wait is
/// real and the roadmap cannot see it (`ADR-82`, until the `condition` record
/// type exists). The narrative fields are the type's own declaration.
fn unstructured_waits(
    open: &BTreeSet<RecordId>,
    index: &RecordIndex,
    config: &Config,
    slots: &[super::relation::PairSlot<'_>],
    notes: &mut Vec<RoadmapNote>,
) {
    for id in open {
        let record = index[id];
        if record.header.is_unreadable() {
            continue;
        }
        let claims_order = slots.iter().any(|s| {
            std::ptr::eq(s.record, record)
                && s.forward
                && present(record.header.get(s.own_field())).is_some()
        });
        if claims_order {
            continue;
        }
        let Some(type_config) = config.record_types.get(&record.record_type) else {
            continue;
        };
        let decl = type_config.status_decl();
        for field in type_config.narrative_fields.iter().flatten() {
            let value = record.header.get(field);
            if classify_with(value, decl.as_ref()) != FieldState::Present
                || present(value).is_none()
            {
                continue;
            }
            notes.push(RoadmapNote {
                kind: NoteKind::UnstructuredWait,
                record: Some(id.as_str().to_string()),
                pair: None,
                field: Some(field.clone()),
                message: format!(
                    "{id} is open and states a wait in {field} but has no sequencing claim, so the roadmap cannot order it"
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut ids: Vec<&str>) -> Vec<&str> {
        ids.sort_by(|a, b| natural_cmp(a, b));
        ids
    }

    #[test]
    fn the_number_is_compared_as_a_number_after_the_prefix_as_text() {
        assert_eq!(
            sorted(vec!["MILE-12", "ADR-100", "MILE-5", "ADR-9", "MILE-100"]),
            ["ADR-9", "ADR-100", "MILE-5", "MILE-12", "MILE-100"]
        );
    }

    #[test]
    fn an_id_with_no_number_falls_back_to_text() {
        assert_eq!(
            sorted(vec!["beta", "alpha", "x-2", "x-10"]),
            ["alpha", "beta", "x-2", "x-10"]
        );
        assert_eq!(natural_cmp("a", "a"), Ordering::Equal);
    }

    #[test]
    fn group_values_sort_by_the_same_order() {
        assert_eq!(sorted(vec!["10", "2", "0", "1"]), ["0", "1", "2", "10"]);
    }
}
