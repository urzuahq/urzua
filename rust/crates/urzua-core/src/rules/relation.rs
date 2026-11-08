use super::*;

/// Rule 5 (Phase 6): `Supersedes`/`Superseded-by` reciprocity. Checking only
/// the forward claim leaves the reverse unguarded -- a record can claim to
/// supersede something that doesn't reciprocally point back, sending a
/// reader of the *target* to a record that denies the relation.
pub fn supersession_reciprocity(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
    untracked: &UntrackedRecords,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();

    // A record whose filename yields no id is eligible but unexaminable: the
    // reciprocity test is "does the target name *me* back", which needs an id.
    let slots = declared_slots_for_roles(
        records,
        config,
        &[crate::config::RelationRole::Supersession],
    );
    let check = SupersessionCheck {
        config,
        index,
        untracked,
    };

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |record| *record,
        |record| check.judge(record, &mut findings),
    );

    (
        RuleExecution {
            rule: RULE_RELATION_SUPERSESSION_RECIPROCITY.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// What `relation.supersession-reciprocity` resolves a claim against.
struct SupersessionCheck<'a> {
    config: &'a Config,
    index: &'a RecordIndex<'a>,
    untracked: &'a UntrackedRecords,
}

/// A record claiming a supersession: the record, the field it writes the claim
/// in, and its id as written and normalised.
struct Claimant<'a> {
    record: &'a Record,
    field: &'a str,
    id: String,
    normalized_id: crate::values::RecordId,
}

impl SupersessionCheck<'_> {
    fn judge(&self, record: &Record, findings: &mut Vec<Finding>) -> Outcome {
        let field = relation_field_name(
            self.config,
            &record.record_type,
            crate::config::RelationRole::Supersession,
        );
        let Some(id) = record_id(record) else {
            return Outcome::OutOfScope;
        };
        let normalized_id = crate::values::RecordId::new(&id);
        // Unreadable, not absent: the header did not parse, so this slot
        // has no value to classify (BUG-125's shape, `field_quality`'s
        // same guard).
        if record.header.is_unreadable() {
            return Outcome::Unreadable;
        }
        let Some(value) = record.header.get(field) else {
            return Outcome::Absent;
        };
        // `—` is this corpus's written "nothing supersedes this", so the slot
        // was answered. Skipping before counting left the reciprocating half of
        // a correct pair uncounted.
        if crate::values::is_no_value_sentinel(value) {
            return Outcome::Examined;
        }
        let claimant = Claimant {
            record,
            field,
            id,
            normalized_id,
        };
        for reference in extract_references(value) {
            self.check_reference(&claimant, &reference, findings);
        }
        Outcome::Examined
    }

    fn check_reference(
        &self,
        claimant: &Claimant<'_>,
        reference: &str,
        findings: &mut Vec<Finding>,
    ) {
        let Claimant {
            record,
            field,
            id,
            normalized_id,
        } = claimant;
        let Some(target) = self.index.get(&crate::values::RecordId::new(reference)) else {
            let (variant, message) = unresolved_reference(
                field,
                reference,
                self.untracked,
                ("supersession-unresolved", "supersession-untracked"),
                "",
            );
            findings.push(Finding {
                rule: RULE_RELATION_SUPERSESSION_RECIPROCITY.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new(variant).field(*field),
                message,
            });
            return;
        };
        // The target's header not parsing is its own defect, already
        // disclosed by that record's own `Unreadable` population entry
        // (and `header.required-fields`) -- `.get()` returns `None` for
        // an unreadable header exactly as it would for a genuinely
        // absent field, so reading it unguarded would blame the citer
        // for a failure that is entirely the target's.
        if target.header.is_unreadable() {
            return;
        }
        let target_field = relation_field_name(
            self.config,
            &target.record_type,
            crate::config::RelationRole::Supersession,
        );
        let target_value = target.header.get(target_field).unwrap_or("");
        let target_names_back = extract_references(target_value)
            .iter()
            .any(|r| crate::values::RecordId::new(r) == *normalized_id);
        if !target_names_back {
            findings.push(Finding {
                rule: RULE_RELATION_SUPERSESSION_RECIPROCITY.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new("supersession-unreciprocated")
                    .file(target.path.clone())
                    .field(target_field)
                    .text(id.clone()),
                message: format!(
                    "{field} claims a relation with {reference}, but {reference}'s {target_field} does not reciprocally name {id}"
                ),
            });
        }
    }
}

