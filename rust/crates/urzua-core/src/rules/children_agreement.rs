use super::*;
use crate::status_model::StatusDecl;
use crate::view_model::{parent_standings, ChildrenByState, Disagreement, ParentStanding};

/// How many contradicting children a finding names.
const NAMED_CHILDREN: usize = 5;

/// A record's `Status` contradicts the records that are part of it: not started
/// while a child is under way or finished, or finished while a child is neither
/// finished nor dropped. Every state comes from `crate::view_model`, the same
/// functions the export's `state_disagrees_with_children` is computed with. A
/// parent whose state is `open` or `none` (nothing says whether work has
/// begun) is out of scope; so is a record whose `part_of` field names several
/// records, which is nobody's child.
pub fn status_children_agreement(
    records: &[Record],
    config: &Config,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_STATUS_CHILDREN_AGREEMENT;
    let declared = statuses_by_type(config);
    let mut findings = Vec::new();
    let standings: HashMap<&Path, ParentStanding<'_>> = parent_standings(config, records)
        .into_iter()
        .map(|standing| (standing.parent.path.as_path(), standing))
        .collect();
    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Record,
        records.iter().collect(),
        |record| record,
        |record| {
            use crate::view_model::StateCategory::{NoStatuses, Open};
            let Some(standing) = standings.get(record.path.as_path()) else {
                return Outcome::Absent;
            };
            if matches!(standing.state, Open | NoStatuses) {
                return Outcome::OutOfScope;
            }
            if let Some(kind) = standing.disagreement {
                findings.push(finding(
                    config,
                    declared.get(&record.record_type),
                    standing,
                    kind,
                ));
            }
            Outcome::Examined
        },
    );
    (
        RuleExecution::ran(RULE_ID, population, examined_records),
        findings,
    )
}

fn finding(
    config: &Config,
    decl: Option<&StatusDecl>,
    standing: &ParentStanding<'_>,
    kind: Disagreement,
) -> Finding {
    let parent = standing.parent;
    let status_field = relation_field_name(
        config,
        &parent.record_type,
        crate::config::RelationRole::Status,
    );
    let status = standing.status.as_deref().unwrap_or_default();
    let id = record_id(parent).unwrap_or_else(|| parent.path.display().to_string());
    let counts = counts_text(&standing.children_by_state);
    let named = named_children(&standing.contradicting);
    let start = (kind == Disagreement::NotStartedButChildBegun)
        .then(|| start_move(config, decl, parent, status, status_field));
    let source = match &start {
        Some((source, _)) => source.clone(),
        None => ActionSource::new("children-parent-finished"),
    };
    let message = match &start {
        Some((_, move_note)) => format!(
            "{id} is {status_field} '{status}', not started, while a child has begun: {counts} ({named}){move_note}"
        ),
        None => format!(
            "{id} is {status_field} '{status}', which finishes it, while a child is not finished or dropped: {counts} ({named})"
        ),
    };
    Finding::new(
        RULE_STATUS_CHILDREN_AGREEMENT,
        FindingSeverity::Error,
        parent.path.clone(),
        source,
        message,
    )
}

/// The action for a parent that has not started while a child has, and what
/// the finding adds about the move. The edit is offered only where the type
/// declares exactly one progress status, its transitions lead there, and no
/// gate reserves the move for a person.
fn start_move(
    config: &Config,
    decl: Option<&StatusDecl>,
    parent: &Record,
    status: &str,
    status_field: &str,
) -> (ActionSource, String) {
    let progress = decl.map(|d| d.progress.as_slice()).unwrap_or_default();
    let [only] = progress else {
        return (
            ActionSource::new("children-parent-not-started-choice"),
            "; the type declares no single progress status to move it to".to_string(),
        );
    };
    let gate = config
        .authority
        .as_ref()
        .and_then(|a| a.status_gates(&parent.record_type, only).next());
    if let Some(gate) = gate {
        return (
            ActionSource::new("children-parent-not-started-gated"),
            format!("; moving it to '{only}' is reserved for {}", gate.by),
        );
    }
    let reachable = decl.is_none_or(|d| !d.has_transitions() || d.allows(status, only));
    if !reachable {
        return (
            ActionSource::new("children-parent-not-started-choice"),
            format!("; its declared transitions do not lead from '{status}' to '{only}'"),
        );
    }
    (
        ActionSource::new("children-parent-not-started")
            .field(status_field)
            .text(only.clone()),
        format!("; set {status_field} to '{only}'"),
    )
}

