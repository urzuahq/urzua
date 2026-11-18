//! The facts a view of the records needs, computed once from the records and
//! the configuration so `urzua export`, the read-only server and `urzua board`
//! all read the same answers. Pure: records, configuration and the resolved
//! edges in, one `RecordView` per record out.
//!
//! The engine names no record type, status or workflow here. Every state comes
//! from what a type declares (`statuses.success`, `failure`, `progress`), every
//! grouping from a declared `part_of` field, every wait from `roadmap.pairs`.

use crate::config::{Config, RelationRole};
use crate::graph::GraphEdge;
use crate::record::Record;
use crate::rules::{
    build_normalized_index, extract_references, natural_cmp, record_id, title_in_text,
    unfinished_dependencies, RecordIndex,
};
use crate::status_model::StatusDecl;
use crate::values::RecordId;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Where a record stands, in a fixed set every type maps onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StateCategory {
    /// The status is in the type's `statuses.success`.
    #[serde(rename = "finished")]
    Finished,
    /// The status is in the type's `statuses.failure`.
    #[serde(rename = "dropped")]
    Dropped,
    /// The status is in the type's `statuses.progress` and in neither of the above.
    #[serde(rename = "under-way")]
    UnderWay,
    /// Anything else of a type that declares statuses.
    #[serde(rename = "not-started")]
    NotStarted,
    /// The type declares no statuses.
    #[serde(rename = "none")]
    NoStatuses,
}

impl StateCategory {
    /// A record that is neither finished nor dropped.
    pub fn is_open(self) -> bool {
        !matches!(self, StateCategory::Finished | StateCategory::Dropped)
    }
}

/// A header field's value: text, or the elements of a list-valued field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    Text(String),
    List(Vec<String>),
}

/// How many children of a record are in each state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ChildrenByState {
    pub finished: usize,
    pub dropped: usize,
    #[serde(rename = "under-way")]
    pub under_way: usize,
    #[serde(rename = "not-started")]
    pub not_started: usize,
    pub none: usize,
}

impl ChildrenByState {
    fn add(&mut self, state: StateCategory) {
        match state {
            StateCategory::Finished => self.finished += 1,
            StateCategory::Dropped => self.dropped += 1,
            StateCategory::UnderWay => self.under_way += 1,
            StateCategory::NotStarted => self.not_started += 1,
            StateCategory::NoStatuses => self.none += 1,
        }
    }
}

/// A move a gate reserves for a person, which the record's declared
/// transitions allow from its current status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GatedMove {
    pub to: String,
    pub by: String,
}

/// What one record adds to its export, beyond what it holds.
#[derive(Debug, Clone, Serialize)]
pub struct RecordView {
    /// The title the engine's own title check reads; the record's id when it
    /// has none (see `title_is_fallback`).
    pub title: String,
    pub title_is_fallback: bool,
    /// The header as a map; a list-valued field is an array.
    pub fields: BTreeMap<String, FieldValue>,
    /// The value of the status field, `null` when the record states none.
    pub status: Option<String>,
    pub state: StateCategory,
    /// The declaration that decided `state`: the `statuses` list a status was
    /// found in, or why none was (`default`, `no-status`, `no-statuses-declared`).
    pub state_decided_by: String,
    /// The records it directly depends on, through the pairs `roadmap.pairs`
    /// names, that are not finished.
    pub waiting_on: Vec<String>,
    /// A record of a work type that is not finished, not dropped and waits on nothing.
    pub ready: bool,
    /// Moves a gate reserves for a person that its declared transitions allow
    /// from its status; always empty for a finished or dropped record.
    pub gated_moves: Vec<GatedMove>,
    /// The record this one is part of, through the type's `part_of` field.
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub children_by_state: ChildrenByState,
    /// The record's state and its children's disagree: it is finished while a
    /// child is neither finished nor dropped, or not started while a child is
    /// under way or finished.
    pub state_disagrees_with_children: bool,
    /// Per reference field, the records it names that exist.
    pub refers_to: BTreeMap<String, Vec<String>>,
    /// Per reference field, the ids it names that no record carries.
    pub unresolved_refs: BTreeMap<String, Vec<String>>,
    /// Per reference field, the records that name this one through it.
    pub referred_by: BTreeMap<String, Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

/// What a record type means to a view, read from the configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeRole {
    /// Whether the type is in `roadmap.work_types` (every type when it is absent).
    pub work: bool,
    /// The pointer field the type declares as containment (`part_of`).
    pub part_of: Option<String>,
    /// The fields the type gives a declared vocabulary of values (`field_values`).
    pub value_fields: Vec<String>,
    /// Whether the type declares statuses at all.
    pub statuses_declared: bool,
    /// A gate reserves a move to some status of the type, but the type
    /// declares no transitions, so the next move cannot be read.
    pub gated_without_transitions: bool,
}

