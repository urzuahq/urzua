use super::relation::{pair_carried_by, pair_slot_claims, pair_slots};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Rule (`RFC-48`, `MILE-27`, `MILE-192`, `ADR-82`): a cycle in an `acyclic`
/// relation pair. The graph has one edge per resolved claim, drawn in the
/// direction its field declares: `A.forward = B` is `A -> B`, and
/// `A.inverse = B` is `B -> A`, so a record that writes only the inverse side
/// still takes part. A claim that resolves to no record, or to a type the pair
/// does not carry, is no edge (`sequence.unresolved-blocker` reports the
/// first). Every record on a cycle is reported with a shortest cycle through
/// it, so a diff-scoped run that examines only one of them still sees the
/// finding.
///
/// The pairs `roadmap.pairs` names are read as one graph (`merged_graph`, the
/// graph `urzua roadmap` reads), so a loop that uses an edge of each of two
/// pairs is found and says which pairs it crosses. Any other acyclic pair is
/// read on its own, as every pair is when `roadmap.pairs` is absent.
pub fn sequence_cycle(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_SEQUENCE_CYCLE;
    let mut findings = Vec::new();
    let slots = pair_slots(records, config, |p| p.acyclic);
    let declared = &config.roadmap.pairs;
    // `None` is the merged roadmap graph; `Some(name)` a pair read alone.
    fn group_of<'n>(declared: &[String], name: &'n str) -> Option<&'n str> {
        (!declared.iter().any(|p| p == name)).then_some(name)
    }

    let graphs = sequence_graphs(&slots, index).graphs;

    let mut grouped: BTreeMap<Option<&str>, SequenceGraph> = BTreeMap::new();
    for (pair, graph) in &graphs {
        match group_of(declared, pair) {
            Some(name) => {
                grouped.insert(Some(name), graph.clone());
            }
            None => merge_into(grouped.entry(None).or_default(), graph),
        }
    }

    // The records on a cycle, per group, each with the rest of its message.
    let mut on_cycle: BTreeMap<(Option<&str>, crate::values::RecordId), CycleReport> =
        BTreeMap::new();
    for (group, graph) in &grouped {
        for component in strongly_connected(graph) {
            let looped = component.len() > 1
                || graph
                    .get(&component[0])
                    .is_some_and(|next| next.contains(&component[0]));
            if !looped {
                continue;
            }
            for member in &component {
                let Some(path) = shortest_cycle(graph, member) else {
                    continue;
                };
                let text = path
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                let report = match group {
                    Some(name) => {
                        let (forward, inverse) = config
                            .relation_pairs
                            .iter()
                            .find(|p| p.name == *name)
                            .map(|p| (p.forward.as_str(), p.inverse.as_str()))
                            .unwrap_or_default();
                        CycleReport {
                            variant: "sequence-cycle",
                            message: format!(
                                "is on a cycle in acyclic pair '{name}' ({forward} -> {inverse}): {text}"
                            ),
                        }
                    }
                    None => cycle_over_roadmap_pairs(&graphs, declared, &path, &text),
                };
                on_cycle.insert((*group, member.clone()), report);
            }
        }
    }

    let mut reported: BTreeSet<(Option<&str>, crate::values::RecordId)> = BTreeSet::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| {
            // A record can sit on a cycle that only other records wrote, so
            // the report does not wait for a claim of its own.
            if let Some(id) = record_id(slot.record) {
                let key = (
                    group_of(declared, slot.pair.name.as_str()),
                    crate::values::RecordId::new(&id),
                );
                if let Some(report) = on_cycle.get(&key) {
                    if reported.insert(key) {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Error,
                            file: slot.record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new(report.variant),
                            message: format!("{id} {}", report.message),
                        });
                    }
                }
            }
            if let Err(outcome) = pair_slot_claims(slot) {
                return outcome;
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

struct CycleReport {
    variant: &'static str,
    message: String,
}

/// The report for a cycle in the merged roadmap graph. A loop whose edges all
/// belong to one declared pair reads as a one-pair loop always did; one that
/// needs edges of more than one pair names every pair it uses and takes the
/// cross-pair action variant.
fn cycle_over_roadmap_pairs(
    graphs: &BTreeMap<&str, SequenceGraph>,
    declared: &[String],
    path: &[crate::values::RecordId],
    text: &str,
) -> CycleReport {
    type Edge<'a> = (&'a crate::values::RecordId, &'a crate::values::RecordId);
    let edges: Vec<Edge> = path.windows(2).map(|w| (&w[0], &w[1])).collect();
    let has_edge = |pair: &str, (from, to): Edge| {
        graphs
            .get(pair)
            .and_then(|g| g.get(from))
            .is_some_and(|next| next.contains(to))
    };
    if let Some(single) = declared
        .iter()
        .find(|pair| edges.iter().all(|e| has_edge(pair, *e)))
    {
        return CycleReport {
            variant: "sequence-cycle",
            message: format!("is on a cycle in acyclic pair '{single}': {text}"),
        };
    }
    let quoted: Vec<String> = declared
        .iter()
        .filter(|pair| edges.iter().any(|e| has_edge(pair, *e)))
        .map(|pair| format!("'{pair}'"))
        .collect();
    CycleReport {
        variant: "sequence-cycle-cross-pair",
        message: format!(
            "is on a cycle that crosses acyclic pairs {} (roadmap.pairs), each acyclic on its own: {text}",
            quoted.join(" and ")
        ),
    }
}

/// Adds every edge of `from` to `into`. `X -> {Y}` reads "X depends on Y".
pub(super) fn merge_into(into: &mut SequenceGraph, from: &SequenceGraph) {
    for (node, targets) in from {
        into.entry(node.clone()).or_default();
        for target in targets {
            into.entry(target.clone()).or_default();
            into.get_mut(node)
                .expect("just inserted")
                .insert(target.clone());
        }
    }
}

/// The one graph over several pairs' graphs, which `urzua roadmap` reads and
/// `sequence.cycle` reads for the pairs `roadmap.pairs` names.
pub(super) fn merged_graph<'g>(
    graphs: impl IntoIterator<Item = &'g SequenceGraph>,
) -> SequenceGraph {
    let mut merged = SequenceGraph::new();
    for graph in graphs {
        merge_into(&mut merged, graph);
    }
    merged
}