/// Rule (`RFC-45`/`ADR-63`): a resolved pointer/narrative reference's target
/// may not declare `Status` at all (`ADR-60`'s gate then silently excludes
/// it from every rule that reads it -- `pointer.target-status`,
/// `narrative-field.stale`). The one place this is diagnosed, rather than
/// each consuming rule inventing its own check, the same shape
/// `header.field-case-mismatch` already is for a declared-field case
/// mismatch (`RFC-40`/`ADR-58`).
pub fn relation_target_status_undeclared(
    records: &[Record],
    config: &Config,
    config_path: &Path,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_TARGET_STATUS_UNDECLARED;
    let mut findings = Vec::new();

    let pointer_fields_by_type = pointer_fields_by_type(config);
    let pointer_fields_by_type = &pointer_fields_by_type;
    let narrative_fields_by_type = narrative_fields_by_type(config);
    let narrative_fields_by_type = &narrative_fields_by_type;
    let declared_by_type = declared_fields_by_type(config);
    let declared_by_type = &declared_by_type;

    let slots =
        pointer_and_narrative_slots(records, pointer_fields_by_type, narrative_fields_by_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |(record, _)| *record,
        |(record, field_name)| {
            let result = for_each_resolved_status_target(
                record,
                field_name.as_str(),
                config,
                index,
                |reference, target, target_status_field| {
                    let declared = declared_by_type
                        .get(&target.record_type)
                        .is_some_and(|fields| fields.contains(target_status_field));
                    if !declared {
                        findings.push(Finding {
                            rule: RULE_ID.to_string(),
                            severity: FindingSeverity::Warning,
                            file: record.path.clone(),
                            line: None,
                            waived: None,
                            declared_severity: None,
                            suggested_action: None,
                            action_source: match config.record_types.get(&target.record_type) {
                                Some(target_config) => super::declaration::known_fields_source(
                                    "target-status-undeclared",
                                    "target-status-undeclared-declare-set",
                                    target.record_type.as_str(),
                                    target_config,
                                    records,
                                    target_status_field,
                                )
                                .file(config_path),
                                None => ActionSource::new("target-status-undeclared-declare-set")
                                    .file(config_path)
                                    .key_path(["record_types", target.record_type.as_str(), "known_fields"])
                                    .list([target_status_field.to_string()]),
                            },
                            message: format!(
                                "{field_name}: {reference} resolves, but its type '{}' does not declare {target_status_field} -- its status can never be checked",
                                target.record_type
                            ),
                        });
                    }
                },
            );
            match result {
                Ok(()) => Outcome::Examined,
                Err(outcome) => outcome,
            }
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

/// One side of one declared relation pair on one record whose type declares
/// that side's field -- the population unit of both pair rules (`RFC-47`).
#[derive(Clone, Copy)]
pub(super) struct PairSlot<'a> {
    pub(super) record: &'a Record,
    pub(super) pair: &'a crate::config::RelationPair,
    /// Is this the pair's `forward` side (else its `inverse`).
    pub(super) forward: bool,
}

impl PairSlot<'_> {
    pub(super) fn own_field(&self) -> &str {
        if self.forward {
            &self.pair.forward
        } else {
            &self.pair.inverse
        }
    }

    /// The field on the claim's target that must echo this side.
    fn echo_field(&self) -> &str {
        if self.forward {
            &self.pair.inverse
        } else {
            &self.pair.forward
        }
    }
}

/// Whether a record type carries a pair: every type does unless the pair
/// declares `types` (`MILE-155`).
pub(super) fn pair_carried_by(pair: &crate::config::RelationPair, record_type: &str) -> bool {
    pair.types
        .as_ref()
        .is_none_or(|types| types.iter().any(|t| t == record_type))
}

/// Every `(record, pair, side)` where the record's type declares the side's
/// field, for the pairs `keep` selects. A type that does not declare a pair's
/// field is not that pair's business, the same gate `ADR-60` gives `Status`.
pub(super) fn pair_slots<'a>(
    records: &'a [Record],
    config: &'a Config,
    keep: impl Fn(&crate::config::RelationPair) -> bool,
) -> Vec<PairSlot<'a>> {
    let declared_by_type = declared_fields_by_type(config);
    let mut slots = Vec::new();
    for pair in config.relation_pairs.iter().filter(|p| keep(p)) {
        for record in records {
            if !pair_carried_by(pair, &record.record_type) {
                continue;
            }
            let Some(declared) = declared_by_type.get(&record.record_type) else {
                continue;
            };
            for forward in [true, false] {
                let slot = PairSlot {
                    record,
                    pair,
                    forward,
                };
                if declared.contains(&crate::values::FieldName::new(slot.own_field())) {
                    slots.push(slot);
                }
            }
        }
    }
    slots
}

/// The references a pair slot claims, or the `Outcome` that ends the slot:
/// no readable id, an unreadable header, an unwritten field (`Absent`), or
/// the written "nothing here" sentinel (answered, so `Examined`).
pub(super) fn pair_slot_claims(slot: &PairSlot) -> Result<Vec<String>, Outcome> {
    if record_id(slot.record).is_none() {
        return Err(Outcome::OutOfScope);
    }
    if slot.record.header.is_unreadable() {
        return Err(Outcome::Unreadable);
    }
    let Some(value) = slot.record.header.get(slot.own_field()) else {
        return Err(Outcome::Absent);
    };
    if crate::values::is_no_value_sentinel(value) {
        return Err(Outcome::Examined);
    }
    Ok(extract_references(value))
}

fn status_field<'a>(record: &Record, config: &'a Config) -> &'a str {
    relation_field_name(
        config,
        &record.record_type,
        crate::config::RelationRole::Status,
    )
}

fn claimant_status<'a>(record: &'a Record, config: &Config) -> Option<&'a str> {
    record.header.get(status_field(record, config))
}

