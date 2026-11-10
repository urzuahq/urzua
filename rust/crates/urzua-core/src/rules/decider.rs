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
    crate::authority::gate_status(&record.header, field)
}

/// What the merge base says about the records the diff touches: each changed
/// record's committed status, and the records the diff deletes.
#[derive(Clone, Copy)]
pub struct BaseHistory<'a> {
    pub committed: Option<&'a HashMap<PathBuf, CommittedStatus>>,
    pub deleted: &'a [DeletedRecord],
}

/// A record the diff deletes, as the merge base held it (`ADR-89` amendment of
/// 2026-11-10, deletions). The caller leaves out a record that pairs as a
/// rename (the same `Stable-Id` and record id still exists in the head), so
/// every entry here is a record that is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedRecord {
    pub path: PathBuf,
    pub record_type: String,
    /// Its `Status` at the base; `None` where the base copy has no readable one.
    pub status: Option<String>,
}

fn gated_changes<'a>(
    paths: &[&'a PathBuf],
    records: &'a [Record],
    config: &Config,
    authority: &'a AuthorityDecl,
    committed: Option<&HashMap<PathBuf, CommittedStatus>>,
    deleted: &'a [DeletedRecord],
) -> Vec<Gated<'a>> {
    let mut out = Vec::new();
    for d in deleted {
        let Some(status) = d.status.as_deref() else {
            continue;
        };
        let field = relation_field_name(config, &d.record_type, RelationRole::Status);
        for gate in authority.status_gates(&d.record_type, status) {
            out.push(Gated {
                path: &d.path,
                gate,
                what: format!("the record was deleted while {field} was '{status}'"),
                variant: "gated-deletion-without-decider",
            });
        }
    }
    for path in paths {
        if let Some(record) = records.iter().find(|r| &r.path == *path) {
            if let Some(status) = current_status(record, config) {
                let field = relation_field_name(config, &record.record_type, RelationRole::Status);
                for gate in authority.status_gates(&record.record_type, status) {
                    let what = match committed.and_then(|c| c.get(&record.path)) {
                        Some(CommittedStatus::Status(old))
                            if crate::authority::same_status(old, status) =>
                        {
                            continue
                        }
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
///
/// A record the diff deletes (`deleted`) while its base `Status` is one a gate
/// names is a gated move too, judged the same way.
pub fn decider_required(
    changed: Option<&[PathBuf]>,
    records: &[Record],
    config: &Config,
    history: &BaseHistory<'_>,
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
        let gated = gated_changes(
            &paths,
            records,
            config,
            judged.table,
            history.committed,
            history.deleted,
        );
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
            &BaseHistory {
                committed: Some(&committed(base)),
                deleted: &[],
            },
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
            &BaseHistory {
                committed: None,
                deleted: &[],
            },
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

    fn deleted(status: Option<&str>) -> Vec<DeletedRecord> {
        vec![DeletedRecord {
            path: PathBuf::from(REC),
            record_type: "adr".into(),
            status: status.map(str::to_string),
        }]
    }

    fn run_deleted(status: Option<&str>, who: &Actors) -> Vec<Finding> {
        decider_required(
            Some(&[PathBuf::from(REC)]),
            &[],
            &config(),
            &BaseHistory {
                committed: Some(&HashMap::new()),
                deleted: &deleted(status),
            },
            Some(who),
            &BaseAuthority::Absent,
            &DeciderRequiredOpts::default(),
        )
        .1
    }

    #[test]
    fn deleting_a_record_at_a_gated_status_is_a_gated_move() {
        let f = run_deleted(Some("Accepted"), &actors("mallory"));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].action_source.variant, "gated-deletion-without-decider");
        assert!(f[0].message.contains("deleted while Status was 'Accepted'"));
        assert!(run_deleted(Some("Accepted"), &actors("alice")).is_empty());
        assert!(run_deleted(Some("accepted"), &actors("mallory")).len() == 1);
        assert!(run_deleted(Some("Proposed"), &actors("mallory")).is_empty());
        assert!(run_deleted(None, &actors("mallory")).is_empty());
    }

    #[test]
    fn a_case_variant_of_a_gated_status_is_still_gated() {
        for status in ["accepted", "ACCEPTED", " Accepted "] {
            let (_, f) = run(
                &[REC],
                status,
                CommittedStatus::Status("Proposed".into()),
                Some(&actors("mallory")),
                false,
            );
            assert_eq!(f.len(), 1, "{status}");
        }
        let (_, f) = run(
            &[REC],
            "accepted",
            CommittedStatus::Status("Accepted".into()),
            Some(&actors("mallory")),
            false,
        );
        assert!(f.is_empty(), "a case-only difference is not a move");
    }
}