/// Every record's view, aligned with the records given, and each type's role.
#[derive(Debug, Clone)]
pub struct ViewModel {
    pub records: Vec<RecordView>,
    pub roles: BTreeMap<String, TypeRole>,
}

/// What `compute` reads beyond the records and configuration.
pub struct ViewInput<'a> {
    pub records: &'a [Record],
    pub config: &'a Config,
    pub full_text: &'a HashMap<PathBuf, String>,
    /// The edges `urzua graph` computes over the same records.
    pub edges: &'a [GraphEdge],
}

fn present(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty() && !crate::values::is_no_value_sentinel(v))
        .map(str::to_string)
}

fn natural_sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort_by(|a, b| natural_cmp(a, b));
    ids.dedup();
    ids
}

/// The status a record states and the state it puts the record in.
fn state_of(
    config: &Config,
    decls: &HashMap<String, StatusDecl>,
    record: &Record,
) -> (Option<String>, StateCategory, String) {
    let type_name = &record.record_type;
    let field = config.record_types[type_name].relation_field(RelationRole::Status);
    let status = present(record.header.get(field));
    let declared = decls.get(type_name).filter(|d| {
        !(d.values.is_empty()
            && d.success.is_empty()
            && d.failure.is_empty()
            && d.progress.is_empty())
    });
    let Some(decl) = declared else {
        return (
            status,
            StateCategory::NoStatuses,
            "no-statuses-declared".to_string(),
        );
    };
    let Some(value) = status.as_deref() else {
        return (status, StateCategory::NotStarted, "no-status".to_string());
    };
    let by = |list: &str| format!("record_types.{type_name}.statuses.{list}");
    let (state, decided_by) = if decl.success.iter().any(|v| v == value) {
        (StateCategory::Finished, by("success"))
    } else if decl.failure.iter().any(|v| v == value) {
        (StateCategory::Dropped, by("failure"))
    } else if decl.is_progress_status(value) {
        (StateCategory::UnderWay, by("progress"))
    } else {
        (StateCategory::NotStarted, "default".to_string())
    };
    (status, state, decided_by)
}

/// The record `record` is part of: the first reference in its type's `part_of`
/// field that resolves to a record of one of the field's declared target
/// types, other than itself. `None` for a type with no `part_of` field.
pub fn parent_of<'a>(
    config: &Config,
    index: &RecordIndex<'a>,
    record: &Record,
) -> Option<&'a Record> {
    let field = config.part_of.get(&record.record_type)?;
    let targets = config
        .pointer_targets
        .get(&record.record_type)?
        .get(field)?;
    let value = record.header.get(field)?;
    extract_references(value).into_iter().find_map(|reference| {
        index
            .get(&RecordId::new(&reference))
            .copied()
            .filter(|p| p.path != record.path && targets.contains(&p.record_type))
    })
}

/// The records `record` is part of, nearest first. A loop in the declarations
/// ends the chain where it would revisit a record.
pub fn ancestors<'a>(config: &Config, index: &RecordIndex<'a>, record: &Record) -> Vec<&'a Record> {
    let mut chain: Vec<&Record> = Vec::new();
    let mut at = record;
    while let Some(parent) = parent_of(config, index, at) {
        if parent.path == record.path || chain.iter().any(|c| c.path == parent.path) {
            break;
        }
        chain.push(parent);
        at = parent;
    }
    chain
}

