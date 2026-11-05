use super::relation::{pair_carried_by, pair_slot_claims, pair_slots};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Rule (`RFC-48`, `MILE-27`): a cycle in an `acyclic` relation pair. The
/// graph has one edge per resolved claim, drawn in the direction its field
/// declares: `A.forward = B` is `A -> B`, and `A.inverse = B` is `B -> A`, so
/// a record that writes only the inverse side still takes part. A claim that
/// resolves to no record, or to a type the pair does not carry, is no edge
/// (`sequence.unresolved-blocker` reports the first). Every record on a cycle
/// is reported with a shortest cycle through it, so a diff-scoped run that
/// examines only one of them still sees the finding.
pub fn sequence_cycle(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_SEQUENCE_CYCLE;
    let mut findings = Vec::new();
    let slots = pair_slots(records, config, |p| p.acyclic);

    let graphs = sequence_graphs(&slots, index).graphs;

    // The records on a cycle, per pair, each with its cycle path.
    let mut on_cycle: BTreeMap<(&str, crate::values::RecordId), String> = BTreeMap::new();
    for (pair, graph) in &graphs {
        for component in strongly_connected(graph) {
            let looped = component.len() > 1
                || graph
                    .get(&component[0])
                    .is_some_and(|next| next.contains(&component[0]));
            if !looped {
                continue;
            }
            for member in &component {
                if let Some(path) = shortest_cycle(graph, member) {
                    on_cycle.insert((pair, member.clone()), path);
                }
            }
        }
    }

    let mut reported: BTreeSet<(&str, crate::values::RecordId)> = BTreeSet::new();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| {
            // A record can sit on a cycle that only other records wrote, so
            // the report does not wait for a claim of its own.
            if let Some(id) = record_id(slot.record) {
                let key = (slot.pair.name.as_str(), crate::values::RecordId::new(&id));
                if let Some(path) = on_cycle.get(&key) {
                    if reported.insert(key) {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Error,
                            file: slot.record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: ActionSource::new("sequence-cycle"),
                            message: format!(
                                "{id} is on a cycle in acyclic pair '{}' ({} -> {}): {path}",
                                slot.pair.name, slot.pair.forward, slot.pair.inverse
                            ),
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

pub(super) type SequenceGraph =
    BTreeMap<crate::values::RecordId, BTreeSet<crate::values::RecordId>>;

/// The sequencing graph of every `acyclic` pair, built from the claims of the
/// given slots. An edge `X -> Y` reads "X depends on Y": `A.forward = B` is
/// `A -> B` and `A.inverse = B` is `B -> A`, so a record that writes only the
/// inverse side still takes part. A claim that resolves to no record, or to a
/// type the pair does not carry, is no edge (`sequence.unresolved-blocker`
/// reports the first), and neither is a claim on a record whose header cannot
/// be read (`ADR-76`); the dependent of such a claim is disclosed in
/// `unreadable_dependencies` so a rule that judges it can say so.
pub(super) struct SequenceGraphs<'a> {
    pub graphs: BTreeMap<&'a str, SequenceGraph>,
    pub unreadable_dependencies: BTreeSet<crate::values::RecordId>,
}

pub(super) fn sequence_graphs<'a>(
    slots: &[super::relation::PairSlot<'a>],
    index: &RecordIndex,
) -> SequenceGraphs<'a> {
    let mut graphs: BTreeMap<&'a str, SequenceGraph> = BTreeMap::new();
    let mut unreadable_dependencies = BTreeSet::new();
    for slot in slots {
        let Ok(references) = pair_slot_claims(slot) else {
            continue;
        };
        let Some(own) = record_id(slot.record) else {
            continue;
        };
        let own = crate::values::RecordId::new(&own);
        let graph = graphs.entry(slot.pair.name.as_str()).or_default();
        graph.entry(own.clone()).or_default();
        for reference in references {
            let target = crate::values::RecordId::new(&reference);
            let Some(record) = index.get(&target) else {
                continue;
            };
            if !pair_carried_by(slot.pair, &record.record_type) {
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
                        push(
                            "blocker-unresolved",
                            format!(
                            "{field}: {entry} does not resolve to any discovered record (pair '{}')",
                            slot.pair.name
                        ),
                        );
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
fn strongly_connected<N: Ord + Clone>(graph: &BTreeMap<N, BTreeSet<N>>) -> Vec<Vec<N>> {
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

/// A shortest cycle through `start`, rendered `A -> B -> A`; ties break on
/// `RecordId` order, so the text is the same on every run.
fn shortest_cycle(
    graph: &BTreeMap<crate::values::RecordId, BTreeSet<crate::values::RecordId>>,
    start: &crate::values::RecordId,
) -> Option<String> {
    let mut previous: BTreeMap<&crate::values::RecordId, &crate::values::RecordId> =
        BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        for next in graph.get(node)? {
            if next == start {
                let mut path = vec![start.as_str()];
                let mut at = node;
                let mut reversed = vec![at.as_str()];
                while at != start {
                    at = previous[at];
                    reversed.push(at.as_str());
                }
                reversed.reverse();
                path.clear();
                path.extend(reversed);
                path.push(start.as_str());
                return Some(path.join(" -> "));
            }
            if !previous.contains_key(next) {
                previous.insert(next, node);
                queue.push_back(next);
            }
        }
    }
    None
}

/// Rule (`RFC-48`, `MILE-144`, `ADR-64`): a record's written `Status` is held
/// to the derived `blocked` value. A record is blocked when any of its
/// dependencies, over every `acyclic` pair, is not in a status its own type
/// declares `success`: the same edges `sequence.cycle` checks, so a written
/// `Blocks` counts. A dependency whose type declares no `success`, or whose
/// `Status` is absent, is not in a declared success status and so blocks.
///
/// Two disagreements are reported, both on the dependent: a status the type
/// declares in `statuses.blocked` while nothing blocks it, and a derived
/// blocked record that states an `implementable` status (including one that
/// says it is finished). Only a type declaring `statuses.blocked` is examined.
/// A record with a claim on a record whose header cannot be read is counted
/// unreadable, since its blocked value cannot be computed.
pub fn sequence_status_agreement(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_SEQUENCE_STATUS_AGREEMENT;
    let mut findings = Vec::new();
    let declared = statuses_by_type(config);
    let slots = pair_slots(records, config, |p| p.acyclic);
    let SequenceGraphs {
        graphs,
        unreadable_dependencies,
    } = sequence_graphs(&slots, index);

    let candidates: Vec<&Record> = records
        .iter()
        .filter(|r| {
            declared
                .get(&r.record_type)
                .is_some_and(|d| !d.blocked.is_empty())
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
            if unreadable_dependencies.contains(&id) {
                return Outcome::Unreadable;
            }
            // Every dependency over every acyclic pair that is not finished.
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
            if decl.is_blocked_status(status) && !blocked {
                push(
                    ActionSource::new("status-blocked-unblocked").field(status_field),
                    format!(
                        "{status_field} is '{status}' but nothing blocks {id}: it has no dependency outside a success status; state the status it is in"
                    ),
                );
            } else if blocked && decl.implementable.iter().any(|s| s == status) {
                let mut source = ActionSource::new("status-blocked-derived").field(status_field);
                // A single declared blocked status is the computed answer.
                if let [only] = decl.blocked.as_slice() {
                    source = source.text(only.clone());
                }
                push(
                    source,
                    format!(
                        "{id} is blocked by {} but {status_field} is '{status}', an implementable status; state a blocked status ({})",
                        open.join(", "),
                        decl.blocked.join(", ")
                    ),
                );
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
        sequence_unresolved_blocker(records, config, &index)
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
