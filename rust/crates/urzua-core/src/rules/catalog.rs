/// Rule 1: header format consistency within a record type, per a
/// config-declared required-field list. A majority-rule inference would
/// silently ratify whatever drifted in, so the shape is declared, not voted.
/// Every rule this build ships, by id. One canonical name per rule: the
/// config is validated against this list, so a typo is a load-time error
/// naming the valid set rather than a check that silently never runs.
pub const RULE_HEADER_REQUIRED_FIELDS: &str = "header.required-fields";
pub const RULE_HEADER_FIELD_SET_CONSISTENCY: &str = "header.field-set-consistency";
pub const RULE_TYPE_NO_DECLARED_SPEC: &str = "type.no-declared-spec";
pub const RULE_TYPE_DIR_MATCHES_NOTHING: &str = "type.dir-matches-nothing";
pub const RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR: &str = "type.record-outside-declared-dir";
pub const RULE_IDENTITY_COLLISION: &str = "identity.collision";
pub const RULE_CONFIG_POINTER_DECLARATION_MISSING: &str = "config.pointer-declaration-missing";
pub const RULE_CONFIG_POINTER_FIELD_NOT_KNOWN: &str = "config.pointer-field-not-known";
pub const RULE_CONFIG_POINTER_NARRATIVE_OVERLAP: &str = "config.pointer-narrative-overlap";
pub const RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS: &str =
    "config.header-none-has-no-required-fields";
pub const RULE_POINTER_RESOLUTION: &str = "pointer.resolution";
pub const RULE_POINTER_TARGET_STATUS: &str = "pointer.target-status";
pub const RULE_HEADER_POINTER_FIELD_CLEAN: &str = "header.pointer-field-clean";
pub const RULE_NARRATIVE_FIELD_STALE: &str = "narrative-field.stale";
pub const RULE_FIELD_QUALITY: &str = "field.quality";
pub const RULE_FIELD_PENDING: &str = "field.pending";
pub const RULE_CLAIM_STATUS_AGREEMENT: &str = "claim.status-agreement";
pub const RULE_FILENAME_TITLE_CONSISTENCY: &str = "filename.title-consistency";
pub const RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED: &str = "revision-log.change-class-required";
pub const RULE_EMBODIMENT_CONSISTENCY: &str = "embodiment.consistency";
pub const RULE_EMBODIMENT_LOCATOR_EXISTS: &str = "embodiment.locator-exists";
pub const RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE: &str =
    "embodiment.locator-promotion-candidate";
pub const RULE_RELATION_SUPERSESSION_RECIPROCITY: &str = "relation.supersession-reciprocity";
pub const RULE_RELATION_PAIR_RECIPROCITY: &str = "relation.pair-reciprocity";
pub const RULE_RELATION_CROSS_TYPE_DISALLOWED: &str = "relation.cross-type-disallowed";
pub const RULE_RELATION_TARGET_STATUS_UNDECLARED: &str = "relation.target-status-undeclared";
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
pub const RULE_FILENAME_IDENTITY_PATTERN: &str = "filename.identity-pattern";
pub const RULE_STATUS_UNDECLARED_VALUE: &str = "status.undeclared-value";
pub const RULE_STATUS_REQUIRED_FIELD_MISSING: &str = "status.required-field-missing";
pub const RULE_STATUS_FORBIDDEN_FIELD_PRESENT: &str = "status.forbidden-field-present";
pub const RULE_STATUS_DELIVERY_AGREEMENT: &str = "status.delivery-agreement";

pub const RULE_INDEX_FRESHNESS: &str = "index.freshness";
pub const RULE_EMBODIMENT_BACK_POINTER_RESOLUTION: &str = "embodiment.back-pointer-resolution";
pub const RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS: &str = "embodiment.requires-accepted-status";
pub const RULE_DATE_FIELD_VALID: &str = "date.field-valid";
pub const RULE_DATE_FIELD_ORDER: &str = "date.field-order";
pub const RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED: &str =
    "embodiment.back-pointer-decision-accepted";

pub const RULE_WAIVER_MATCHES_NOTHING: &str = "waiver.matches-nothing";