/// The severity a claim from `record` is held to when the pair declares a
/// provisional split and the claimant's status is one of its statuses.
/// `None` means the rule's own level applies (`MILE-154`).
fn provisional_severity(
    pair: &crate::config::RelationPair,
    record: &Record,
    config: &Config,
) -> Option<FindingSeverity> {
    let prov = pair.provisional.as_ref()?;
    let status = claimant_status(record, config)?;
    prov.statuses
        .iter()
        .any(|s| s == status)
        .then_some(match prov.level {
            crate::config::ProvisionalLevel::Warn => FindingSeverity::Warning,
            crate::config::ProvisionalLevel::Error => FindingSeverity::Error,
        })
}

/// Whether `target` names `claimant` in the pair's declared
/// `provisional.acknowledged_by` field (`MILE-163`). False when none is declared.
fn acknowledges(
    pair: &crate::config::RelationPair,
    target: &Record,
    claimant: &crate::values::RecordId,
) -> bool {
    let Some(field) = pair
        .provisional
        .as_ref()
        .and_then(|p| p.acknowledged_by.as_deref())
    else {
        return false;
    };
    extract_references(target.header.get(field).unwrap_or(""))
        .iter()
        .any(|r| crate::values::RecordId::new(r) == *claimant)
}

/// Rule (`RFC-47`, `MILE-122`): reciprocity over every declared relation pair.
/// A claim `A.forward = B` needs `B.inverse` to name `A`, and `A.inverse = B`
/// needs `B.forward` to name `A`; direction is the field name, so two records
/// each claiming the forward side do not satisfy each other. A pair declaring
/// `cross_type: false` has its cross-type claims excluded here (counted out of
/// scope when every claim of a slot is) and reported by
/// `relation.cross-type-disallowed`.
pub fn relation_pair_reciprocity(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
    untracked: &UntrackedRecords,
) -> (RuleExecution, Vec<Finding>) {
    let mut findings = Vec::new();

    let slots = pair_slots(records, config, |p| {
        p.reciprocity == crate::config::PairReciprocity::Required
    });
    let check = PairReciprocityCheck {
        config,
        index,
        untracked,
    };

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| check.judge(slot, &mut findings),
    );

    (
        RuleExecution {
            rule: RULE_RELATION_PAIR_RECIPROCITY.to_string(),
            population: Some(population),
            status: RuleStatus::Ran,
            examined_records,
        },
        findings,
    )
}

/// What `relation.pair-reciprocity` resolves a claim against.
struct PairReciprocityCheck<'a> {
    config: &'a Config,
    index: &'a RecordIndex<'a>,
    untracked: &'a UntrackedRecords,
}

/// How one claim of a slot counts toward the slot's outcome.
enum ClaimEcho {
    Checked,
    /// Excluded from the rule (a cross-type claim of a pair that forbids it).
    Excluded,
    /// Excluded because the target's type is outside a scoped pair.
    ScopedOut,
}

impl PairReciprocityCheck<'_> {
    fn judge(&self, slot: &PairSlot<'_>, findings: &mut Vec<Finding>) -> Outcome {
        let references = match pair_slot_claims(slot) {
            Ok(references) => references,
            Err(outcome) => return outcome,
        };
        let mut excluded = 0;
        let mut scoped_out = 0;
        for reference in &references {
            match self.check_claim(slot, reference, findings) {
                ClaimEcho::Checked => {}
                ClaimEcho::Excluded => excluded += 1,
                ClaimEcho::ScopedOut => {
                    excluded += 1;
                    scoped_out += 1;
                }
            }
        }
        if !references.is_empty() && excluded == references.len() {
            if scoped_out == excluded {
                Outcome::Absent
            } else {
                Outcome::OutOfScope
            }
        } else {
            Outcome::Examined
        }
    }

    fn check_claim(
        &self,
        slot: &PairSlot<'_>,
        reference: &str,
        findings: &mut Vec<Finding>,
    ) -> ClaimEcho {
        let record = slot.record;
        let field = slot.own_field();
        let echo = slot.echo_field();
        let id = record_id(record).unwrap_or_default();
        let normalized_id = crate::values::RecordId::new(&id);
        let Some(target) = self.index.get(&crate::values::RecordId::new(reference)) else {
            let (variant, message) = unresolved_reference(
                field,
                reference,
                self.untracked,
                ("pair-unresolved", "pair-untracked"),
                "",
            );
            findings.push(Finding {
                rule: RULE_RELATION_PAIR_RECIPROCITY.to_string(),
                severity: FindingSeverity::Error,
                file: record.path.clone(),
                line: None,
                waived: None,
                declared_severity: None,
                suggested_action: None,
                action_source: ActionSource::new(variant).field(field),
                message,
            });
            return ClaimEcho::Checked;
        };
        // Its own `Unreadable` entry discloses this; reading it here
        // would blame the citer for the target's failure.
        if target.header.is_unreadable() {
            return ClaimEcho::Checked;
        }
        if !slot.pair.cross_type && target.record_type != record.record_type {
            return ClaimEcho::Excluded;
        }
        // A target of a type outside a scoped pair cannot hold the
        // inverse field, so the claim cannot be echoed: not an unechoed
        // claim (`MILE-155`). Eligible and unexamined, not out of scope:
        // the claim is legitimate and no edit to the configuration's
        // reach is wrong, so `config.scope-matches-nothing` must not
        // warn that the configuration cannot reach it.
        if !pair_carried_by(slot.pair, &target.record_type) {
            return ClaimEcho::ScopedOut;
        }
        let names_back = extract_references(target.header.get(echo).unwrap_or(""))
            .iter()
            .any(|r| crate::values::RecordId::new(r) == normalized_id);
        if !names_back {
            // A claim from a record not yet at an accepted status is
            // provisional (`MILE-154`): the pair's own level applies
            // instead of the rule's. Anything else keeps the rule's.
            let declared_severity = provisional_severity(slot.pair, record, self.config);
            // A provisional claim the peer acknowledges in its declared
            // acknowledgement field is settled by the peer's own word
            // (`MILE-163`); a claim from any other status is not.
            if declared_severity.is_some() && acknowledges(slot.pair, target, &normalized_id) {
                return ClaimEcho::Checked;
            }
            let note = if declared_severity.is_some() {
                format!(
                    " (provisional: {} is '{}')",
                    status_field(record, self.config),
                    claimant_status(record, self.config).unwrap_or_default()
                )
            } else {
                String::new()
            };
            findings.push(Finding {
                rule: RULE_RELATION_PAIR_RECIPROCITY.to_string(),
                severity: declared_severity.unwrap_or(FindingSeverity::Error),
                file: record.path.clone(),
                line: None,
                waived: None,
                declared_severity,
                suggested_action: None,
                action_source: ActionSource::new("pair-unreciprocated")
                    .file(target.path.clone())
                    .field(echo)
                    .text(id.clone()),
                message: format!(
                    "{field} claims a relation with {reference} (pair '{}'), but {reference}'s {echo} does not reciprocally name {id}{note}",
                    slot.pair.name
                ),
            });
        }
        ClaimEcho::Checked
    }
}