pub(super) type SequenceGraph =
    BTreeMap<crate::values::RecordId, BTreeSet<crate::values::RecordId>>;

/// The sequencing graph of every pair in the given slots. An edge `X -> Y`
/// reads "X depends on Y": `A.forward = B` is `A -> B` and `A.inverse = B` is
/// `B -> A`, so a record that writes only the inverse side still takes part. A
/// claim that resolves to no record, or to a type the pair does not carry, is
/// no edge (`sequence.unresolved-blocker` reports the first), and neither is a
/// claim on a record whose header cannot be read (`ADR-76`). What a rule that
/// derives a value from the graph must not guess at is disclosed:
/// `unreadable_dependencies` (the dependent of such a claim),
/// `unresolved_claimants` (a record whose own dependency field names something
/// that is not a resolvable record, so fixing the reference may add an edge)
/// and `unreadable_writers` (a record whose header cannot be read but whose
/// type declares a pair's inverse side, so it may be a dependency of a record
/// that cannot be seen).
pub(super) struct SequenceGraphs<'a> {
    pub graphs: BTreeMap<&'a str, SequenceGraph>,
    pub unreadable_dependencies: BTreeSet<crate::values::RecordId>,
    pub unresolved_claimants: BTreeSet<crate::values::RecordId>,
    pub unreadable_writers: bool,
}