pub const ALL_RULES: &[&str] = &[
    RULE_HEADER_REQUIRED_FIELDS,
    RULE_HEADER_FIELD_SET_CONSISTENCY,
    RULE_TYPE_NO_DECLARED_SPEC,
    RULE_TYPE_DIR_MATCHES_NOTHING,
    RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
    RULE_IDENTITY_COLLISION,
    RULE_CONFIG_POINTER_DECLARATION_MISSING,
    RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
    RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
    RULE_POINTER_RESOLUTION,
    RULE_POINTER_TARGET_STATUS,
    RULE_HEADER_POINTER_FIELD_CLEAN,
    RULE_NARRATIVE_FIELD_STALE,
    RULE_FIELD_QUALITY,
    RULE_FIELD_PENDING,
    RULE_CLAIM_STATUS_AGREEMENT,
    RULE_FILENAME_TITLE_CONSISTENCY,
    RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
    RULE_EMBODIMENT_CONSISTENCY,
    RULE_EMBODIMENT_LOCATOR_EXISTS,
    RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
    RULE_RELATION_SUPERSESSION_RECIPROCITY,
    RULE_CONFIG_SCOPE_MATCHES_NOTHING,
    RULE_FIELD_UNTRIMMED_VALUE,
    RULE_HEADER_FIELD_CASE_MISMATCH,
    RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
    RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
    RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
    RULE_RELATION_TARGET_STATUS_UNDECLARED,
    RULE_FIELD_LEADING_RESERVED_INDICATOR,
    RULE_HEADER_CASE_VARIANT_KEY,
    RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_CROSS_TYPE_DISALLOWED,
    RULE_CITATION_RESOLUTION,
    RULE_CITATION_PATH_EXISTS,
    RULE_CITATION_ISSUE_EXISTS,
    RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
    RULE_SECTION_REQUIRED,
    RULE_SECTION_SHAPE,
    RULE_DIFF_SOURCE_REQUIRES_RECORD,
    RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
    RULE_FILENAME_IDENTITY_PATTERN,
    RULE_STATUS_UNDECLARED_VALUE,
    RULE_STATUS_REQUIRED_FIELD_MISSING,
    RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
    RULE_STATUS_DELIVERY_AGREEMENT,
    RULE_INDEX_FRESHNESS,
    RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
    RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS,
    RULE_DATE_FIELD_VALID,
    RULE_DATE_FIELD_ORDER,
    RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
    RULE_WAIVER_MATCHES_NOTHING,
];

/// One rule's id and its adopter-facing description -- the single place this
/// text lives; `SPEC-2`'s own rule table is generated from it.
pub struct RuleMeta {
    pub id: &'static str,
    pub description: &'static str,
}