/// Rule (`RFC-47`, `MILE-122`): a claim crossing record types in a pair that
/// declares `cross_type: false`. The reciprocity rule excludes such a claim as
/// out of scope; this is where it is reported, so it is never a silent skip.
pub fn relation_cross_type_disallowed(
    records: &[Record],
    config: &Config,
    index: &RecordIndex,
) -> (RuleExecution, Vec<Finding>) {
    const RULE_ID: &str = RULE_RELATION_CROSS_TYPE_DISALLOWED;
    let mut findings = Vec::new();

    let slots = pair_slots(records, config, |p| !p.cross_type);

    let (population, examined_records) = scoped_census_records(
        config,
        PopulationUnit::Field,
        slots,
        |slot| slot.record,
        |slot| {
            let references = match pair_slot_claims(slot) {
                Ok(references) => references,
                Err(outcome) => return outcome,
            };
            for reference in references {
                // An unresolved reference is `relation.pair-reciprocity`'s.
                let Some(target) = index.get(&crate::values::RecordId::new(&reference)) else {
                    continue;
                };
                if target.record_type != slot.record.record_type {
                    findings.push(Finding {
                        rule: RULE_ID.to_string(),
                        severity: FindingSeverity::Error,
                        file: slot.record.path.clone(),
                        line: None,
                        waived: None,
                        declared_severity: None,
                        suggested_action: None,
                        action_source: ActionSource::new("cross-type-claim").field(slot.own_field()).text(reference.clone()),
                        message: format!(
                            "{}: {reference} is a '{}' record, but pair '{}' declares cross_type: false and this record is '{}'",
                            slot.own_field(),
                            target.record_type,
                            slot.pair.name,
                            slot.record.record_type
                        ),
                    });
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

    #[test]
    fn a_supersession_to_an_untracked_file_on_disk_is_its_own_variant_observed_failing() {
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-1, ADR-7\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Supersedes / Superseded-by"], None, None, None),
        )]);
        let records = [new];
        let index = build_normalized_index(&records);
        let untracked: UntrackedRecords = [(
            crate::values::RecordId::new("ADR-1"),
            PathBuf::from("docs/adr/ADR-1-old.md"),
        )]
        .into();
        let (_, findings) = supersession_reciprocity(&records, &config, &index, &untracked);
        let variant = |id: &str| {
            findings
                .iter()
                .find(|f| f.message.contains(id))
                .map(|f| f.action_source.variant)
        };
        assert_eq!(
            variant("ADR-1"),
            Some("supersession-untracked"),
            "{findings:?}"
        );
        assert_eq!(
            variant("ADR-7"),
            Some("supersession-unresolved"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_pair_claim_on_an_untracked_file_on_disk_is_its_own_variant_observed_failing() {
        let config = amend_config();
        let rfc = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nAmends: RFC-2, RFC-3\n---\n",
        );
        let records = [rfc];
        let index = build_normalized_index(&records);
        let untracked: UntrackedRecords = [(
            crate::values::RecordId::new("RFC-2"),
            PathBuf::from("docs/rfc/RFC-2-b.md"),
        )]
        .into();
        let (_, findings) = relation_pair_reciprocity(&records, &config, &index, &untracked);
        let variant = |id: &str| {
            findings
                .iter()
                .find(|f| f.message.contains(id))
                .map(|f| f.action_source.variant)
        };
        assert_eq!(variant("RFC-2"), Some("pair-untracked"), "{findings:?}");
        assert_eq!(variant("RFC-3"), Some("pair-unresolved"), "{findings:?}");
    }

    #[test]
    fn a_supersession_slot_declared_only_as_required_is_still_declared() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nSupersedes / Superseded-by: —\n---\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-0001\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&["Supersedes / Superseded-by"], None, None, None),
        )]);

        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(findings.len(), 1, "the claim is one-directional");
    }

    #[test]
    fn a_one_directional_supersession_claim_is_a_reciprocity_violation() {
        // 0002 claims to supersede 0001, but 0001 doesn't reciprocally name
        // 0002 -- a reader of 0001 would never know it was superseded.
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nSupersedes / Superseded-by: —\n---\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-0001\n---\n",
        );

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(
            population.examined(),
            2,
            "the reciprocating half answered its slot with the em dash"
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn a_reciprocated_supersession_produces_no_finding() {
        let old = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-0002\n---\n",
        );
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-0001\n---\n",
        );

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!((population.eligible(), population.examined()), (2, 2));
        assert!(findings.is_empty(), "unexpected findings: {findings:?}");
    }

    /// `BUG-125`'s shape, a second instance found alongside
    /// `header_pointer_field_clean`'s by the same review: an unparsed header
    /// must disclose `unreadable`, not fold silently into `absent`.
    #[test]
    fn supersession_reciprocity_discloses_unreadable_on_its_own_observed_failing() {
        let r = record("docs/adr/ADR-1-x.md", "adr", "this is not a header at all");
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [r];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.examined(), 0, "{population:?}");
        assert_eq!(population.unreadable(), 1, "{population:?}");
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// The citing record's own header being unreadable is already covered
    /// (`supersession_reciprocity_discloses_unreadable_on_its_own_observed_failing`).
    /// This is the *target*'s header being unreadable: `target.header.get()`
    /// returns `None` for that just as readily as for a genuinely absent
    /// field, so an un-gated `.unwrap_or("")` blamed the citing record for
    /// the target's own parse failure instead of skipping a judgment neither
    /// side's data supports.
    #[test]
    fn a_target_with_an_unreadable_header_is_not_blamed_for_failing_to_reciprocate_observed_failing(
    ) {
        let citer = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nSupersedes / Superseded-by: ADR-0002\n---\n",
        );
        let target = record("docs/adr/ADR-2-y.md", "adr", "this is not a header at all");

        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [citer, target];
        let index = build_normalized_index(&records);
        let (_exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        assert!(
            findings.is_empty(),
            "the target's own parse failure is not the citer's fault: {findings:?}"
        );
    }

    #[test]
    fn a_type_declaring_a_custom_supersession_field_name_is_read_by_that_name_observed_failing() {
        let old = record("docs/adr/ADR-1-x.md", "adr", "---\nReplaces: —\n---\n");
        let new = record(
            "docs/adr/ADR-2-y.md",
            "adr",
            "---\nReplaces: ADR-0001\n---\n",
        );

        let config = config_with_types(vec![(
            "adr",
            crate::config::RecordTypeConfig {
                exclude_files: Vec::new(),
                relation_fields: Some(crate::config::RelationFields {
                    supersession: Some("Replaces".to_string()),
                    ..Default::default()
                }),
                ..type_config_pointer(&[], Some(&["Replaces"]), None, None)
            },
        )]);
        let records = [old, new];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(population.eligible(), 2, "both records declare the slot");
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("ADR-0001"));
    }

    #[test]
    fn an_em_dash_supersession_value_answers_its_slot() {
        let r = record(
            "docs/adr/ADR-1-x.md",
            "adr",
            "---\nSupersedes / Superseded-by: —\n---\n",
        );
        let config = config_with_types(vec![(
            "adr",
            type_config_pointer(&[], Some(&["Supersedes / Superseded-by"]), None, None),
        )]);
        let records = [r];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            supersession_reciprocity(&records, &config, &index, &Default::default());
        let population = exec
            .population
            .expect("the rule must state what it was handed");
        assert_eq!(
            (population.eligible(), population.examined()),
            (1, 1),
            "the em dash is a written answer, not an unfilled slot"
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn relation_target_status_undeclared_flags_a_resolving_reference_to_an_undeclaring_type_observed_failing(
    ) {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "---\nDate: 2026-01-01\n---\n");
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-1\n---\n",
        );
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            // "rfc" declares no Status at all -- ADR-60's gate would silently
            // exclude any rule reading it, which is exactly what this rule
            // exists to surface.
            ("rfc", type_config_pointer(&["Date"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) = relation_target_status_undeclared(
            &records,
            &config,
            Path::new(".urzua/config.yaml"),
            &index,
        );
        assert_eq!(examined(&exec), 1);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-1"), "{findings:?}");
        assert!(findings[0].message.contains("Status"), "{findings:?}");
    }

    #[test]
    fn relation_target_status_undeclared_is_silent_when_the_target_declares_status() {
        let target = record("docs/rfc/RFC-1-x.md", "rfc", "---\nStatus: Draft\n---\n");
        let source = record(
            "docs/specs/SPEC-1-x.md",
            "spec",
            "---\nImplements: RFC-1\n---\n",
        );
        let config = config_with_types(vec![
            (
                "spec",
                type_config_pointer(&[], None, Some(&["Implements"]), Some(&[])),
            ),
            ("rfc", type_config_pointer(&["Status"], None, None, None)),
        ]);
        let records = [target, source];
        let index = build_normalized_index(&records);

        let (exec, findings) = relation_target_status_undeclared(
            &records,
            &config,
            Path::new(".urzua/config.yaml"),
            &index,
        );
        assert_eq!(examined(&exec), 1);
        assert!(findings.is_empty(), "{findings:?}");
    }

    // ---- relation pairs (RFC-47, MILE-122) ----

    fn pair(name: &str, fwd: &str, inv: &str, cross_type: bool) -> crate::config::RelationPair {
        crate::config::RelationPair {
            name: name.to_string(),
            forward: fwd.to_string(),
            inverse: inv.to_string(),
            reciprocity: crate::config::PairReciprocity::Required,
            cross_type,
            provisional: None,
            types: None,
            acyclic: false,
        }
    }

    fn pair_config(types: Vec<(&str, &[&str])>, pairs: Vec<crate::config::RelationPair>) -> Config {
        let mut config = config_with_types(
            types
                .into_iter()
                .map(|(t, known)| (t, type_config_pointer(&[], Some(known), None, None)))
                .collect(),
        );
        config.relation_pairs = pairs;
        config
    }

    fn amend_config() -> Config {
        pair_config(
            vec![("rfc", &["Amends", "Amended-by"])],
            vec![pair("amendment", "Amends", "Amended-by", false)],
        )
    }

    fn run_pairs(config: &Config, records: &[Record]) -> (RuleExecution, Vec<Finding>) {
        let index = build_normalized_index(records);
        relation_pair_reciprocity(records, config, &index, &Default::default())
    }

    #[test]
    fn a_forward_claim_the_inverse_side_does_not_echo_is_a_finding() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---
Amended-by: —
---
",
        );
        let amender = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---
Amends: RFC-1
---
",
        );
        let records = [base, amender];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, RULE_RELATION_PAIR_RECIPROCITY);
        assert!(findings[0].message.contains("RFC-1"), "{findings:?}");
        assert!(findings[0].message.contains("Amended-by"), "{findings:?}");
        assert_eq!(examined(&exec), 2, "both slots answered: {exec:?}");
    }

    #[test]
    fn a_reciprocated_pair_has_no_finding_and_both_slots_are_examined() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---