pub(super) fn sequence_graphs<'a>(
    slots: &[super::relation::PairSlot<'a>],
    index: &RecordIndex,
) -> SequenceGraphs<'a> {
    let mut graphs: BTreeMap<&'a str, SequenceGraph> = BTreeMap::new();
    let mut unreadable_dependencies = BTreeSet::new();
    let mut unresolved_claimants = BTreeSet::new();
    let mut unreadable_writers = false;
    for slot in slots {
        let references = match pair_slot_claims(slot) {
            Ok(references) => references,
            Err(Outcome::Unreadable) => {
                if !slot.forward {
                    unreadable_writers = true;
                }
                continue;
            }
            Err(_) => continue,
        };
        let Some(own) = record_id(slot.record) else {
            continue;
        };
        let own = crate::values::RecordId::new(&own);
        let graph = graphs.entry(slot.pair.name.as_str()).or_default();
        graph.entry(own.clone()).or_default();
        // A forward value that is not a record reference at all is prose.
        if slot.forward
            && references.is_empty()
            && slot
                .record
                .header
                .get(slot.own_field())
                .is_some_and(|v| !crate::values::is_no_value_sentinel(v) && !v.trim().is_empty())
        {
            unresolved_claimants.insert(own.clone());
        }
        for reference in references {
            let target = crate::values::RecordId::new(&reference);
            let Some(record) = index.get(&target) else {
                if slot.forward {
                    unresolved_claimants.insert(own.clone());
                }
                continue;
            };
            if !pair_carried_by(slot.pair, &record.record_type) {
                if slot.forward {
                    unresolved_claimants.insert(own.clone());
                }
                continue;
            }
            if record.header.is_unreadable() {
                if slot.forward {
                    unreadable_dependencies.insert(own.clone());
                }
                continue;
            }
            graph.entry(target.clone()).or_default();
            let (from, to) = if slot.forward {
                (own.clone(), target)
            } else {
                (target, own.clone())
            };
            graph.entry(from).or_default().insert(to);
        }
    }
    SequenceGraphs {
        graphs,
        unreadable_dependencies,
        unresolved_claimants,
        unreadable_writers,
    }
}

