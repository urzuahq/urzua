// One id constant per rule, named by the rule's own code and by `RULE_TABLE`
// below, which is the only place a rule is registered.
pub const RULE_HEADER_REQUIRED_FIELDS: &str = "header.required-fields";
pub const RULE_HEADER_FIELD_SET_CONSISTENCY: &str = "header.field-set-consistency";
pub const RULE_TYPE_NO_DECLARED_SPEC: &str = "type.no-declared-spec";
pub const RULE_TYPE_DIR_MATCHES_NOTHING: &str = "type.dir-matches-nothing";
pub const RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR: &str = "type.record-outside-declared-dir";
pub const RULE_IDENTITY_COLLISION: &str = "identity.collision";
pub const RULE_IDENTITY_OPEN_PR_COLLISION: &str = "identity.open-pr-collision";
pub const RULE_CONFIG_POINTER_DECLARATION_MISSING: &str = "config.pointer-declaration-missing";
pub const RULE_CONFIG_POINTER_FIELD_NOT_KNOWN: &str = "config.pointer-field-not-known";
pub const RULE_CONFIG_POINTER_NARRATIVE_OVERLAP: &str = "config.pointer-narrative-overlap";
pub const RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS: &str =
    "config.header-none-has-no-required-fields";
pub const RULE_CONFIG_COMPANION_DECLARATION_MISSING: &str = "config.companion-declaration-missing";
pub const RULE_POINTER_RESOLUTION: &str = "pointer.resolution";
pub const RULE_POINTER_TARGET_STATUS: &str = "pointer.target-status";
pub const RULE_HEADER_POINTER_FIELD_CLEAN: &str = "header.pointer-field-clean";
pub const RULE_NARRATIVE_FIELD_STALE: &str = "narrative-field.stale";
pub const RULE_FIELD_QUALITY: &str = "field.quality";
pub const RULE_FIELD_PENDING: &str = "field.pending";
pub const RULE_CLAIM_STATUS_AGREEMENT: &str = "claim.status-agreement";
pub const RULE_FILENAME_TITLE_CONSISTENCY: &str = "filename.title-consistency";
pub const RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED: &str = "revision-log.change-class-required";
pub const RULE_REVISION_LOG_VERSION_BUMP_REQUIRED: &str = "revision-log.version-bump-required";
pub const RULE_EMBODIMENT_CONSISTENCY: &str = "embodiment.consistency";
pub const RULE_EMBODIMENT_LOCATOR_EXISTS: &str = "embodiment.locator-exists";
pub const RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE: &str =
    "embodiment.locator-promotion-candidate";
pub const RULE_RELATION_SUPERSESSION_RECIPROCITY: &str = "relation.supersession-reciprocity";
pub const RULE_RELATION_PAIR_RECIPROCITY: &str = "relation.pair-reciprocity";
pub const RULE_RELATION_CROSS_TYPE_DISALLOWED: &str = "relation.cross-type-disallowed";
pub const RULE_SEQUENCE_CYCLE: &str = "sequence.cycle";
pub const RULE_SEQUENCE_UNRESOLVED_BLOCKER: &str = "sequence.unresolved-blocker";
pub const RULE_SEQUENCE_STATUS_AGREEMENT: &str = "sequence.status-agreement";
pub const RULE_RELATION_TARGET_STATUS_UNDECLARED: &str = "relation.target-status-undeclared";
pub const RULE_CONFIG_SCHEMA_VERSION_OUTDATED: &str = "config.schema-version-outdated";
pub const RULE_CONFIG_SCOPE_MATCHES_NOTHING: &str = "config.scope-matches-nothing";
pub const RULE_FIELD_UNTRIMMED_VALUE: &str = "field.untrimmed-value";
pub const RULE_HEADER_FIELD_CASE_MISMATCH: &str = "header.field-case-mismatch";
pub const RULE_CONFIG_RELATION_FIELD_NOT_KNOWN: &str = "config.relation-field-not-known";
pub const RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING: &str =
    "config.known-fields-declaration-missing";
pub const RULE_FIELD_LEADING_RESERVED_INDICATOR: &str = "field.leading-reserved-indicator";
pub const RULE_HEADER_CASE_VARIANT_KEY: &str = "header.case-variant-key";
pub const RULE_CITATION_RESOLUTION: &str = "citation.resolution";
pub const RULE_CITATION_PATH_EXISTS: &str = "citation.path-exists";
pub const RULE_CITATION_ISSUE_EXISTS: &str = "citation.issue-exists";
pub const RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE: &str = "boundary.published-path-reference";
pub const RULE_SECTION_REQUIRED: &str = "section.required";
pub const RULE_SECTION_SHAPE: &str = "section.shape";
pub const RULE_DIFF_SOURCE_REQUIRES_RECORD: &str = "diff.source-requires-record";
pub const RULE_DIFF_SOURCE_AND_RECORDS_MIXED: &str = "diff.source-and-records-mixed";
pub const RULE_DIFF_DECIDER_REQUIRED: &str = "diff.decider-required";
pub const RULE_FILENAME_IDENTITY_PATTERN: &str = "filename.identity-pattern";
pub const RULE_STATUS_UNDECLARED_VALUE: &str = "status.undeclared-value";
pub const RULE_STATUS_REQUIRED_FIELD_MISSING: &str = "status.required-field-missing";
pub const RULE_STATUS_FORBIDDEN_FIELD_PRESENT: &str = "status.forbidden-field-present";
pub const RULE_STATUS_DELIVERY_AGREEMENT: &str = "status.delivery-agreement";
pub const RULE_STATUS_TRANSITION_ALLOWED: &str = "status.transition-allowed";
pub const RULE_STATUS_UNREACHABLE: &str = "status.unreachable-state";

pub const RULE_INDEX_FRESHNESS: &str = "index.freshness";
pub const RULE_EMBODIMENT_BACK_POINTER_RESOLUTION: &str = "embodiment.back-pointer-resolution";
pub const RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS: &str = "embodiment.requires-accepted-status";
pub const RULE_DATE_FIELD_VALID: &str = "date.field-valid";
pub const RULE_DATE_FIELD_ORDER: &str = "date.field-order";
pub const RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED: &str =
    "embodiment.back-pointer-decision-accepted";

pub const RULE_WAIVER_MATCHES_NOTHING: &str = "waiver.matches-nothing";

pub const RULE_OVERRIDE_UNREVIEWED: &str = "override.unreviewed";
pub const RULE_EMBODIMENT_LOCATOR_ON_BACK_POINTER_PATH: &str =
    "embodiment.locator-on-back-pointer-path";