/// One entry per `ALL_RULES` member, same order.
pub const RULE_METADATA: &[RuleMeta] = &[
    RuleMeta {
        id: RULE_HEADER_REQUIRED_FIELDS,
        description: "Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here.",
    },
    RuleMeta {
        id: RULE_HEADER_FIELD_SET_CONSISTENCY,
        description: "Every header field belongs to its type's `required_fields` \u{222a} `known_fields`, when `known_fields` is declared (ADR-39). The optional `allow_extension_prefix` option (`MILE-141`), off unless declared `true`, exempts header keys beginning `x-` in any case; an unprefixed undeclared key is still a finding.",
    },
    RuleMeta {
        id: RULE_TYPE_NO_DECLARED_SPEC,
        description: "Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared.",
    },
    RuleMeta {
        id: RULE_TYPE_DIR_MATCHES_NOTHING,
        description: "A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. Also reports a type\u{2019}s `exclude_files` entry that names no tracked file directly in its `dir` (`MILE-150`), so a stale exclusion is visible.",
    },
    RuleMeta {
        id: RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
        description: "A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise.",
    },
    RuleMeta {
        id: RULE_IDENTITY_COLLISION,
        description: "Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`).",
    },
    RuleMeta {
        id: RULE_CONFIG_POINTER_DECLARATION_MISSING,
        description: "A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind.",
    },
    RuleMeta {
        id: RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it.",
    },
    RuleMeta {
        id: RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
        description: "A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44).",
    },
    RuleMeta {
        id: RULE_POINTER_RESOLUTION,
        description: "A type's config-declared `pointer_fields` \u{222a} `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted.",
    },
    RuleMeta {
        id: RULE_POINTER_TARGET_STATUS,
        description: "A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`.",
    },
    RuleMeta {
        id: RULE_HEADER_POINTER_FIELD_CLEAN,
        description: "A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose.",
    },
    RuleMeta {
        id: RULE_NARRATIVE_FIELD_STALE,
        description: "A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository.",
    },
    RuleMeta {
        id: RULE_FIELD_QUALITY,
        description: "Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`.",
    },
    RuleMeta {
        id: RULE_FIELD_PENDING,
        description: "A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `pending_until_required` option (`MILE-169`), off unless declared `true`, treats a pending value as legitimate until the record reaches a status the type's `statuses.require` ties the field to: a deferred slot stays eligible and is counted unexamined, and a field no `require` entry names is judged as before.",
    },
    RuleMeta {
        id: RULE_CLAIM_STATUS_AGREEMENT,
        description: "A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared.",
    },
    RuleMeta {
        id: RULE_FILENAME_TITLE_CONSISTENCY,
        description: "The filename's display number and the H1's display number agree (ADR-3).",
    },
    RuleMeta {
        id: RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
        description: "Every revision-log entry carries a `change_class` (ADR-14); missing or unclassified is blocking. The log is the section a type's `revision_log` identity (default the line `**Revision log**`) opens, found as that line and never by a line that mentions the text (`MILE-167`).",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_CONSISTENCY,
        description: "A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (`test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to `Drift detected` unconditionally (RFC-5 tier 1, ADR-18/ADR-32). A record with no `Realized-by` field is judged on the back-pointers that cite it (`MILE-149`); one stating `Verified` or `Implemented` that has neither a `Realized-by` field nor a citing back-pointer is reported as a claim with no evidence, computed `Not started` (`MILE-161`), and one stating any other value is not judged and stays unexamined. Requires full git history — a shallow checkout makes this rule silently unable to detect anything. Three opt-in refinements, all off by default (`MILE-129`): a locator written `path@sha256:<64 hex>` drifts when the file's content hash differs and not otherwise, in place of the git-history heuristic; the `drift_grace_days` option holds back reporting drift whose locator last changed fewer than that many days ago, and a record in that window agrees with either its tier or a stated `Drift detected` (the window never contradicts a record that already says so, and `urzua fix` leaves it alone); the optional `drift_audit_field` option names a declared date field (needs `drift_grace_days`): a record carrying a `YYYY-MM-DD` date there has its window counted from that last-audit date instead of from the cited file's last change, and a malformed date is reported and falls back to the file (`MILE-166`); the `claim_max_age_days` option warns when the `Realized-by` line was last touched more than that many days ago. The optional `sticky_states` option lists `Embodiment` values a human sets that no evidence can compute, such as `Deprecated` and `Inactive`: a record stating one is examined and never reported as drift or as an old claim, `urzua fix` leaves it alone, and a listed value that evidence does compute (`Verified`, `Implemented`, `Specified`, `Not started`, `Drift detected`) is a load error because it would hide real drift (`MILE-148`). Source back-pointers (see `embodiment.back-pointer-resolution`) are evidence too: an `Implements:` citation is code tier and a `Verifies:` citation in a declared test file is test tier, equal to a `test:` locator (`MILE-146`). A record with `Embodiment` but no `Realized-by` field is judged on back-pointer evidence alone when one cites it, and a record neither cited nor carrying `Realized-by` is not judged (`MILE-149`); a finding names the evidence it computed from.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_LOCATOR_EXISTS,
        description: "A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, or is empty. Tracked alone passes a staged deletion; on disk alone passes a gitignored file.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
        description: "The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18).",
    },
    RuleMeta {
        id: RULE_RELATION_SUPERSESSION_RECIPROCITY,
        description: "If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`.",
    },
    RuleMeta {
        id: RULE_CONFIG_SCOPE_MATCHES_NOTHING,
        description: "Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`).",
    },
    RuleMeta {
        id: RULE_FIELD_UNTRIMMED_VALUE,
        description: "A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed.",
    },
    RuleMeta {
        id: RULE_HEADER_FIELD_CASE_MISMATCH,
        description: "A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption.",
    },
    RuleMeta {
        id: RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
        description: "A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`).",
    },
    RuleMeta {
        id: RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`) must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`.",
    },
    RuleMeta {
        id: RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
        description: "A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s \"declared, not voted\" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`).",
    },
    RuleMeta {
        id: RULE_RELATION_TARGET_STATUS_UNDECLARED,
        description: "A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`).",
    },
    RuleMeta {
        id: RULE_FIELD_LEADING_RESERVED_INDICATOR,
        description: "A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand.",
    },
    RuleMeta {
        id: RULE_HEADER_CASE_VARIANT_KEY,
        description: "One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding.",
    },
    RuleMeta {
        id: RULE_RELATION_PAIR_RECIPROCITY,
        description: "For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. A pair that declares `types` is carried by those types only, and a claim to a type outside the list is excluded as out of scope (`MILE-155`). A pair that declares `provisional: {statuses, level}` reports an unechoed claim from a claimant at one of those statuses at the provisional level (default `warn`), and any other claimant at the rule's level (`MILE-154`). The optional `provisional.acknowledged_by` names a field on the peer that settles a provisional claim by naming the claimant (`MILE-163`); it is not read for any other claimant. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key.",
    },
    RuleMeta {
        id: RULE_RELATION_CROSS_TYPE_DISALLOWED,
        description: "A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently.",
    },
    RuleMeta {
        id: RULE_CITATION_RESOLUTION,
        description: "A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in. The optional `external_ids` option (`MILE-156`) lists ids that share a record prefix but are not records, each an exact id (`RFC-4122`) or an inclusive numeric range written `PREFIX-LOW..HIGH` under a declared prefix; a matching citation is counted as eligible and unexamined, never resolved or reported, and not as out of scope, so `config.scope-matches-nothing` does not read a declared id as a misconfiguration. An entry under no declared prefix is a config error. Its population counts the citations, so a run over none is distinguishable from a clean one, and external citations are visible as eligible and unexamined. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that cites an id as it was is not reported; it is `false` by default (`MILE-156`).",
    },
    RuleMeta {
        id: RULE_CITATION_PATH_EXISTS,
        description: "A repository path cited in backticks inside a `reference_sections` section (a `##` heading, matched exactly) exists in the working tree, tracked and on disk (`MILE-143`). A cited path is a code span with no whitespace, containing a `/` and ending in a file name with an extension or in `/`; a URL, a host-led path (`example.com/x`), an absolute or `~` path, a glob or placeholder (`*?<>{}$|`) and anything else is not a path and is not read. A trailing `:line` or `#anchor` is dropped, and `./` or `../` resolves from the record's directory. Fenced code blocks are not read. Needs `reference_sections`; a record without such a section is out of scope, and the population counts the paths cited. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that names a path as it was is not reported. It is `false` by default. Offline and deterministic. Cited issue and pull request numbers are `citation.issue-exists`'s.",
    },
    RuleMeta {
        id: RULE_CITATION_ISSUE_EXISTS,
        description: "An issue or pull request cited inside a `reference_sections` section (a `##` heading, matched exactly) is in the committed snapshot of the repository's numbers (`MILE-143`). `issue_refs` declares the `snapshot` file (relative to the repository root), the `repository` (`owner/name`) it describes and the citation `forms` read: `number` is `#123` (outside inline code, not `owner/repo#123`, not an anchor link `](#123)`), `url` is a `https://github.com/owner/name/issues/123` or `/pull/123` link to that repository; a form not declared is not read. A number is judged against the snapshot's issues and pull requests together, since GitHub shares one number space and redirects either URL. Offline and deterministic: `check` never reaches the network, and only `urzua refs refresh` writes the snapshot. A snapshot that is absent, unreadable, of another schema or of another repository is one finding on the snapshot path, and every citation is counted as eligible and unexamined: the rule never passes for want of its input. Each finding names the date the snapshot was generated, and `check` prints a notice with that date and the snapshot's size, so staleness is visible. The optional `skip_revision_log` option behaves as for `citation.path-exists`. The population's unit is the citation.",
    },
    RuleMeta {
        id: RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        description: "A record identifier (any identifier carrying a configured type prefix, resolvable or not) is cited from inside a `published_paths` entry (a path prefix, normalised at config load; globs are not supported and a glob is treated literally), which publishes a pointer no external reader can resolve (`RFC-11`/`ADR-9`, `MILE-125`). An identifier split across lines is not detected. A declared path matching no tracked file is itself a finding, never a clean pass; the rule's level is the boundary's severity.",
    },
    RuleMeta {
        id: RULE_SECTION_REQUIRED,
        description: "Each `##` section a type's declared `sections.required` names is present in every record of that type (`MILE-4`, `RFC-17`), under its `name` or one of its declared `aliases` (`MILE-165`; `check` discloses each alias in use as an info notice), or, for a `y-statement` declaring `preamble: true` and a `marker`, in the text before the first `##`. A type that declares none is out of scope; a record whose status is in the declaration's `exempt_statuses` (a record that made no decision, `MILE-7`) is counted eligible and unexamined, not asked for the sections, and not out of scope, so `config.scope-matches-nothing` does not read a declared exemption as a misconfiguration (`BUG-187`). Presence only: it never judges what a section says.",
    },
    RuleMeta {
        id: RULE_SECTION_SHAPE,
        description: "A required section that declares a `shape` has that shape (`MILE-5`, `MILE-121`): `y-statement` needs its TL;DR block to carry the five phrases `In the context of`, `facing`, `we decided`, `to achieve`, `accepting`, word-boundary matched, and reports a marker with no block as its own error; `table-min-rows` needs a table with at least `min_rows` data rows. A section found under an alias is checked too, and a `preamble: true` Y-statement is judged from the text before the first `##`. A missing section is `section.required`'s to report, not this rule's. Shape only, never meaning.",
    },
    RuleMeta {
        id: RULE_DIFF_SOURCE_REQUIRES_RECORD,
        description: "A `--base` diff changes a path under a declared `source_paths` entry (a prefix or a glob) but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`, `MILE-158`). A changed path matching `exclude_paths` is not source and stays in the population unexamined, so a diff touching only such files (tests beside the code) is not a trigger. One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed.",
    },
    RuleMeta {
        id: RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
        description: "A `--base` diff changes both a governed record and a path under a declared `application_paths` entry, a prefix or a glob (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean.",
    },
    RuleMeta {
        id: RULE_FILENAME_IDENTITY_PATTERN,
        description: "A file directly in a type's `dir` whose name does not match the type's declared `identity.pattern` (or, for a type declaring none, the `TYPE-NNNN-slug.md` convention) is reported rather than left out of every rule's population (`MILE-120`). When every file in the directory is off-pattern, such as a bare-number corpus (`0001-slug.md`), it is one finding per type naming the expected file name for each file, since renaming is the supported route (`RFC-46` was rejected). The census counts each file examined, and omits a file the type declares in `exclude_files` (`MILE-150`).",
    },
    RuleMeta {
        id: RULE_STATUS_UNDECLARED_VALUE,
        description: "A record's `Status` is not one of the values its type declares in `statuses.values` (`MILE-132`). A type declaring no `statuses` is not examined, and a record with no `Status` is left to `header.required-fields`.",
    },
    RuleMeta {
        id: RULE_STATUS_REQUIRED_FIELD_MISSING,
        description: "A field the type's `statuses.require` ties to the record's current `Status` is blank, unedited template text, or (when the type declares `statuses.pending`) one of that declared placeholder vocabulary's whole values such as `Unknown` or `none recorded`, or a value matching one of its `{pattern: ...}` entries for a placeholder family (`MILE-132`, `MILE-153`, `MILE-162`), for example `Deciders` on an `Accepted` record. A type declaring no `pending` keeps the old reading, so a value that only contains the word is still a real value. A type declaring no `require` entries is not examined. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`.",
    },
    RuleMeta {
        id: RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
        description: "A field the type's `statuses.forbid` bars at the record's current `Status` carries a value (`MILE-132`), for example an acceptance date on a record that is not accepted. A type declaring no `forbid` entries is not examined.",
    },
    RuleMeta {
        id: RULE_STATUS_DELIVERY_AGREEMENT,
        description: "A record's `Status` agrees with its delivery evidence, in both directions (`MILE-177`): the tier computed from its `Realized-by` locators and source back-pointers (drift ignored) is one of the type's `statuses.delivery.delivered_tiers` while the `Status` is one of its `not_done` statuses, so a built record still reads as unbuilt; and a `success` status whose evidence computes to one of its `unevidenced_tiers`, so a record claims to be done with nothing behind it. The mapping is the type's own declaration (`delivered_tiers`, `unevidenced_tiers`, `not_done`, `set_status`); the rule knows no status or type by name. `urzua fix --apply` writes `set_status` for the first direction; the second has no single correct value and is repaired by adding evidence. A type declaring no `delivery` is not examined. The population counts the records of types that declare it.",
    },
    RuleMeta {
        id: RULE_INDEX_FRESHNESS,
        description: "A committed index of one record type (a file, or a marked list inside a README, in a declared `format` with `{id}`, `{file}`, `{title}` and declared-field placeholders such as `{Status}`) lists exactly the records: a record with no entry, an entry naming no record, a duplicated entry, an entry whose file or title differs from the record's, and a line the format does not match are each an error (`MILE-139`). A `format` may span several lines (written with `\\n`), reading one entry over that many non-blank lines; it needs `{id}` or `{file}` (a link list that names no id is identified by its file); `{title}` is the H1 without its leading number or the record's own id (`MILE-152`). A missing artifact, or one whose declared `begin`/`end` markers are absent, is an error too. Needs `indexes` (`path`, `type`, `format`, optional `begin`/`end`); `check` only compares, and `urzua index` regenerates the artifacts from the records. The population counts the artifacts compared, so a run over none is distinguishable from a clean one.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        description: "A source comment line `Implements: ADR-12` (a comment leader such as `//`, `#`, `--` or `*`, then the marker, then comma- or space-separated record ids, each optionally followed by a note, as in `Implements: ADR-1 (why), ADR-2`) in a file under a `back_pointer_paths` entry counts as code-tier realization evidence for the record it cites, in `embodiment.consistency` and `urzua fix` (`MILE-128`, narrowing `RFC-5` section 2), and a back-pointer citing a record that does not exist is reported here. The marker is read at the start of a comment line or in a trailing comment after code (`//`, `#`, `--`, `/*`, `<!--` after whitespace); the marker is case-sensitive; reading continues past a note to the next comma-separated id and stops at a segment that does not open with one; ids are compared numerically. Opt-in and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs); evidence is collected only while this rule is enabled. A declared path covering no tracked file is itself a finding, and a non-text file is counted as unreadable, never as clean. The optional `test_paths` option (path prefixes, each inside a `back_pointer_paths` entry, else a load error, or globs such as `**/*.test.*` matched among the scanned files; a glob that cannot compile is a load error and one matching no scanned file is a finding) declares the test class: a `Verifies: ADR-12` comment in a file under it is test-tier evidence for the record it cites, and is checked for resolution like an `Implements:` one; a `Verifies:` comment elsewhere is not read (`MILE-146`).",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS,
        description: "A record whose `Status` is not an accepted status states an `Embodiment` of `Implemented` or `Verified` (`MILE-138`): a decision not yet made cannot have been built. The accepted statuses are the type's declared `statuses.success`; a type declaring none uses the rule's `accepted_statuses` option, and with neither the type is not examined. `Specified`, `Not started`, `Inactive` and `Drift detected` are not flagged, and a record missing either field is left to the header rules. Opt-in; the population counts the records the rule could judge.",
    },
    RuleMeta {
        id: RULE_DATE_FIELD_VALID,
        description: "A value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date, or is well-formed but not a real calendar date such as `2026-02-30`, is an error (`MILE-137`). A blank, placeholder or pending value is not a date value and is left to `field.quality` and `status.required-field-missing`. Needs `date_fields`. The population counts the date values examined.",
    },
    RuleMeta {
        id: RULE_DATE_FIELD_ORDER,
        description: "Within one record, a date field that is later than the field `date_order` says must not be later is an error: `Date <= Accepted-on` reports a record accepted before it was written (`MILE-157`). Equal dates pass. A value that is blank, a placeholder, pending or not a real `YYYY-MM-DD` date is not compared and is left to `date.field-valid`, `field.quality` and `status.required-field-missing`, which own those states. Applies to every record type that declares both fields of an entry. Needs `date_order`. The population counts one slot per record and entry; a record missing either date is eligible and unexamined.",
    },
    RuleMeta {
        id: RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
        description: "A source back-pointer (`Implements: ADR-12`, read exactly as `embodiment.back-pointer-resolution` reads it) citing a record whose `Status` is not one at which implementation is permitted is an error: code that says it implements a decision is landing ahead of that decision (`MILE-133`). Permitted means the cited type's declared `statuses.implementable` (a terminal `success` status is not assumed to permit it); a type that declares none uses the rule's `accepted_statuses`, and with neither the status cannot be judged, which is reported rather than passed. Needs `back_pointer_paths`; `accepted_statuses` is optional. On a `--base` run only back-pointers in files the diff changed are judged (file granularity). A citation of no record is `embodiment.back-pointer-resolution`'s, not this rule's. A declared path covering no tracked file is itself a finding. Mechanical fixes carry no back-pointer and are outside it. It checks the cited record's status, never whether the code matches the decision.",
    },
    RuleMeta {
        id: RULE_WAIVER_MATCHES_NOTHING,
        description: "A waiver that does nothing is reported on the waiver record, naming the rule and scope it claimed (`MILE-168`). Three cases: the waiver is expired (its `Expires` has passed or is not a `YYYY-MM-DD` date, so it already covers nothing); it names a rule this build does not ship; or it is active, its rule ran, and no finding of that rule falls in its scope. A waiver matching a finding is not reported, and an expired waiver is reported once, as expired. The unmatched case is judged only on a whole-corpus run in which the target rule ran: under a path scope or `--base`, or with the target rule off, an absent finding proves nothing, so the waiver is counted as eligible and unexamined (never out of scope). The expired and unknown-rule cases need only the waiver record to have been examined. Opt-in. The population counts waiver records.",
    },
];