fn counts_text(by: &ChildrenByState) -> String {
    [
        ("finished", by.finished),
        ("dropped", by.dropped),
        ("under-way", by.under_way),
        ("not-started", by.not_started),
        ("open", by.open),
        ("none", by.none),
    ]
    .iter()
    .filter(|(_, n)| *n > 0)
    .map(|(state, n)| format!("{n} {state}"))
    .collect::<Vec<_>>()
    .join(", ")
}

fn named_children(children: &[&Record]) -> String {
    let ids: Vec<String> = children
        .iter()
        .take(NAMED_CHILDREN)
        .map(|c| record_id(c).unwrap_or_else(|| c.path.display().to_string()))
        .collect();
    match children.len().saturating_sub(NAMED_CHILDREN) {
        0 => ids.join(", "),
        more => format!("{}, +{more} more", ids.join(", ")),
    }
}

/// What the rule cannot do in this configuration, said once: it is on, and no
/// type declares a `part_of` field, or the types a `part_of` field names declare
/// no `statuses.progress`, so no parent can be not started or under way.
pub fn status_children_agreement_notices(config: &Config) -> Vec<Notice> {
    if effective_level(config, RULE_STATUS_CHILDREN_AGREEMENT) == crate::config::RuleLevel::Off {
        return Vec::new();
    }
    let notice = |message: String| Notice {
        severity: NoticeSeverity::Warning,
        subject: RULE_STATUS_CHILDREN_AGREEMENT.to_string(),
        message,
        detail: None,
    };
    if config.part_of.is_empty() {
        return vec![notice(format!(
            "{RULE_STATUS_CHILDREN_AGREEMENT} is enabled but no record type declares a `part_of: true` pointer field, so no record is part of another and it examines nothing"
        ))];
    }
    let declared = statuses_by_type(config);
    let parent_types: std::collections::BTreeSet<&str> = config
        .part_of
        .iter()
        .filter_map(|(record_type, field)| config.pointer_targets.get(record_type)?.get(field))
        .flatten()
        .map(String::as_str)
        .collect();
    let without: Vec<&str> = parent_types
        .iter()
        .copied()
        .filter(|t| declared.get(*t).is_none_or(|d| d.progress.is_empty()))
        .collect();
    if without.is_empty() {
        return Vec::new();
    }
    vec![notice(format!(
        "{RULE_STATUS_CHILDREN_AGREEMENT} is enabled but record type(s) {} declare no `statuses.progress`, so a parent of those types is never not started or under way and the rule judges only a finished one with an open child (none when the type declares no `statuses.success` either)",
        without.join(", ")
    ))]
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUSES: &str = "      values: [Planned, Active, Done, Dropped]
      initial: [Planned]
      progress: [Active]
      success: [Done]
      failure: [Dropped]
      transitions:
        Planned: [Active, Dropped]
        Active: [Done, Dropped]
        Done: [Active]
";

    fn config_yaml(statuses: &str, extra: &str) -> String {
        format!(
            "schema_version: 2
rules:
  status.children-agreement: error
record_types:
  work:
    dir: docs/work
    prefix: WORK
    header_shape: yaml-frontmatter
    required_fields: [Status]
    known_fields: [Part-of]
    narrative_fields: []
    pointer_fields:
      - {{name: Part-of, targets: [work], part_of: true}}
    statuses:
{statuses}{extra}"
        )
    }

    fn config(extra: &str) -> Config {
        crate::config::parse(&config_yaml(STATUSES, extra)).unwrap()
    }

    fn work(n: u32, status: &str, part_of: &str) -> Record {
        let line = if part_of.is_empty() {
            String::new()
        } else {
            format!("Part-of: {part_of}\n")
        };
        Record::parse_with_shape_and_prefix(
            PathBuf::from(format!("docs/work/WORK-{n}-x.md")),
            "work".to_string(),
            &format!("---\nStatus: {status}\n{line}---\n# {n} - X\n"),
            crate::header::HeaderShape::YamlFrontmatter,
            "WORK".to_string(),
        )
    }

    fn run(config: &Config, records: &[Record]) -> Vec<Finding> {
        let (_, mut findings) = status_children_agreement(records, config);
        resolve_actions(RULE_STATUS_CHILDREN_AGREEMENT, &mut findings);
        findings
    }

    #[test]
    fn a_not_started_parent_with_a_begun_child_is_reported_on_the_parent() {
        for child in ["Active", "Done"] {
            let records = [work(1, "Planned", ""), work(2, child, "WORK-1")];
            let found = run(&config(""), &records);
            assert_eq!(found.len(), 1, "{found:?}");
            assert_eq!(found[0].file, PathBuf::from("docs/work/WORK-1-x.md"));
            let m = &found[0].message;
            assert!(
                m.contains("WORK-1") && m.contains("'Planned'") && m.contains("WORK-2"),
                "{m}"
            );
        }
    }

    #[test]
    fn the_message_counts_the_children_by_state_and_names_up_to_five() {
        let mut records = vec![work(1, "Planned", "")];
        for n in 2..=8 {
            records.push(work(n, "Active", "WORK-1"));
        }
        records.push(work(9, "Done", "WORK-1"));
        records.push(work(10, "Planned", "WORK-1"));
        let found = run(&config(""), &records);
        let m = &found[0].message;
        assert!(m.contains("1 finished, 7 under-way, 1 not-started"), "{m}");
        assert!(
            m.contains("WORK-2, WORK-3, WORK-4, WORK-5, WORK-6, +3 more"),
            "{m}"
        );
        assert!(!m.contains("WORK-7,"), "{m}");
    }

    #[test]
    fn a_finished_parent_with_an_unfinished_child_is_reported() {
        for child in ["Planned", "Active"] {
            let records = [
                work(1, "Done", ""),
                work(2, child, "WORK-1"),
                work(3, "Done", "WORK-1"),
            ];
            let found = run(&config(""), &records);
            assert_eq!(found.len(), 1, "{found:?}");
            assert!(found[0].message.contains("'Done'") && found[0].message.contains("WORK-2"));
            assert!(
                !found[0].message.contains("WORK-3)"),
                "{}",
                found[0].message
            );
            assert!(found[0].suggested_action.is_none());
        }
    }

    #[test]
    fn a_finished_parent_whose_children_are_finished_or_dropped_agrees() {
        let records = [
            work(1, "Done", ""),
            work(2, "Done", "WORK-1"),
            work(3, "Dropped", "WORK-1"),
        ];
        assert!(run(&config(""), &records).is_empty());
    }

    #[test]
    fn finishing_every_child_while_the_parent_is_under_way_is_not_reported() {
        let records = [
            work(1, "Active", ""),
            work(2, "Done", "WORK-1"),
            work(3, "Done", "WORK-1"),
        ];
        assert!(run(&config(""), &records).is_empty());
    }

    #[test]
    fn a_dropped_parent_is_not_reported_for_any_child() {
        for child in ["Planned", "Active", "Done"] {
            let records = [work(1, "Dropped", ""), work(2, child, "WORK-1")];
            assert!(run(&config(""), &records).is_empty(), "{child}");
        }
    }

    #[test]
    fn an_under_way_parent_and_a_not_started_one_with_not_started_children_agree() {
        let records = [
            work(1, "Planned", ""),
            work(2, "Planned", "WORK-1"),
            work(3, "Dropped", "WORK-1"),
        ];
        assert!(run(&config(""), &records).is_empty());
        let records = [work(1, "Active", ""), work(2, "Planned", "WORK-1")];
        assert!(run(&config(""), &records).is_empty());
    }

    #[test]
    fn a_parent_of_a_type_that_declares_no_progress_is_never_not_started() {
        let statuses = "      values: [Planned, Done]\n      success: [Done]\n";
        let config = crate::config::parse(&config_yaml(statuses, "")).unwrap();
        let records = [work(1, "Planned", ""), work(2, "Done", "WORK-1")];
        let (exec, found) = status_children_agreement(&records, &config);
        assert!(found.is_empty(), "{found:?}");
        let population = exec.population.unwrap();
        assert_eq!((population.eligible(), population.examined()), (2, 0));
    }

    #[test]
    fn a_record_naming_two_parents_is_nobodys_child() {
        let records = [
            work(1, "Planned", ""),
            work(2, "Planned", ""),
            work(3, "Done", "WORK-1, WORK-2"),
        ];
        assert!(run(&config(""), &records).is_empty());
    }

    #[test]
    fn a_parent_with_an_unreadable_header_is_not_judged() {
        let broken = Record::parse_with_shape_and_prefix(
            PathBuf::from("docs/work/WORK-1-x.md"),
            "work".to_string(),
            "---\nStatus: [unclosed\n---\n# 1 - X\n",
            crate::header::HeaderShape::YamlFrontmatter,
            "WORK".to_string(),
        );
        assert!(run(&config(""), &[broken, work(2, "Active", "WORK-1")]).is_empty());
    }

    #[test]
    fn the_start_move_is_offered_as_a_status_edit_when_one_progress_status_leads_there() {
        let found = run(
            &config(""),
            &[work(1, "Planned", ""), work(2, "Active", "WORK-1")],
        );
        let action = found[0].suggested_action.as_ref().unwrap();
        assert_eq!(action.kind, crate::report::ActionKind::EditField);
        assert_eq!(action.op, Some(crate::report::ActionOp::Set));
        assert_eq!(action.target.field.as_deref(), Some("Status"));
        assert_eq!(
            action.value,
            Some(crate::report::ActionValue::Text("Active".to_string()))
        );
    }

    #[test]
    fn a_gated_start_move_is_never_offered_and_names_who_may_make_it() {
        let config = config(
            "authority:
  deciders:
    leads: {logins: [someone]}
  gates:
    - {type: work, to: [Active], by: leads}
",
        );
        let found = run(
            &config,
            &[work(1, "Planned", ""), work(2, "Active", "WORK-1")],
        );
        assert!(found[0].suggested_action.is_none());
        assert_eq!(
            found[0].action_source.variant,
            "children-parent-not-started-gated"
        );
        assert!(
            found[0].message.contains("reserved for leads"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn several_progress_statuses_or_no_transition_leave_the_choice_to_the_author() {
        let two = "      values: [Planned, Active, Review, Done]
      initial: [Planned]
      progress: [Active, Review]
      success: [Done]
      transitions:
        Planned: [Active, Review]
        Active: [Done]
        Review: [Done]
";
        let config = crate::config::parse(&config_yaml(two, "")).unwrap();
        let found = run(
            &config,
            &[work(1, "Planned", ""), work(2, "Active", "WORK-1")],
        );
        assert!(found[0].suggested_action.is_none());
        assert_eq!(
            found[0].action_source.variant,
            "children-parent-not-started-choice"
        );

        let unreachable = STATUSES.replace("Planned: [Active, Dropped]", "Planned: [Dropped]");
        let config = crate::config::parse(&config_yaml(&unreachable, "")).unwrap();
        let found = run(
            &config,
            &[work(1, "Planned", ""), work(2, "Active", "WORK-1")],
        );
        assert_eq!(
            found[0].action_source.variant,
            "children-parent-not-started-choice"
        );
    }

    #[test]
    fn the_notices_say_once_what_the_configuration_cannot_support() {
        let none = crate::config::parse(
            "schema_version: 2
rules:
  status.children-agreement: error
record_types:
  work:
    dir: docs/work
    prefix: WORK
    required_fields: [Status]
    statuses:
      values: [Open, Done]
",
        )
        .unwrap();
        let notices = status_children_agreement_notices(&none);
        assert_eq!(notices.len(), 1);
        assert!(notices[0]
            .message
            .contains("no record type declares a `part_of: true`"));

        let statuses = "      values: [Planned, Done]\n      success: [Done]\n";
        let no_progress = crate::config::parse(&config_yaml(statuses, "")).unwrap();
        let notices = status_children_agreement_notices(&no_progress);
        assert_eq!(notices.len(), 1);
        assert!(notices[0]
            .message
            .contains("work declare no `statuses.progress`"));

        assert!(status_children_agreement_notices(&config("")).is_empty());
    }

    #[test]
    fn nothing_is_said_when_the_rule_is_not_enabled() {
        let yaml =
            config_yaml(STATUSES, "").replace("rules:\n  status.children-agreement: error\n", "");
        let off = crate::config::parse(&yaml).unwrap();
        assert!(status_children_agreement_notices(&off).is_empty());
        let no_part_of = crate::config::parse("schema_version: 2\nrecord_types: {}\n").unwrap();
        assert!(status_children_agreement_notices(&no_part_of).is_empty());
    }
}
