//! `diff.decider-required` (`ADR-89` section 4, `MILE-200`): a gated move of a
//! record's status, or a change under a gated path, needs host evidence that
//! names a login in the gate's decider group.
//!
//! The evidence is authorship (`pr-author`) or a current approval
//! (`pr-approver`), read from the actors file the gate action writes. It is
//! never fetched here. With a single maintainer every pull request an agent
//! opens under the maintainer's token is authored by a decider, so this rule
//! cannot tell an agent from the maintainer: what it gives is a recorded trail
//! and, where a second human or a distinct agent identity exists, a check that
//! means something.

use super::*;
use crate::authority::{Actors, AuthorityDecl, BaseAuthority, DeciderTable, EvidenceKind, Gate};
use crate::config::RelationRole;
use crate::report::{census, Outcome};

/// What the actors file says, in words, for the kinds of evidence the table
/// accepts.
fn evidence_text(authority: &AuthorityDecl, actors: &Actors) -> String {
    let parts: Vec<String> = authority
        .evidence
        .iter()
        .map(|kind| match kind {
            EvidenceKind::PrAuthor => match &actors.pr_author {
                Some(a) => format!("the pull request author is '{a}'"),
                None => "the pull request author is not known".to_string(),
            },
            EvidenceKind::PrApprover => {
                if actors.approvers.is_empty() {
                    "no approval is current for the head commit".to_string()
                } else {
                    format!(
                        "the current approvers are {}",
                        actors
                            .approvers
                            .iter()
                            .map(|a| format!("'{a}'"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        })
        .collect();
    parts.join("; ")
}

fn kinds_text(authority: &AuthorityDecl) -> String {
    authority
        .evidence
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(" or ")
}

/// One gated change found in the diff.
struct Gated<'a> {
    path: &'a Path,
    gate: &'a Gate,
    /// What changed, for the message: a status move or a protected path.
    what: String,
    variant: &'static str,
}

/// The status a changed record holds now, by its type's declared status field.
fn current_status<'r>(record: &'r Record, config: &Config) -> Option<&'r str> {
    let field = relation_field_name(config, &record.record_type, RelationRole::Status);
    record
        .header
        .get(field)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn gated_changes<'a>(
    paths: &[&'a PathBuf],
    records: &'a [Record],
    config: &Config,
    authority: &'a AuthorityDecl,
    committed: Option<&HashMap<PathBuf, CommittedStatus>>,
) -> Vec<Gated<'a>> {
    let mut out = Vec::new();
    for path in paths {
        if let Some(record) = records.iter().find(|r| &r.path == *path) {
            if let Some(status) = current_status(record, config) {
                let field = relation_field_name(config, &record.record_type, RelationRole::Status);
                for gate in authority.status_gates(&record.record_type, status) {
                    let what = match committed.and_then(|c| c.get(&record.path)) {
                        Some(CommittedStatus::Status(old)) if old == status => continue,
                        Some(CommittedStatus::Status(old)) => {
                            format!("{field} moved from '{old}' to '{status}'")
                        }
                        Some(CommittedStatus::New) => {
                            format!("the record is new and starts at {field} '{status}'")
                        }
                        Some(CommittedStatus::Unreadable) | None => format!(
                            "{field} is '{status}' and the merge base's copy could not be read, so it is treated as a move"
                        ),
                    };
                    out.push(Gated {
                        path,
                        gate,
                        what,
                        variant: "gated-move-without-decider",
                    });
                }
            }
        }
        for gate in authority.gates.iter().filter(|g| g.is_path_gate()) {
            if gate.paths.iter().any(|e| path_matches_entry(path, e)) {
                out.push(Gated {
                    path,
                    gate,
                    what: "a protected path changed".to_string(),
                    variant: "gated-path-without-decider",
                });
            }
        }
    }
    out
}

/// One table a run judges against, and what to call it in a message: `None`
/// under `head` alone, so that mode's findings read as they always have.
struct Judged<'a> {
    table: &'a AuthorityDecl,
    label: Option<&'static str>,
}

/// The tables `mode` judges against. A base with no config or no `authority`
/// names no decider and no gate, so a branch cut before the table existed could
/// declare its own and have it waved through: that base is no table to judge
/// against, and the head table is judged instead (the caller says so in a
/// notice). An unreadable base falls back to the head table the same way.
fn judged_tables<'a>(
    mode: DeciderTable,
    head: &'a AuthorityDecl,
    base: &'a BaseAuthority,
) -> Vec<Judged<'a>> {
    let head_j = |label| Judged { table: head, label };
    match (mode, base) {
        (DeciderTable::Head, _) => vec![head_j(None)],
        (_, BaseAuthority::Absent) => vec![head_j(Some("the head's table"))],
        (_, BaseAuthority::Unreadable) => vec![head_j(None)],
        (DeciderTable::Base, BaseAuthority::Declared(table)) => vec![Judged {
            table,
            label: Some("the base revision's table"),
        }],
        (DeciderTable::Both, BaseAuthority::Declared(table)) => vec![
            head_j(Some("the head's table")),
            Judged {
                table,
                label: Some("the base revision's table"),
            },
        ],
    }
}