/// Rules that derive a record's identity from its filename's type prefix
/// (`ADR-36` declined the bare `NNNN-slug` shape), so a corpus with no prefix
/// gives them nothing to examine. Single source of this fact: `init`'s
/// adopt-mode proposal (`BUG-61`) is the only consumer today, but the fact
/// belongs to the rules themselves, not to one command's module.
/// Rules whose findings name a file outside the corpus a `check <path>`
/// invocation scopes against -- `claim.status-agreement` reads a claim file
/// under `claim_paths`, which a record-type `dir` never covers. `BUG-67`'s
/// path-prefix scoping is correct for every rule reporting on a record; a
/// rule like this one needs the same unconditional exemption `check.rs`
/// already gives a finding about the config file itself, or `BUG-86`'s
/// "scoped invocation silently drops a real finding" recurs for any narrower
/// scope than the one that repository's own Makefile was changed to use.
///
/// The diff gates report on a changed source path, which is no record at all,
/// so the diff scope's "only examined records" filter would otherwise drop
/// exactly the finding they exist to make.
pub const RULES_REPORTING_OUTSIDE_THE_CORPUS: &[&str] = &[
    RULE_CLAIM_STATUS_AGREEMENT,
    RULE_DIFF_SOURCE_REQUIRES_RECORD,
    RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
    RULE_INDEX_FRESHNESS,
    // A dangling back-pointer is reported on the source file that holds it,
    // which no record-type `dir` scope covers.
    RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
    RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
];