Amended-by: RFC-2
---
",
        );
        let amender = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---
Amends: RFC-1
---
",
        );
        let records = [base, amender];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert!(findings.is_empty(), "{findings:?}");
        // 2 records x 2 sides declared; the two unwritten sides are absent.
        let pop = exec.population.unwrap();
        assert_eq!((pop.eligible(), pop.examined()), (4, 2), "{pop:?}");
    }

    #[test]
    fn an_inverse_claim_the_forward_side_does_not_echo_is_a_finding() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---
Amended-by: RFC-2
---
",
        );
        let other = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---
Amends: —
---
",
        );
        let records = [base, other];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("Amends"), "{findings:?}");
    }

    /// The direction-blind rule passes this: each side names the other
    /// somewhere. With direction declared, `RFC-1`'s `Amends` claim needs
    /// `RFC-2`'s `Amended-by`, which is absent.
    #[test]
    fn two_records_each_claiming_the_forward_direction_are_not_reciprocal() {
        let a = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---
Amends: RFC-2
---
",
        );
        let b = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---
Amends: RFC-1
---
",
        );
        let records = [a, b];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn a_pair_may_span_two_record_types() {
        let config = pair_config(
            vec![("adr", &["Resolves"]), ("bug", &["Resolved-by"])],
            vec![pair("resolution", "Resolves", "Resolved-by", true)],
        );
        let adr = record(
            "docs/adr/ADR-1-a.md",
            "adr",
            "---
Resolves: BUG-7
---
",
        );
        let bug = record(
            "docs/bug/BUG-7-b.md",
            "bug",
            "---
Resolved-by: ADR-1
---
",
        );
        let records = [adr.clone(), bug];
        let (_, findings) = run_pairs(&config, &records);
        assert!(findings.is_empty(), "{findings:?}");

        let bug = record(
            "docs/bug/BUG-7-b.md",
            "bug",
            "---
Resolved-by: —
---
",
        );
        let records = [adr, bug];
        let (_, findings) = run_pairs(&config, &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_target_type_that_declares_no_echo_field_cannot_echo() {
        let config = pair_config(
            vec![("adr", &["Resolves"]), ("bug", &["Title"])],
            vec![pair("resolution", "Resolves", "Resolved-by", true)],
        );
        let adr = record(
            "docs/adr/ADR-1-a.md",
            "adr",
            "---
Resolves: BUG-7
---
",
        );
        let bug = record("docs/bug/BUG-7-b.md", "bug", "---\nTitle: x\n---\n");
        let records = [adr, bug];
        let (_, findings) = run_pairs(&config, &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_cross_type_claim_in_a_same_type_pair_is_left_to_its_own_rule() {
        let config = pair_config(
            vec![
                ("rfc", &["Amends", "Amended-by"]),
                ("adr", &["Amends", "Amended-by"]),
            ],
            vec![pair("amendment", "Amends", "Amended-by", false)],
        );
        let rfc = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: ADR-2\n---\n");
        let adr = record("docs/adr/ADR-2-b.md", "adr", "---\nAmended-by: —\n---\n");
        let records = [rfc, adr];
        let index = build_normalized_index(&records);
        let (exec, findings) =
            relation_pair_reciprocity(&records, &config, &index, &Default::default());
        assert!(findings.is_empty(), "excluded, not judged: {findings:?}");
        assert_eq!(exec.population.unwrap().out_of_scope(), 1);
        let (cross_exec, cross) = relation_cross_type_disallowed(&records, &config, &index);
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].rule, RULE_RELATION_CROSS_TYPE_DISALLOWED);
        assert!(examined(&cross_exec) >= 1);
    }

    #[test]
    fn a_pair_declaring_no_reciprocity_is_not_examined() {
        let mut p = pair("amendment", "Amends", "Amended-by", true);
        p.reciprocity = crate::config::PairReciprocity::None;
        let config = pair_config(vec![("rfc", &["Amends", "Amended-by"])], vec![p]);
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: RFC-2\n---\n");
        let b = record("docs/rfc/RFC-2-b.md", "rfc", "---\nAmended-by: —\n---\n");
        let records = [a, b];
        let (exec, findings) = run_pairs(&config, &records);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(exec.population.unwrap().eligible(), 0);
    }

    #[test]
    fn a_pair_claim_naming_no_record_is_a_finding() {
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: RFC-99\n---\n");
        let records = [a];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].message.contains("does not resolve"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_pair_discloses_an_unreadable_header() {
        let r = record("docs/rfc/RFC-1-a.md", "rfc", "this is not a header at all");
        let records = [r];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.unwrap();
        assert_eq!((pop.examined(), pop.unreadable()), (0, 2), "{pop:?}");
    }

    #[test]
    fn an_unreadable_target_is_not_blamed_on_the_citer() {
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: RFC-2\n---\n");
        let b = record("docs/rfc/RFC-2-b.md", "rfc", "this is not a header at all");
        let records = [a, b];
        let (_, findings) = run_pairs(&amend_config(), &records);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_claim_naming_several_targets_is_judged_per_target() {
        let one = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nAmended-by: RFC-3\n---\n",
        );
        let two = record("docs/rfc/RFC-2-b.md", "rfc", "---\nAmended-by: —\n---\n");
        let three = record(
            "docs/rfc/RFC-3-c.md",
            "rfc",
            "---\nAmends: RFC-1, RFC-2\n---\n",
        );
        let records = [one, two, three];
        let (exec, findings) = run_pairs(&amend_config(), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("RFC-2's"), "{findings:?}");
        assert_eq!(findings[0].file, records[2].path);
        assert_eq!(examined(&exec), 3);
    }

    fn provisional_config() -> Config {
        let mut p = pair("amendment", "Amends", "Amended-by", false);
        p.provisional = Some(crate::config::ProvisionalClaim {
            statuses: vec!["Draft".to_string()],
            level: crate::config::ProvisionalLevel::Warn,
            acknowledged_by: None,
        });
        pair_config(
            vec![(
                "rfc",
                &["Status", "Amends", "Amended-by", "Amended-by-pending"],
            )],
            vec![p],
        )
    }

    #[test]
    fn a_draft_claim_echoed_only_in_a_pending_field_is_provisional_observed_failing() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nStatus: Accepted\nAmended-by: —\nAmended-by-pending: RFC-2\n---\n",
        );
        let draft = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---\nStatus: Draft\nAmends: RFC-1\n---\n",
        );
        let (_, findings) = run_pairs(&provisional_config(), &[base, draft]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Warning);
        assert_eq!(
            findings[0].declared_severity,
            Some(FindingSeverity::Warning)
        );
        assert!(findings[0].message.contains("provisional"), "{findings:?}");
    }

    fn acknowledging_config() -> Config {
        let mut cfg = provisional_config();
        cfg.relation_pairs[0]
            .provisional
            .as_mut()
            .unwrap()
            .acknowledged_by = Some("Amended-by-pending".to_string());
        cfg
    }

    #[test]
    fn a_provisional_claim_the_peer_acknowledges_is_not_reported_observed_failing() {
        let acknowledging = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nStatus: Accepted\nAmended-by: —\nAmended-by-pending: RFC-2\n---\n",
        );
        let silent = record(
            "docs/rfc/RFC-3-c.md",
            "rfc",
            "---\nStatus: Accepted\nAmended-by: —\n---\n",
        );
        let draft = |n: &str, target: &str| {
            record(
                &format!("docs/rfc/RFC-{n}-b.md"),
                "rfc",
                &format!("---\nStatus: Draft\nAmends: {target}\n---\n"),
            )
        };
        let (_, findings) = run_pairs(
            &acknowledging_config(),
            &[acknowledging.clone(), draft("2", "RFC-1")],
        );
        assert!(findings.is_empty(), "{findings:?}");
        // A peer without the acknowledgement still reports it.
        let (_, findings) = run_pairs(&acknowledging_config(), &[silent, draft("4", "RFC-3")]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].message.contains("provisional"), "{findings:?}");
        // The acknowledgement names a different record: not this claimant's.
        let (_, findings) = run_pairs(
            &acknowledging_config(),
            &[acknowledging, draft("9", "RFC-1")],
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn an_accepted_claim_is_not_settled_by_an_acknowledgement_observed_failing() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nStatus: Accepted\nAmended-by: —\nAmended-by-pending: RFC-2\n---\n",
        );
        let accepted = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---\nStatus: Accepted\nAmends: RFC-1\n---\n",
        );
        let (_, findings) = run_pairs(&acknowledging_config(), &[base, accepted]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
    }

    #[test]
    fn an_accepted_claim_with_the_same_gap_keeps_the_full_severity_observed_failing() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nStatus: Accepted\nAmended-by: —\nAmended-by-pending: RFC-2\n---\n",
        );
        let accepted = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---\nStatus: Accepted\nAmends: RFC-1\n---\n",
        );
        let (_, findings) = run_pairs(&provisional_config(), &[base, accepted]);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].declared_severity, None);
        assert!(!findings[0].message.contains("provisional"), "{findings:?}");
    }

    #[test]
    fn a_draft_claim_the_peer_settles_is_not_reported() {
        let base = record(
            "docs/rfc/RFC-1-a.md",
            "rfc",
            "---\nStatus: Accepted\nAmended-by: RFC-2\n---\n",
        );
        let draft = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---\nStatus: Draft\nAmends: RFC-1\n---\n",
        );
        let (_, findings) = run_pairs(&provisional_config(), &[base, draft]);
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_pair_declaring_no_provisional_split_is_unchanged_by_status() {
        let base = record("docs/rfc/RFC-1-a.md", "rfc", "---\nStatus: Accepted\n---\n");
        let draft = record(
            "docs/rfc/RFC-2-b.md",
            "rfc",
            "---\nStatus: Draft\nAmends: RFC-1\n---\n",
        );
        let (_, findings) = run_pairs(&amend_config(), &[base, draft]);
        assert_eq!(findings[0].severity, FindingSeverity::Error);
        assert_eq!(findings[0].declared_severity, None);
    }

    fn scoped_config(types: Option<Vec<&str>>) -> Config {
        let mut p = pair("amendment", "Amends", "Amended-by", true);
        p.types = types.map(|t| t.into_iter().map(String::from).collect());
        pair_config(
            vec![("rfc", &["Amends", "Amended-by"]), ("adr", &["Title"])],
            vec![p],
        )
    }

    #[test]
    fn a_claim_to_a_type_outside_a_scoped_pair_is_eligible_and_unexamined_not_an_error_observed_failing(
    ) {
        let rfc = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: ADR-2\n---\n");
        let adr = record("docs/adr/ADR-2-b.md", "adr", "---\nTitle: x\n---\n");
        let records = [rfc, adr];
        let (exec, findings) = run_pairs(&scoped_config(Some(vec!["rfc"])), &records);
        assert!(findings.is_empty(), "{findings:?}");
        let pop = exec.population.unwrap();
        assert_eq!(pop.out_of_scope(), 0, "{pop:?}");
        assert_eq!((pop.eligible(), pop.examined()), (2, 0), "{pop:?}");
        // The rule that judges the run must not blame the configuration.
        let (_, scope) = crate::rules::config_scope_matches_nothing(
            &[exec],
            std::path::Path::new(".urzua/config.yaml"),
        );
        assert!(scope.is_empty(), "{scope:?}");
    }

    #[test]
    fn the_same_claim_in_a_global_pair_still_errors() {
        let rfc = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: ADR-2\n---\n");
        let adr = record("docs/adr/ADR-2-b.md", "adr", "---\nTitle: x\n---\n");
        let records = [rfc, adr];
        let (_, findings) = run_pairs(&scoped_config(None), &records);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_scoped_pair_still_judges_claims_between_its_own_types() {
        let a = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: RFC-2\n---\n");
        let b = record("docs/rfc/RFC-2-b.md", "rfc", "---\nAmended-by: —\n---\n");
        let (_, findings) = run_pairs(&scoped_config(Some(vec!["rfc"])), &[a, b]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    #[test]
    fn a_record_of_a_type_outside_a_scoped_pair_is_not_its_business() {
        let config = pair_config(
            vec![
                ("rfc", &["Amends", "Amended-by"]),
                ("adr", &["Amends", "Amended-by"]),
            ],
            vec![{
                let mut p = pair("amendment", "Amends", "Amended-by", true);
                p.types = Some(vec!["rfc".to_string()]);
                p
            }],
        );
        let adr = record("docs/adr/ADR-1-a.md", "adr", "---\nAmends: RFC-2\n---\n");
        let rfc = record("docs/rfc/RFC-2-b.md", "rfc", "---\nAmended-by: —\n---\n");
        let (exec, findings) = run_pairs(&config, &[adr, rfc]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(
            exec.population.unwrap().eligible(),
            2,
            "only the rfc's two sides"
        );
    }

    #[test]
    fn a_self_referencing_claim_is_judged_like_any_other() {
        let lone = record("docs/rfc/RFC-1-a.md", "rfc", "---\nAmends: RFC-1\n---\n");
        let (_, findings) = run_pairs(&amend_config(), &[lone]);
        assert_eq!(findings.len(), 1, "{findings:?}");
    }
}