fn finding_for(
    g: &Gated<'_>,
    judged: &Judged<'_>,
    actors: Option<&Actors>,
    opts: &DeciderRequiredOpts,
) -> Option<Finding> {
    let group = &g.gate.by;
    let gate_text = match judged.label {
        Some(label) => format!("{}, in {label}", g.gate.describe()),
        None => g.gate.describe(),
    };
    let (message, declared_severity) = match actors {
        Some(actors) if judged.table.satisfied(g.gate, actors) => return None,
        Some(actors) => (
            format!(
                "{}: {} (gate: {gate_text}), and {} -- none is in the decider group '{group}' (evidence accepted: {})",
                g.path.display(),
                g.what,
                evidence_text(judged.table, actors),
                kinds_text(judged.table)
            ),
            None,
        ),
        None => (
            format!(
                "{}: {} (gate: {gate_text}), but no --actors file was given, so who made it is not known and the decider group '{group}' cannot be checked; run `urzua check --base REF --actors FILE` (the gate action writes FILE)",
                g.path.display(),
                g.what,
            ),
            (!opts.require_evidence).then_some(FindingSeverity::Info),
        ),
    };
    Some(
        Finding::new(
            RULE_DIFF_DECIDER_REQUIRED,
            FindingSeverity::Error,
            g.path.to_path_buf(),
            ActionSource::new(g.variant),
            message,
        )
        .declared_opt(declared_severity),
    )
}

