//! `urzua roadmap` (`ADR-82`, `MILE-182`, `RFC-48`): the order of work computed
//! from the records alone. One graph over the sequencing pairs `roadmap.pairs`
//! declares, read through the same `sequence_graphs` the checks use, so the
//! roadmap and `check` cannot disagree about an edge. Pure: records and
//! configuration in, one serializable `Roadmap` out. Nothing is stored.
//!
//! Implements: MILE-182

use super::relation::{pair_carried_by, pair_slot_claims, pair_slots};
use super::sequence::{sequence_graphs, strongly_connected, SequenceGraphs};
use super::*;
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

/// Compute the roadmap (`ADR-82`) from the loaded records and configuration.
pub fn compute_roadmap(records: &[Record], config: &Config) -> Roadmap {
    let pairs = config.roadmap.pairs.clone();
    let mut roadmap = empty_roadmap(&pairs);
    if pairs.is_empty() {
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
        roadmap.notes.push(RoadmapNote {
            kind: NoteKind::NoSequencingPair,
            record: None,
            pair: None,
            field: None,
            message,
        });
        return roadmap;
    }

    let index = build_normalized_index(records);
    let declared = statuses_by_type(config);
    let slots = pair_slots(records, config, |p| pairs.contains(&p.name));
    let SequenceGraphs { graphs, .. } = sequence_graphs(&slots, &index);
    let mut notes: Vec<RoadmapNote> = Vec::new();

    // One graph: X -> {Y} reads "X depends on Y" over every declared pair.
    let mut depends: BTreeMap<RecordId, BTreeSet<RecordId>> = BTreeMap::new();
    for graph in graphs.values() {
        for (from, tos) in graph {
            depends.entry(from.clone()).or_default();
            for to in tos {
                depends.entry(to.clone()).or_default();
                depends
                    .get_mut(from)
                    .expect("just inserted")
                    .insert(to.clone());
            }
        }
    }
    // Every record a declared pair can say something about is in scope, linked
    // or not: a record whose type declares a side of a pair, and every record
    // an edge reaches.
    let mut scope: BTreeSet<RecordId> = depends.keys().cloned().collect();
    for slot in &slots {
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

    let status_of = |id: &RecordId| -> (Option<String>, Standing, Option<&'static str>) {
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
            (Some(d), Some(s)) if d.success.iter().any(|v| v == s) => {
                (status, Standing::Success, None)
            }
            (Some(d), Some(s)) if d.failure.iter().any(|v| v == s) => {
                (status, Standing::Failure, None)
            }
            _ => (status, Standing::Open, None),
        }
    };

    let mut standing: BTreeMap<RecordId, Standing> = BTreeMap::new();
    let mut written: BTreeMap<RecordId, Option<String>> = BTreeMap::new();
    for id in &scope {
        let Some(record) = index.get(id) else {
            continue;
        };
        let (status, state, why) = status_of(id);
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
    let scope: BTreeSet<RecordId> = standing.keys().cloned().collect();
    let state = |id: &RecordId| standing.get(id).copied().unwrap_or(Standing::Success);
    let open: BTreeSet<RecordId> = scope
        .iter()
        .filter(|id| state(id) == Standing::Open)
        .cloned()
        .collect();

    notes_about_claims(&slots, &index, &pairs, &mut notes);

    // ---- cycles: strongly connected components of the whole declared graph.
    let mut cycles: Vec<Vec<RecordId>> = Vec::new();
    let mut in_cycle: BTreeSet<RecordId> = BTreeSet::new();
    for mut component in strongly_connected(&depends) {
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

    // ---- exclusion: a cycle, a failed dependency, or a dependency kept out.
    let deps_of = |id: &RecordId| -> &BTreeSet<RecordId> { &depends[id] };
    let mut reason: BTreeMap<RecordId, (ReplanReason, Option<RecordId>)> = BTreeMap::new();
    for id in &open {
        if in_cycle.contains(id) {
            reason.insert(id.clone(), (ReplanReason::Cycle, None));
        } else if let Some(via) =
            smallest(deps_of(id).iter().filter(|d| state(d) == Standing::Failure))
        {
            reason.insert(
                id.clone(),
                (ReplanReason::FailedDependency, Some(via.clone())),
            );
        }
    }
    // The least fixed point: an open record with a dependency kept out is kept out.
    let mut excluded: BTreeSet<RecordId> = reason.keys().cloned().collect();
    loop {
        let more: Vec<RecordId> = open
            .iter()
            .filter(|id| {
                !excluded.contains(*id) && deps_of(id).iter().any(|d| excluded.contains(d))
            })
            .cloned()
            .collect();
        if more.is_empty() {
            break;
        }
        excluded.extend(more);
    }
    // Reasons for the indirect ones, each after every dependency it reads.
    let mut pending: Vec<RecordId> = excluded
        .iter()
        .filter(|id| !reason.contains_key(*id))
        .cloned()
        .collect();
    while !pending.is_empty() {
        let (ready_now, rest): (Vec<RecordId>, Vec<RecordId>) =
            pending.into_iter().partition(|id| {
                deps_of(id)
                    .iter()
                    .filter(|d| excluded.contains(*d))
                    .all(|d| reason.contains_key(d))
            });
        // A dependency graph with no cycle among these cannot stall; if it
        // ever did, stop rather than loop.
        if ready_now.is_empty() {
            for id in rest {
                let via = smallest(deps_of(&id).iter().filter(|d| excluded.contains(*d))).cloned();
                reason.insert(id, (ReplanReason::DependsOnExcluded, via));
            }
            break;
        }
        for id in ready_now {
            let kept_out: Vec<&RecordId> = deps_of(&id)
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
    let included: BTreeSet<RecordId> = open.difference(&excluded).cloned().collect();

    // ---- waves: 1 + the deepest open dependency; a success dependency is ignored.
    let mut wave: BTreeMap<RecordId, usize> = BTreeMap::new();
    let mut waiting: Vec<&RecordId> = included.iter().collect();
    while !waiting.is_empty() {
        let mut rest = Vec::new();
        for id in waiting.iter().copied() {
            let open_deps: Vec<&RecordId> = deps_of(id)
                .iter()
                .filter(|d| state(d) == Standing::Open)
                .collect();
            if open_deps.iter().all(|d| wave.contains_key(*d)) {
                let w = open_deps.iter().map(|d| wave[*d] + 1).max().unwrap_or(0);
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
    let depth = wave.values().max().map_or(0, |m| m + 1);
    let mut waves: Vec<Vec<String>> = vec![Vec::new(); depth];
    for (id, w) in &wave {
        waves[*w].push(id.as_str().to_string());
    }
    for layer in &mut waves {
        layer.sort_by(|a, b| natural_cmp(a, b));
    }

    // ---- per record: blocked_by, blocks, ready.
    let mut dependents: BTreeMap<&RecordId, Vec<&RecordId>> = BTreeMap::new();
    for id in &open {
        for d in deps_of(id) {
            dependents.entry(d).or_default().push(id);
        }
    }
    let mut entries: Vec<(String, RoadmapRecord)> = Vec::new();
    let mut ready: Vec<String> = Vec::new();
    for id in sorted_naturally(open.iter()) {
        let blocked_by = names(
            deps_of(id)
                .iter()
                .filter(|d| state(d) != Standing::Success)
                .map(RecordId::as_str),
        );
        let blocked = !blocked_by.is_empty();
        let is_ready = included.contains(id) && !blocked;
        let mut seen: BTreeSet<&RecordId> = BTreeSet::new();
        let mut frontier = vec![id];
        while let Some(at) = frontier.pop() {
            for next in dependents.get(at).into_iter().flatten().copied() {
                if next != id && seen.insert(next) {
                    frontier.push(next);
                }
            }
        }
        if is_ready {
            ready.push(id.as_str().to_string());
        }
        entries.push((
            id.as_str().to_string(),
            RoadmapRecord {
                record_type: index[id].record_type.clone(),
                status: written.get(id).cloned().flatten(),
                wave: wave.get(id).copied(),
                ready: is_ready,
                blocked,
                blocked_by,
                blocks: names(seen.into_iter().map(RecordId::as_str)),
            },
        ));
    }

    // ---- critical path: the longest chain of included records, and among the
    // longest the lexicographically smallest id sequence, blocker first.
    let mut longest: BTreeMap<&RecordId, usize> = BTreeMap::new();
    let mut by_wave: Vec<&RecordId> = included.iter().collect();
    by_wave.sort_by_key(|id| std::cmp::Reverse(wave[*id]));
    let next_of = |id: &RecordId| -> Vec<&RecordId> {
        dependents
            .get(id)
            .map(|v| {
                v.iter()
                    .copied()
                    .filter(|n| included.contains(*n))
                    .collect()
            })
            .unwrap_or_default()
    };
    for id in by_wave {
        let tail = next_of(id).iter().map(|n| longest[n]).max().unwrap_or(0);
        longest.insert(id, tail + 1);
    }
    let mut critical_path: Vec<String> = Vec::new();
    if let Some(top) = longest.values().max().copied() {
        let mut at = smallest(
            longest
                .iter()
                .filter(|(_, l)| **l == top)
                .map(|(id, _)| *id),
        );
        while let Some(id) = at {
            critical_path.push(id.as_str().to_string());
            let want = longest[id] - 1;
            at = smallest(next_of(id).into_iter().filter(|n| longest[n] == want));
        }
    }

    // ---- groups.
    let fields = &config.roadmap.group_by;
    let value_of = |id: &RecordId, field: &str| -> Option<String> {
        present(index[id].header.get(field)).map(str::to_string)
    };
    let all: Vec<&RecordId> = open.iter().collect();
    let groups = if fields.is_empty() {
        Vec::new()
    } else {
        group(&all, fields, &value_of)
    };
    for field in fields {
        if !open.iter().any(|id| value_of(id, field).is_some()) {
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

    // ---- notes that read the open set.
    if open.is_empty() {
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
    unstructured_waits(&open, &index, config, &slots, &mut notes);

    notes.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| cmp_opt(&a.record, &b.record, natural_cmp))
            .then_with(|| cmp_opt(&a.pair, &b.pair, |x, y| x.cmp(y)))
            .then_with(|| a.field.cmp(&b.field))
            .then_with(|| a.message.cmp(&b.message))
    });
    notes.dedup();

    let mut needs_replan: Vec<NeedsReplan> = reason
        .iter()
        .map(|(id, (why, via))| NeedsReplan {
            record: id.as_str().to_string(),
            reason: *why,
            via: via.as_ref().map(|v| v.as_str().to_string()),
        })
        .collect();
    needs_replan.sort_by(|a, b| natural_cmp(&a.record, &b.record));

    roadmap.waves = waves;
    roadmap.ready = ready;
    roadmap.critical_path = critical_path;
    roadmap.records = OrderedMap(entries);
    roadmap.groups = groups;
    roadmap.needs_replan = needs_replan;
    roadmap.cycles = cycles
        .into_iter()
        .map(|c| c.iter().map(|i| i.as_str().to_string()).collect())
        .collect();
    roadmap.notes = notes;
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