/// Whether a parent's own state and its children's disagree.
fn disagrees(state: StateCategory, children: &ChildrenByState) -> bool {
    match state {
        StateCategory::Finished => children.under_way + children.not_started + children.none > 0,
        StateCategory::NotStarted => children.under_way + children.finished > 0,
        _ => false,
    }
}

/// Header fields whose value was written as a YAML list, which the header
/// reader joins into one comma-separated string.
fn yaml_list_keys(record: &Record, text: &str) -> Vec<String> {
    let Some((1, end)) = record.header.region else {
        return Vec::new();
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let body: Vec<&str> = text.lines().take(end.saturating_sub(1)).skip(1).collect();
    match yaml_serde::from_str::<yaml_serde::Value>(&body.join("\n")) {
        Ok(yaml_serde::Value::Mapping(map)) => map
            .into_iter()
            .filter(|(_, v)| matches!(v, yaml_serde::Value::Sequence(_)))
            .filter_map(|(k, _)| k.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect()
}

/// A record's header as a map. A field is list-valued when its type declares it
/// a pointer field or gives it a vocabulary of values, or when it was written
/// as a YAML list.
fn fields_of(config: &Config, record: &Record, text: &str) -> BTreeMap<String, FieldValue> {
    let type_config = &config.record_types[&record.record_type];
    let written_as_list = yaml_list_keys(record, text);
    let mut fields = BTreeMap::new();
    for field in &record.header.fields {
        let name = field.key.as_str();
        if fields.contains_key(name) {
            continue;
        }
        let is_list = written_as_list.iter().any(|k| k == name)
            || type_config
                .pointer_fields
                .as_ref()
                .is_some_and(|p| p.iter().any(|f| f == name))
            || type_config
                .field_values
                .as_ref()
                .is_some_and(|v| v.contains_key(name));
        let value = if is_list {
            FieldValue::List(split_list(&field.value))
        } else {
            FieldValue::Text(field.value.clone())
        };
        fields.insert(name.to_string(), value);
    }
    fields
}

/// Moves a gate reserves for a person that the record's declared transitions
/// allow from its current status.
fn gated_moves(
    config: &Config,
    decl: Option<&StatusDecl>,
    record: &Record,
    status: Option<&str>,
) -> Vec<GatedMove> {
    let (Some(authority), Some(decl), Some(status)) = (&config.authority, decl, status) else {
        return Vec::new();
    };
    let Some(next) = decl.transitions.get(status) else {
        return Vec::new();
    };
    let mut moves = Vec::new();
    for gate in &authority.gates {
        if gate.record_type.as_deref() != Some(record.record_type.as_str()) {
            continue;
        }
        for to in next {
            let reserved = gate.to.iter().any(|g| crate::authority::same_status(g, to));
            let listed = moves.iter().any(|m: &GatedMove| m.to == *to);
            if reserved && !listed {
                moves.push(GatedMove {
                    to: to.clone(),
                    by: gate.by.clone(),
                });
            }
        }
    }
    moves
}

fn roles(config: &Config, decls: &HashMap<String, StatusDecl>) -> BTreeMap<String, TypeRole> {
    config
        .record_types
        .iter()
        .map(|(name, type_config)| {
            let decl = decls.get(name);
            let gated = config.authority.as_ref().is_some_and(|a| {
                a.gates
                    .iter()
                    .any(|g| g.record_type.as_deref() == Some(name.as_str()))
            });
            let role = TypeRole {
                work: config
                    .roadmap
                    .work_types
                    .as_ref()
                    .is_none_or(|types| types.contains(name)),
                part_of: config.part_of.get(name).cloned(),
                value_fields: type_config
                    .field_values
                    .as_ref()
                    .map(|v| v.keys().cloned().collect())
                    .unwrap_or_default(),
                statuses_declared: decl.is_some_and(|d| !d.values.is_empty()),
                gated_without_transitions: gated && decl.is_none_or(|d| !d.has_transitions()),
            };
            (name.clone(), role)
        })
        .collect()
}

/// The reference edges per record, by field: what each names and what names it.
struct Edges {
    refers_to: Vec<BTreeMap<String, Vec<String>>>,
    unresolved: Vec<BTreeMap<String, Vec<String>>>,
    referred_by: Vec<BTreeMap<String, Vec<String>>>,
}

fn collect_edges(
    records: &[Record],
    index: &RecordIndex<'_>,
    edges: &[GraphEdge],
    ids: &[Option<String>],
) -> Edges {
    let position: HashMap<&Path, usize> = records
        .iter()
        .enumerate()
        .map(|(i, r)| (r.path.as_path(), i))
        .collect();
    let by_id: HashMap<&str, usize> = ids
        .iter()
        .enumerate()
        .filter_map(|(i, id)| id.as_deref().map(|id| (id, i)))
        .collect();
    let n = records.len();
    let mut out = Edges {
        refers_to: vec![BTreeMap::new(); n],
        unresolved: vec![BTreeMap::new(); n],
        referred_by: vec![BTreeMap::new(); n],
    };
    for edge in edges {
        let Some(&from) = by_id.get(edge.from.as_str()) else {
            continue;
        };
        let target = index
            .get(&RecordId::new(&edge.to))
            .and_then(|r| position.get(r.path.as_path()).copied());
        match target {
            Some(to) if !edge.dangling => {
                let to_id = ids[to].clone().unwrap_or_else(|| edge.to.clone());
                out.refers_to[from]
                    .entry(edge.relation.clone())
                    .or_default()
                    .push(to_id);
                if let Some(from_id) = &ids[from] {
                    out.referred_by[to]
                        .entry(edge.relation.clone())
                        .or_default()
                        .push(from_id.clone());
                }
            }
            _ => out.unresolved[from]
                .entry(edge.relation.clone())
                .or_default()
                .push(edge.to.clone()),
        }
    }
    for map in out
        .refers_to
        .iter_mut()
        .chain(out.unresolved.iter_mut())
        .chain(out.referred_by.iter_mut())
    {
        for list in map.values_mut() {
            *list = natural_sorted(std::mem::take(list));
        }
    }
    out
}

/// The facts about one record's place in the containment tree.
struct Tree {
    parent: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
}

fn containment(records: &[Record], config: &Config, index: &RecordIndex<'_>) -> Tree {
    let position: HashMap<&Path, usize> = records
        .iter()
        .enumerate()
        .map(|(i, r)| (r.path.as_path(), i))
        .collect();
    let parent: Vec<Option<usize>> = records
        .iter()
        .map(|r| parent_of(config, index, r).and_then(|p| position.get(p.path.as_path()).copied()))
        .collect();
    let mut children = vec![Vec::new(); records.len()];
    for (child, parent) in parent.iter().enumerate() {
        if let Some(parent) = parent {
            children[*parent].push(child);
        }
    }
    Tree { parent, children }
}

/// Computes every record's view.
pub fn compute(input: &ViewInput<'_>) -> ViewModel {
    let ViewInput {
        records,
        config,
        full_text,
        edges,
    } = input;
    let decls = crate::rules::statuses_by_type(config);
    let index = build_normalized_index(records);
    let ids: Vec<Option<String>> = records.iter().map(record_id).collect();
    let standings: Vec<(Option<String>, StateCategory, String)> = records
        .iter()
        .map(|r| state_of(config, &decls, r))
        .collect();
    let waits = unfinished_dependencies(records, config);
    let tree = containment(records, config, &index);
    let edge_maps = collect_edges(records, &index, edges, &ids);
    let roles = roles(config, &decls);
    let display = |id: &str| -> Option<String> {
        let record = index.get(&RecordId::new(id))?;
        record_id(record)
    };

    let views = records
        .iter()
        .enumerate()
        .map(|(i, record)| {
            let record_ref = &records[i];
            let text = full_text
                .get(&record.path)
                .map(String::as_str)
                .unwrap_or_default();
            let (status, state, decided_by) = standings[i].clone();
            let waiting_on = ids[i]
                .as_deref()
                .and_then(|id| waits.get(RecordId::new(id).as_str()))
                .map(|deps| natural_sorted(deps.iter().filter_map(|d| display(d)).collect()))
                .unwrap_or_default();
            let mut by_state = ChildrenByState::default();
            for child in &tree.children[i] {
                by_state.add(standings[*child].1);
            }
            let mut children: Vec<String> = tree.children[i]
                .iter()
                .filter_map(|c| ids[*c].clone())
                .collect();
            children = natural_sorted(children);
            let (title, title_is_fallback) =
                match title_in_text(record, text).filter(|t| !t.is_empty()) {
                    Some(title) => (title, false),
                    None => (
                        ids[i].clone().unwrap_or_else(|| {
                            record
                                .path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default()
                        }),
                        true,
                    ),
                };
            let is_work = roles[&record.record_type].work;
            RecordView {
                title,
                title_is_fallback,
                fields: fields_of(config, record_ref, text),
                gated_moves: if state.is_open() {
                    gated_moves(
                        config,
                        decls.get(&record.record_type),
                        record,
                        status.as_deref(),
                    )
                } else {
                    Vec::new()
                },
                ready: is_work && state.is_open() && waiting_on.is_empty(),
                waiting_on,
                state_disagrees_with_children: disagrees(state, &by_state),
                children_by_state: by_state,
                children,
                parent: tree.parent[i].and_then(|p| ids[p].clone()),
                refers_to: edge_maps.refers_to[i].clone(),
                unresolved_refs: edge_maps.unresolved[i].clone(),
                referred_by: edge_maps.referred_by[i].clone(),
                link: config.export.link_for(&record.path),
                status,
                state,
                state_decided_by: decided_by,
            }
        })
        .collect();
    ViewModel {
        records: views,
        roles,
    }
}

/// The counts of ticked and unticked checkbox lines in a section's text,
/// outside fenced code.
pub fn checkbox_counts(content: &str) -> (usize, usize) {
    let (mut ticked, mut unticked) = (0, 0);
    let mut fence: Option<char> = None;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = ['`', '~']
            .into_iter()
            .find(|m| trimmed.starts_with([*m; 3]))
        {
            fence = match fence {
                Some(open) if open == marker => None,
                None => Some(marker),
                other => other,
            };
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let item = trimmed
            .strip_prefix(['-', '*', '+'])
            .filter(|rest| rest.starts_with(' '))
            .map(str::trim_start);
        let boxed = item.filter(|rest| {
            rest[3.min(rest.len())..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
        });
        match boxed.and_then(|rest| rest.get(..3)) {
            Some("[ ]") => unticked += 1,
            Some("[x]" | "[X]") => ticked += 1,
            _ => {}
        }
    }
    (ticked, unticked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_lines_are_counted_outside_fences() {
        let text = "- [x] one\n- [ ] two\n* [X] three\n+ [ ] four\n- [ ]no space\n- plain\n```\n- [ ] in a fence\n```\n  - [x] nested\n";
        assert_eq!(checkbox_counts(text), (3, 2));
    }

    #[test]
    fn a_state_is_not_open_once_finished_or_dropped() {
        assert!(StateCategory::UnderWay.is_open());
        assert!(StateCategory::NoStatuses.is_open());
        assert!(!StateCategory::Finished.is_open());
        assert!(!StateCategory::Dropped.is_open());
    }

    #[test]
    fn a_parent_disagrees_with_a_child_that_contradicts_its_state() {
        let by = |finished, under_way, not_started| ChildrenByState {
            finished,
            under_way,
            not_started,
            ..ChildrenByState::default()
        };
        assert!(disagrees(StateCategory::Finished, &by(1, 0, 1)));
        assert!(!disagrees(StateCategory::Finished, &by(2, 0, 0)));
        assert!(disagrees(StateCategory::NotStarted, &by(0, 1, 1)));
        assert!(disagrees(StateCategory::NotStarted, &by(1, 0, 1)));
        assert!(!disagrees(StateCategory::NotStarted, &by(0, 0, 2)));
        assert!(!disagrees(StateCategory::UnderWay, &by(0, 0, 2)));
        assert!(!disagrees(StateCategory::NoStatuses, &by(1, 1, 1)));
    }
}