/// A gated move or protected-path change whose author or current approvers are
/// not in the gate's group (`ADR-89` section 4). `changed` is `None` on a run
/// with no `--base`: the rule then reports not-applicable, never clean. With no
/// `actors` file a gated change is reported as `info`, or as an error under
/// `require_evidence`, so an absent input is never a silent pass.
///
/// Which table is the gate's is `authority.decider_table` (`ADR-89` amendment
/// of 2026-11-09): `head` (undeclared) reads the checkout's, `base` reads the
/// base revision's, `both` judges each gated change under both.
pub fn decider_required(
    changed: Option<&[PathBuf]>,
    records: &[Record],
    config: &Config,
    committed: Option<&HashMap<PathBuf, CommittedStatus>>,
    actors: Option<&Actors>,
    base: &BaseAuthority,
    opts: &DeciderRequiredOpts,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_DIFF_DECIDER_REQUIRED;
    let (Some(changed), Some(head)) = (changed, config.authority.as_ref()) else {
        return not_applicable(RULE_ID);
    };
    let mut paths: Vec<&PathBuf> = changed.iter().collect();
    paths.sort();
    paths.dedup();
    let mode = head.effective_table(base);
    let mut findings = Vec::new();
    let mut gated_paths: HashSet<&Path> = HashSet::new();
    for judged in judged_tables(mode, head, base) {
        let gated = gated_changes(&paths, records, config, judged.table, committed);
        for g in &gated {
            gated_paths.insert(g.path);
            findings.extend(finding_for(g, &judged, actors, opts));
        }
    }
    let population = census(PopulationUnit::ChangedPath, paths, |path| {
        if gated_paths.contains(path.as_path()) {
            Outcome::Examined
        } else {
            Outcome::Absent
        }
    });
    (
        RuleExecution::ran(RULE_ID, population, Vec::new()),
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        crate::config::parse(
            "schema_version: 2\nrecord_types:\n  adr:\n    dir: docs/adr\n    statuses: {values: [Proposed, Accepted, Rejected]}\nauthority:\n  deciders:\n    m: {logins: [alice]}\n  gates:\n    - {type: adr, to: [Accepted], by: m}\n    - {paths: [.agent-config], by: m}\n",
        )
        .unwrap()
    }

    fn adr(status: &str) -> Record {
        Record::parse(
            PathBuf::from("docs/adr/ADR-1-a.md"),
            "adr".to_string(),
            &format!("---\nStatus: {status}\n---\n# ADR-1: T\n"),
        )
    }

    fn committed(c: CommittedStatus) -> HashMap<PathBuf, CommittedStatus> {
        HashMap::from([(PathBuf::from("docs/adr/ADR-1-a.md"), c)])
    }

    fn actors(author: &str) -> Actors {
        Actors {
            pr_author: Some(author.to_string()),
            approvers: vec![],
        }
    }

    fn run(
        changed: &[&str],
        status: &str,
        base: CommittedStatus,
        who: Option<&Actors>,
        require: bool,
    ) -> (RuleExecution, Vec<Finding>) {
        let changed: Vec<PathBuf> = changed.iter().map(PathBuf::from).collect();
        decider_required(
            Some(&changed),
            &[adr(status)],
            &config(),
            Some(&committed(base)),
            who,
            &BaseAuthority::Absent,
            &DeciderRequiredOpts {
                require_evidence: require,
            },
        )
    }

    const REC: &str = "docs/adr/ADR-1-a.md";

    #[test]
    fn no_diff_is_not_applicable_not_clean() {
        let (exec, findings) = decider_required(
            None,
            &[],
            &config(),
            None,
            None,
            &BaseAuthority::Absent,
            &DeciderRequiredOpts::default(),
        );
        assert_eq!(exec.status, RuleStatus::NotApplicable);
        assert!(findings.is_empty());
    }

    #[test]
    fn a_decider_author_passes_and_anyone_else_is_an_error() {
        let base = || CommittedStatus::Status("Proposed".into());
        let (_, ok) = run(&[REC], "Accepted", base(), Some(&actors("Alice")), false);
        assert!(ok.is_empty());
        let (exec, bad) = run(&[REC], "Accepted", base(), Some(&actors("mallory")), false);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].declared_severity, None);
        assert!(bad[0].message.contains("'mallory'") && bad[0].message.contains("'m'"));
        let pop = exec.population.unwrap();
        assert_eq!(pop.examined(), 1);
    }

    #[test]
    fn a_status_that_did_not_move_or_is_not_gated_is_not_judged() {
        let (_, f) = run(
            &[REC],
            "Accepted",
            CommittedStatus::Status("Accepted".into()),
            Some(&actors("mallory")),
            false,
        );
        assert!(
            f.is_empty(),
            "an Accepted record edited in place is not a move"
        );
        let (_, f) = run(
            &[REC],
            "Proposed",
            CommittedStatus::Status("Accepted".into()),
            Some(&actors("mallory")),
            false,
        );
        assert!(f.is_empty(), "Proposed is not a gated status");
        let (_, f) = run(
            &["docs/other.md"],
            "Accepted",
            CommittedStatus::New,
            Some(&actors("mallory")),
            false,
        );
        assert!(f.is_empty(), "the record is not in the diff");
    }

    #[test]
    fn a_new_record_and_an_unreadable_base_count_as_a_move() {
        for base in [CommittedStatus::New, CommittedStatus::Unreadable] {
            let (_, f) = run(&[REC], "Accepted", base, Some(&actors("mallory")), false);
            assert_eq!(f.len(), 1);
        }
    }

    #[test]
    fn a_protected_path_needs_the_group() {
        let (_, f) = run(
            &[".agent-config/settings.json"],
            "Proposed",
            CommittedStatus::Status("Proposed".into()),
            Some(&actors("mallory")),
            false,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].action_source.variant, "gated-path-without-decider");
        let (_, f) = run(
            &[".agent-config/settings.json"],
            "Proposed",
            CommittedStatus::Status("Proposed".into()),
            Some(&actors("alice")),
            false,
        );
        assert!(f.is_empty());
    }

    #[test]
    fn no_actors_is_info_unless_evidence_is_required() {
        let base = || CommittedStatus::Status("Proposed".into());
        let (_, f) = run(&[REC], "Accepted", base(), None, false);
        assert_eq!(f[0].declared_severity, Some(FindingSeverity::Info));
        assert!(f[0].message.contains("--actors"));
        let (_, f) = run(&[REC], "Accepted", base(), None, true);
        assert_eq!(f[0].declared_severity, None);
    }
}