/// Rule (`RFC-48`, `MILE-27`): a value in either field of an `acyclic` pair
/// that names no record. Two shapes, both reported: a reference that resolves
/// to nothing, and a comma-separated entry that is not a record reference, so
/// prose cannot sit in a sequencing field and be dropped from the graph without
/// a word. A target whose header cannot be read is disclosed under its own
/// entry, not blamed on the citer.
pub fn sequence_unresolved_blocker(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
    untracked: &UntrackedRecords,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_SEQUENCE_UNRESOLVED_BLOCKER;
    let mut findings = Vec::new();
    let slots = pair_slots(records, config, |p| p.acyclic);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| {
            if let Err(outcome) = pair_slot_claims(slot) {
                return outcome;
            }
            let field = slot.own_field();
            // A claim a pointer rule already reports is not reported twice: the
            // field is one of the type's `pointer_fields` and the rule that
            // judges that shape is enabled.
            let is_pointer = config
                .record_types
                .get(&slot.record.record_type)
                .and_then(|t| t.pointer_fields.as_ref())
                .is_some_and(|fields| fields.iter().any(|f| f == field));
            let enabled = |rule: &str| {
                config
                    .rules
                    .get(rule)
                    .is_some_and(|s| s.level != crate::config::RuleLevel::Off)
            };
            let resolution_reported = is_pointer && enabled(RULE_POINTER_RESOLUTION);
            let prose_reported = is_pointer && enabled(RULE_HEADER_POINTER_FIELD_CLEAN);
            let mut push = |variant: &'static str, message: String| {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: slot.record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: ActionSource::new(variant).field(field),
                    message,
                });
            };
            let value = slot.record.header.get(field).unwrap_or("");
            // An entry is a reference only when the whole trimmed entry is one
            // record reference; `MILE-4 MILE-5` and `MILE-5 (see note)` are
            // prose, not a reference followed by commentary.
            for entry in value.split(',').map(str::trim).filter(|e| !e.is_empty()) {
                if is_record_reference(entry) {
                    if !resolution_reported
                        && !index.contains_key(&crate::values::RecordId::new(entry))
                    {
                        let (variant, message) = unresolved_reference(
                            field,
                            entry,
                            untracked,
                            ("blocker-unresolved", "blocker-untracked"),
                            &format!(" (pair '{}')", slot.pair.name),
                        );
                        push(variant, message);
                    }
                } else if !prose_reported {
                    push(
                        "blocker-not-a-reference",
                        format!(
                        "{field}: '{entry}' is not a record reference, so it orders nothing (pair '{}')",
                        slot.pair.name
                        ),
                    );
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

/// Tarjan's algorithm, iterative so a long chain cannot exhaust the stack.
/// Components and their members come out in `RecordId` order.
pub(super) fn strongly_connected<N: Ord + Clone>(graph: &BTreeMap<N, BTreeSet<N>>) -> Vec<Vec<N>> {
    let nodes: Vec<&N> = graph.keys().collect();
    let position: BTreeMap<&N, usize> = nodes.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let edges: Vec<Vec<usize>> = nodes
        .iter()
        .map(|n| {
            graph[*n]
                .iter()
                .filter_map(|m| position.get(m).copied())
                .collect()
        })
        .collect();
    let count = nodes.len();
    let mut next_index = 0;
    let mut order = vec![usize::MAX; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    for root in 0..count {
        if order[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        while let Some(&(node, cursor)) = work.last() {
            if cursor == 0 {
                order[node] = next_index;
                low[node] = next_index;
                next_index += 1;
                stack.push(node);
                on_stack[node] = true;
            }
            if let Some(&next) = edges[node].get(cursor) {
                work.last_mut().expect("work is non-empty").1 += 1;
                if order[next] == usize::MAX {
                    work.push((next, 0));
                } else if on_stack[next] {
                    low[node] = low[node].min(order[next]);
                }
                continue;
            }
            work.pop();
            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == order[node] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(nodes[member].clone());
                    if member == node {
                        break;
                    }
                }
                component.sort();
                components.push(component);
            }
        }
    }
    components.sort();
    components
}

/// A shortest cycle through `start`, as the ids `A, B, A`; ties break on
/// `RecordId` order, so the path is the same on every run.
fn shortest_cycle(
    graph: &SequenceGraph,
    start: &crate::values::RecordId,
) -> Option<Vec<crate::values::RecordId>> {
    let mut previous: BTreeMap<&crate::values::RecordId, &crate::values::RecordId> =
        BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        for next in graph.get(node)? {
            if next == start {
                let mut at = node;
                let mut reversed = vec![at.clone()];
                while at != start {
                    at = previous[at];
                    reversed.push(at.clone());
                }
                reversed.reverse();
                reversed.push(start.clone());
                return Some(reversed);
            }
            if !previous.contains_key(next) {
                previous.insert(next, node);
                queue.push_back(next);
            }
        }
    }
    None
}

/// Rule (`RFC-48`, `MILE-144`, `ADR-64`, `ADR-83`): a record's written `Status`
/// is held to the derived `blocked` value. A record is blocked when any of its
/// dependencies, over the pairs `roadmap.pairs` declares (the pairs the
/// roadmap reads, so the two cannot disagree), is not in a status its own type
/// declares `success`. A dependency whose type declares no `success`, or whose
/// `Status` is absent, is not in a declared success status and so blocks, and
/// so does one in a `failure` status. A written `Blocks` counts, since the
/// edges are the ones `sequence.cycle` checks.
///
/// Two disagreements are reported, both on the dependent: a blocked record that
/// states a `progress` status, `Done` included when the type lists it, and a
/// status the type declares in `blocked` while nothing blocks it. A status that
/// does not claim progress, `Planned` for instance, is right for a blocked
/// record. Only a type declaring `blocked` or `progress` is examined. A blocked
/// value that cannot be computed is not guessed: a record in a blocked or a
/// progress status that would be reported unblocked is counted unreadable when
/// it names a dependency that does not resolve or whose header cannot be read,
/// or when a record that may write an inverse claim has an unreadable header.
/// This holds for a type declaring only `progress` as for one that also writes
/// `blocked` (`ADR-83`).
///
/// One blocked record is not reported (`ADR-83`, amendment of 2026-11-07): a
/// record whose `partial_status` comes from the dependency cap of `ADR-96`,
/// that is, whose Done means items all resolve and which only an unfinished
/// dependency holds back. It has not made progress out of order; its items were
/// proven ahead of the dependency. It gets an info finding saying so, naming the
/// dependency. A record genuinely in that status with an unresolved item is
/// still reported.
pub fn sequence_status_agreement(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
    back_pointed: &BackPointerEvidence,
    env: &LocatorEnv<'_>,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_SEQUENCE_STATUS_AGREEMENT;
    let mut findings = Vec::new();
    let declared = statuses_by_type(config);
    let slots = pair_slots(records, config, |p| config.roadmap.pairs.contains(&p.name));
    let SequenceGraphs {
        graphs,
        unreadable_dependencies,
        unresolved_claimants,
        unreadable_writers,
    } = sequence_graphs(&slots, index);

    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| {
            declared
                .get(&r.record_type)
                .is_some_and(|d| !d.blocked.is_empty() || !d.progress.is_empty())
        })
        .collect();

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        candidates,
        |record| *record,
        |record| {
            let decl = &declared[&record.record_type];
            let Some(id) = record_id(record) else {
                return Outcome::OutOfScope;
            };
            let id = crate::values::RecordId::new(&id);
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
            // Every dependency over every declared pair that is not finished.
            let mut open: Vec<String> = Vec::new();
            for graph in graphs.values() {
                for dependency in graph.get(&id).into_iter().flatten() {
                    let Some(target) = index.get(dependency) else {
                        continue;
                    };
                    let target_field = relation_field_name(
                        config,
                        &target.record_type,
                        crate::config::RelationRole::Status,
                    );
                    let target_status = target.header.get(target_field).map(str::trim);
                    let finished = target_status.is_some_and(|s| {
                        declared
                            .get(&target.record_type)
                            .is_some_and(|d| d.is_success(s))
                    });
                    if !finished {
                        let shown = format!(
                            "{dependency} ({})",
                            target_status
                                .filter(|s| !s.is_empty())
                                .unwrap_or("no status")
                        );
                        if !open.contains(&shown) {
                            open.push(shown);
                        }
                    }
                }
            }
            let blocked = !open.is_empty();
            let mut push = |source: ActionSource, message: String| {
                findings.push(Finding {
                    rule: RULE_ID.to_string(),
                    severity: FindingSeverity::Error,
                    file: record.path.clone(),
                    line: None,
                    waived: None,
                    declared_severity: None,
                    suggested_action: None,
                    action_source: source,
                    message,
                });
            };
            if blocked {
                if decl.is_progress_status(status) {
                    // A status that comes from the dependency cap is not out of
                    // order: say so, and report nothing else for the record.
                    let one_blocked = HashMap::from([(record.path.clone(), open.clone())]);
                    let ctx = DeliveryContext {
                        back_pointed,
                        env,
                        blocked: &one_blocked,
                    };
                    let capped = judge_delivery(record, config, decl, &ctx)
                        .ok()
                        .and_then(|judged| judged.items)
                        .filter(|items| !items.capped_by.is_empty())
                        .filter(|_| {
                            decl.delivery
                                .as_ref()
                                .and_then(|d| d.items.as_ref())
                                .is_some_and(|i| i.partial_status == status)
                        });
                    if let Some(items) = capped {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Info,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: Some(FindingSeverity::Info),
                            suggested_action: None,
                            action_source: ActionSource::new("status-blocked-capped"),
                            message: format!(
                                "{id} is {status_field} '{status}', not out of order: all items proven ahead of dependency {}, which caps the status",
                                items.capped_by.join(", ")
                            ),
                        });
                        return Outcome::Examined;
                    }
                    let mut source =
                        ActionSource::new("status-blocked-derived").field(status_field);
                    // The computed answer, only when one status is declared for it
                    // and the lifecycle lets this record move there: a finished
                    // record's transitions do not lead back to a waiting state.
                    if let [only] = decl.blocked.as_slice() {
                        if !decl.is_success(status)
                            && (!decl.has_transitions() || decl.allows(status, only))
                        {
                            source = source.text(only.clone());
                        }
                    }
                    push(
                        source,
                        format!(
                            "{id} is blocked by {} but {status_field} is '{status}', which claims progress; state a status that does not claim progress ({})",
                            open.join(", "),
                            decl.progress.join(", ")
                        ),
                    );
                }
            } else {
                // "Nothing blocks it" is only known when every claim was read,
                // and it is the answer both for a written blocked status (is it
                // contradicted?) and for a progress status (is it contradicted
                // by a blocker?), so a type declaring only `progress` is guarded
                // too (`ADR-83`).
                let judged = decl.is_blocked_status(status) || decl.is_progress_status(status);
                if judged
                    && (unreadable_dependencies.contains(&id)
                        || unresolved_claimants.contains(&id)
                        || unreadable_writers)
                {
                    return Outcome::Unreadable;
                }
                if decl.is_blocked_status(status) {
                    push(
                        ActionSource::new("status-blocked-unblocked").field(status_field),
                        format!(
                            "{status_field} is '{status}' but nothing blocks {id}: it has no dependency outside a success status; state the status it is in"
                        ),
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::*;

    fn seq_pair(reciprocity: crate::config::PairReciprocity) -> crate::config::RelationPair {
        crate::config::RelationPair {
            name: "sequencing".to_string(),
            forward: "Depends-on".to_string(),
            inverse: "Blocks".to_string(),
            reciprocity,
            cross_type: true,
            provisional: None,
            types: None,
            acyclic: true,
        }
    }

    fn seq_config(types: &[&str]) -> Config {
        let mut config = config_with_types(
            types
                .iter()
                .map(|t| {
                    (
                        *t,
                        type_config_pointer(
                            &[],
                            Some(&["Depends-on", "Blocks", "Title"]),
                            None,
                            None,
                        ),
                    )
                })
                .collect(),
        );
        config.relation_pairs = vec![seq_pair(crate::config::PairReciprocity::None)];
        config
    }

    fn mile(n: u32, header: &str) -> Record {
        record_with_prefix(
            &format!("docs/milestones/MILE-{n}-x.md"),
            "milestone",
            "MILE",
            &format!("---\n{header}\n---\n"),
        )
    }

    fn cycle(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
        let index = build_normalized_index(records);
        sequence_cycle(records, config, &index)
    }

    fn unresolved(records: &[Record], config: &Config) -> (RuleExecution, Vec<Finding>) {
        let index = build_normalized_index(records);
        sequence_unresolved_blocker(records, config, &index, &Default::default())
    }

    #[test]
    fn a_two_record_cycle_is_reported_on_both_records_with_its_path() {
        let config = seq_config(&["milestone"]);
        let records = [mile(1, "Depends-on: MILE-2"), mile(2, "Depends-on: MILE-1")];
        let (exec, findings) = cycle(&records, &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings.iter().all(|f| f.rule == RULE_SEQUENCE_CYCLE));
        assert!(
            findings[0].message.contains("MILE-1 -> MILE-2 -> MILE-1"),
            "{findings:?}"
        );
        assert_eq!(examined(&exec), 2);
    }

    #[test]
    fn a_record_that_depends_on_itself_is_a_cycle() {
        let config = seq_config(&["milestone"]);
        let records = [mile(1, "Depends-on: MILE-1")];
        let (_, findings) = cycle(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("MILE-1 -> MILE-1"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_cycle_across_record_types_is_reported() {
        let config = seq_config(&["milestone", "bug"]);
        let bug = record_with_prefix(
            "docs/bugs/BUG-7-x.md",
            "bug",
            "BUG",
            "---\nDepends-on: MILE-1\n---\n",
        );
        let records = [mile(1, "Depends-on: BUG-7"), bug];
        let (_, findings) = cycle(&records, &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn a_chain_and_a_diamond_are_not_cycles_and_are_counted_examined() {
        let config = seq_config(&["milestone"]);
        let records = [
            mile(1, "Depends-on: MILE-2, MILE-3"),
            mile(2, "Depends-on: MILE-4"),
            mile(3, "Depends-on: MILE-4"),
            mile(4, "Title: leaf"),
        ];
        let (exec, findings) = cycle(&records, &config);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 3, "the leaf writes no claim");
    }

    #[test]
    fn the_written_inverse_field_contributes_the_reverse_edge() {
        // 1 depends on 2; 1 also says it blocks 2, i.e. 2 depends on 1.
        let config = seq_config(&["milestone"]);
        let records = [
            mile(1, "Depends-on: MILE-2\nBlocks: MILE-2"),
            mile(2, "Title: x"),
        ];
        let (_, findings) = cycle(&records, &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn a_cycle_is_only_looked_for_in_pairs_that_declare_acyclic() {
        let mut config = seq_config(&["milestone"]);
        config.relation_pairs[0].acyclic = false;
        let records = [mile(1, "Depends-on: MILE-2"), mile(2, "Depends-on: MILE-1")];
        let (exec, findings) = cycle(&records, &config);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn an_unresolved_target_is_no_edge_and_does_not_hide_a_real_cycle() {
        let config = seq_config(&["milestone"]);
        let records = [
            mile(1, "Depends-on: MILE-2, MILE-99"),
            mile(2, "Depends-on: MILE-1"),
        ];
        let (_, findings) = cycle(&records, &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn a_dangling_target_is_reported_by_the_unresolved_blocker_rule() {
        let config = seq_config(&["milestone"]);
        let records = [mile(1, "Depends-on: MILE-2, MILE-99"), mile(2, "Title: x")];
        let (exec, findings) = unresolved(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, RULE_SEQUENCE_UNRESOLVED_BLOCKER);
        assert!(findings[0].message.contains("MILE-99"), "{findings:?}");
        assert_eq!(examined(&exec), 1);
    }

    #[test]
    fn a_blocker_on_an_untracked_file_on_disk_is_its_own_variant_observed_failing() {
        let config = seq_config(&["milestone"]);
        let records = [mile(1, "Depends-on: MILE-2, MILE-99")];
        let index = build_normalized_index(&records);
        let untracked: UntrackedRecords = [(
            crate::values::RecordId::new("MILE-2"),
            PathBuf::from("docs/milestones/MILE-2-x.md"),
        )]
        .into();
        let (_, findings) = sequence_unresolved_blocker(&records, &config, &index, &untracked);
        let variant = |id: &str| {
            findings
                .iter()
                .find(|f| f.message.contains(id))
                .map(|f| f.action_source.variant)
        };
        assert_eq!(variant("MILE-2"), Some("blocker-untracked"), "{findings:?}");
        assert_eq!(
            variant("MILE-99"),
            Some("blocker-unresolved"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_value_that_names_no_record_is_reported_not_dropped() {
        let config = seq_config(&["milestone"]);
        let records = [
            mile(1, "Depends-on: a real adopter asking for it, MILE-2"),
            mile(2, "Title: x"),
        ];
        let (_, findings) = unresolved(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("a real adopter asking for it"),
            "{findings:?}"
        );
    }

    #[test]
    fn an_unwritten_field_and_the_nothing_here_sentinel_are_not_findings() {
        let config = seq_config(&["milestone"]);
        let records = [mile(1, "Depends-on: —"), mile(2, "Title: x")];
        let (exec, findings) = unresolved(&records, &config);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(examined(&exec), 1, "the sentinel is an answer");
    }

    #[test]
    fn the_blocks_side_is_checked_for_dangling_targets_too() {
        let config = seq_config(&["milestone"]);
        let records = [mile(1, "Blocks: MILE-98")];
        let (_, findings) = unresolved(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Blocks"), "{findings:?}");
    }

    #[test]
    fn an_unacyclic_pair_is_not_the_unresolved_blocker_rules() {
        let mut config = seq_config(&["milestone"]);
        config.relation_pairs[0].acyclic = false;
        let records = [mile(1, "Depends-on: MILE-99")];
        let (exec, findings) = unresolved(&records, &config);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn an_entry_that_starts_with_a_reference_but_is_not_one_is_prose() {
        let config = seq_config(&["milestone"]);
        let records = [
            mile(1, "Depends-on: MILE-2 MILE-3"),
            mile(2, "Depends-on: MILE-3 (see the note)"),
            mile(3, "Title: x"),
        ];
        let (_, findings) = unresolved(&records, &config);
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings
            .iter()
            .all(|f| f.message.contains("not a record reference")));
    }

    fn with_rule(mut config: Config, rule: &str) -> Config {
        let setting: crate::config::RuleSetting =
            yaml_serde::from_str("error").expect("a level parses");
        config.rules.insert(rule.to_string(), setting);
        config
    }

    fn pointer_config() -> Config {
        let mut config = seq_config(&["milestone"]);
        for t in config.record_types.values_mut() {
            t.pointer_fields = Some(vec!["Depends-on".into(), "Blocks".into()]);
        }
        config
    }

    #[test]
    fn a_claim_the_pointer_rules_report_is_not_reported_again() {
        let records = [mile(1, "Depends-on: MILE-99, waiting on an adopter")];
        // Neither pointer rule enabled: the sequence rule is the only reporter.
        let (_, findings) = unresolved(&records, &pointer_config());
        assert_eq!(findings.len(), 2, "{findings:?}");
        // pointer.resolution enabled: the dangling target is its report.
        let config = with_rule(pointer_config(), RULE_POINTER_RESOLUTION);
        let (exec, findings) = unresolved(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("not a record reference"));
        assert_eq!(examined(&exec), 1, "the claim is still examined");
        // Both enabled: nothing is left for the sequence rule.
        let config = with_rule(config, RULE_HEADER_POINTER_FIELD_CLEAN);
        let (_, findings) = unresolved(&records, &config);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_pair_field_that_is_not_a_pointer_field_is_always_reported() {
        let config = with_rule(
            with_rule(seq_config(&["milestone"]), RULE_POINTER_RESOLUTION),
            RULE_HEADER_POINTER_FIELD_CLEAN,
        );
        let records = [mile(1, "Depends-on: MILE-99")];
        let (_, findings) = unresolved(&records, &config);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }
}