/// Rules that need `relation_pairs` declared (`RFC-47`); enabling one with
/// none is a config load error.
pub const RULES_REQUIRING_RELATION_PAIRS: &[&str] = &[
    RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_CROSS_TYPE_DISALLOWED,
];

pub const IDENTITY_DEPENDENT_RULES: &[&str] = &[
    RULE_POINTER_RESOLUTION,
    RULE_POINTER_TARGET_STATUS,
    RULE_FILENAME_TITLE_CONSISTENCY,
    RULE_RELATION_SUPERSESSION_RECIPROCITY,
    RULE_NARRATIVE_FIELD_STALE,
    // Shares `for_each_resolved_status_target`'s `RecordIndex` lookup with
    // `RULE_POINTER_TARGET_STATUS` above -- a prefix-less corpus builds an
    // empty index (`record_id` returns `None` for every record), so this
    // rule resolves nothing for exactly the same reason its sibling does
    // (`BUG-61`'s shape, missed here when the rule was added).
    RULE_RELATION_TARGET_STATUS_UNDECLARED,
    RULE_RELATION_PAIR_RECIPROCITY,
    RULE_RELATION_CROSS_TYPE_DISALLOWED,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// `RULE_METADATA` feeds a generator, not a rule -- nothing else would
    /// catch it silently under- or over-reporting the rule set it's supposed
    /// to describe completely.
    #[test]
    fn every_rule_in_all_rules_has_metadata_in_the_same_order() {
        let all_rules_order: Vec<&str> = ALL_RULES.to_vec();
        let metadata_order: Vec<&str> = RULE_METADATA.iter().map(|m| m.id).collect();
        assert_eq!(
            all_rules_order, metadata_order,
            "RULE_METADATA must list every ALL_RULES entry, in the same order"
        );
        for meta in RULE_METADATA {
            assert!(
                !meta.description.trim().is_empty(),
                "{} has an empty description",
                meta.id
            );
        }
    }
}
