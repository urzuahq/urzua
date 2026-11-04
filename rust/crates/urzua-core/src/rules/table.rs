use super::*;

/// Where a rule looks, which fixes what its population is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Loaded records: a header, a filename or a body.
    Records,
    /// The configuration alone.
    Config,
    /// Files that are not records: claim files, published files, source files,
    /// index artifacts, tracked paths.
    Files,
    /// The paths a `--base` run changed.
    Diff,
    /// The run itself: other rules' executions and findings, waiver records.
    Run,
}

/// How a rule meets a `header_shape: none` type (`ADR-50`, `MILE-119`): such a
/// type has nowhere for a header to be, so a rule that reads one classifies the
/// type's records `OutOfScope`. A rule that cannot read a header says why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderNone {
    OutOfScope,
    Exempt(&'static str),
}

/// One rule, with every property of it that anything outside the rule needs.
/// The single place a rule is registered: the shipped rule set, `urzua rules`,
/// `SPEC-2`'s rule table, config validation and `init`'s proposal are all
/// derived from `RULE_TABLE`, so a property cannot live in a second list.
pub struct RuleDef {
    pub id: &'static str,
    /// Adopter-facing; `SPEC-2`'s own rule table is generated from it.
    pub description: &'static str,
    pub scope: Scope,
    /// The file a finding of this rule sits on, which decides whether a path
    /// scope or `--base` keeps it.
    pub anchor: Anchor,
    /// Resolves through the identity index, which is built from filename type
    /// prefixes (`ADR-36`), so a corpus with no prefix gives it nothing to
    /// examine (`BUG-61`).
    pub needs_identity_index: bool,
    /// Enabling it with no `relation_pairs` declared is a config load error
    /// (`RFC-47`).
    pub requires_relation_pairs: bool,
    pub header_none: HeaderNone,
    /// The `RuleSetting` options the rule accepts, by name. Declaring one on any
    /// other rule is a load-time error: a silently ignored option is a check
    /// that never fires.
    pub options: &'static [&'static str],
    /// When in a run the rule executes.
    pub phase: Phase,
    /// Judges the run's inputs; never called for a rule the repository has not
    /// enabled, and its findings are given the declared severity by the runner.
    pub run: fn(&RuleCtx<'_>) -> (RuleExecution, Vec<Finding>),
}

/// What a rule's findings sit on. A scoped `check` (a path argument or
/// `--base`) keeps or drops a finding by this, never by rule id: a rule that
/// reports on something other than a record would otherwise be silently
/// dropped from a narrowed run (`BUG-67`, `BUG-86`, `BUG-121`).
#[derive(Clone, Copy)]
pub enum Anchor {
    /// A loaded record. A path scope keeps it under a requested prefix; a diff
    /// run keeps it when the record was examined.
    Record,
    /// The configuration file. Outside every scope, so every scope keeps it.
    Config,
    /// A file the rule reads that no record-type `dir` covers (a claim file, a
    /// source file, an index artifact, a changed path). Every scope keeps it:
    /// the run, not a record, is what it is about.
    Outside,
    /// A tracked path that is not a loaded record, judged by path. A path scope
    /// applies its prefixes; a diff run keeps it when the path changed.
    ChangedPath,
    /// A type's directory. A path scope applies its prefixes; a diff run keeps
    /// it when any changed path lies inside.
    Directory,
    /// A record, or a file the rule's own options declare (returned by the
    /// function, `None` when the rule is not enabled): a finding on that file
    /// is no record's and belongs to every scope.
    RecordOrDeclaredFile(fn(&Config) -> Option<PathBuf>),
}

/// When a rule runs. A rule that judges other rules' executions, or the
/// findings left after scoping, can only run after those exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Main,
    /// After every `Main` rule: it reads their executions.
    AfterRules,
    /// After scope filtering: it reads the findings the run will report.
    AfterScoping,
}

/// Every rule this build ships, in the order `check` runs them and lists them
/// in `rules_executed`.
pub const RULE_TABLE: &[RuleDef] = &[
    RuleDef {
        id: RULE_HEADER_REQUIRED_FIELDS,
        description: "Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            header_required_fields(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_POINTER_RESOLUTION,
        description: "A type's config-declared `pointer_fields` \u{222a} `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            pointer_resolution(c.records, c.config, c.index)
        },
    },
    RuleDef {
        id: RULE_POINTER_TARGET_STATUS,
        description: "A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["not_in", "fields"],
        phase: Phase::Main,
        run: |c| {
            let opts: PointerTargetOpts = c.config.rule_opts(RULE_POINTER_TARGET_STATUS);
            pointer_target_status(c.records, c.config, &opts.not_in, opts.fields.as_deref(), c.index)
        },
    },
    RuleDef {
        id: RULE_FIELD_QUALITY,
        description: "Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["field_levels"],
        phase: Phase::Main,
        run: |c| {
            field_quality(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_FIELD_PENDING,
        description: "A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `pending_until_required` option (`MILE-169`), off unless declared `true`, treats a pending value as legitimate until the record reaches a status the type's `statuses.require` ties the field to: a deferred slot stays eligible and is counted unexamined, and a field no `require` entry names is judged as before.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["pending_until_required", "field_levels"],
        phase: Phase::Main,
        run: |c| {
            field_pending(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_CLAIM_STATUS_AGREEMENT,
        description: "A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is claims, not records"),
        options: &["claim_paths", "closed_statuses"],
        phase: Phase::Main,
        run: |c| {
            let opts: ClaimOpts = c.config.rule_opts(RULE_CLAIM_STATUS_AGREEMENT);
            claim_status_agreement(c.claims, &opts.closed_statuses, c.config, c.index)
        },
    },
    RuleDef {
        id: RULE_FILENAME_TITLE_CONSISTENCY,
        description: "The filename's display number and the H1's display number agree (ADR-3).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads the filename and H1 only"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            filename_title_consistency(c.records, c.full_text)
        },
    },
    RuleDef {
        id: RULE_RELATION_SUPERSESSION_RECIPROCITY,
        description: "If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| supersession_reciprocity(c.records, c.config, c.index),
    },
    RuleDef {
        id: RULE_RELATION_PAIR_RECIPROCITY,
        description: "For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. A pair that declares `types` is carried by those types only, and a claim to a type outside the list is excluded as out of scope (`MILE-155`). A pair that declares `provisional: {statuses, level}` reports an unechoed claim from a claimant at one of those statuses at the provisional level (default `warn`), and any other claimant at the rule's level (`MILE-154`). The optional `provisional.acknowledged_by` names a field on the peer that settles a provisional claim by naming the claimant (`MILE-163`); it is not read for any other claimant. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: true,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            relation_pair_reciprocity(c.records, c.config, c.index)
        },
    },
    RuleDef {
        id: RULE_RELATION_CROSS_TYPE_DISALLOWED,
        description: "A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: true,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            relation_cross_type_disallowed(c.records, c.config, c.index)
        },
    },
    RuleDef {
        id: RULE_RELATION_TARGET_STATUS_UNDECLARED,
        description: "A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| relation_target_status_undeclared(c.records, c.config, c.index),
    },
    RuleDef {
        id: RULE_REVISION_LOG_CHANGE_CLASS_REQUIRED,
        description: "Every revision-log entry carries a `change_class` (ADR-14); missing or unclassified is blocking. The log is the section a type's `revision_log` identity (default the line `**Revision log**`) opens, found as that line and never by a line that mentions the text (`MILE-167`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads body text only"),
        options: &[],
        phase: Phase::Main,
        run: |c| revision_log_change_class(c.records, c.config, c.full_text),
    },
    RuleDef {
        id: RULE_EMBODIMENT_CONSISTENCY,
        description: "A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (`test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to `Drift detected` unconditionally (RFC-5 tier 1, ADR-18/ADR-32). A record with no `Realized-by` field is judged on the back-pointers that cite it (`MILE-149`); one stating `Verified` or `Implemented` that has neither a `Realized-by` field nor a citing back-pointer is reported as a claim with no evidence, computed `Not started` (`MILE-161`), and one stating any other value is not judged and stays unexamined. Requires full git history — a shallow checkout makes this rule silently unable to detect anything. Three opt-in refinements, all off by default (`MILE-129`): a locator written `path@sha256:<64 hex>` drifts when the file's content hash differs and not otherwise, in place of the git-history heuristic; the `drift_grace_days` option holds back reporting drift whose locator last changed fewer than that many days ago, and a record in that window agrees with either its tier or a stated `Drift detected` (the window never contradicts a record that already says so, and `urzua fix` leaves it alone); the optional `drift_audit_field` option names a declared date field (needs `drift_grace_days`): a record carrying a `YYYY-MM-DD` date there has its window counted from that last-audit date instead of from the cited file's last change, and a malformed date is reported and falls back to the file (`MILE-166`); the `claim_max_age_days` option warns when the `Realized-by` line was last touched more than that many days ago. The optional `sticky_states` option lists `Embodiment` values a human sets that no evidence can compute, such as `Deprecated` and `Inactive`: a record stating one is examined and never reported as drift or as an old claim, `urzua fix` leaves it alone, and a listed value that evidence does compute (`Verified`, `Implemented`, `Specified`, `Not started`, `Drift detected`) is a load error because it would hide real drift (`MILE-148`). Source back-pointers (see `embodiment.back-pointer-resolution`) are evidence too: an `Implements:` citation is code tier and a `Verifies:` citation in a declared test file is test tier, equal to a `test:` locator (`MILE-146`). A record with `Embodiment` but no `Realized-by` field is judged on back-pointer evidence alone when one cites it, and a record neither cited nor carrying `Realized-by` is not judged (`MILE-149`); a finding names the evidence it computed from.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["drift_grace_days", "drift_audit_field", "claim_max_age_days", "sticky_states"],
        phase: Phase::Main,
        run: |c| {
            embodiment_consistency(
                c.records,
                c.config,
                c.drifted,
                c.fresh,
                c.back_pointed,
                c.claim_ages,
            )
        },
    },
    RuleDef {
        id: RULE_EMBODIMENT_REQUIRES_ACCEPTED_STATUS,
        description: "A record whose `Status` is not an accepted status states an `Embodiment` of `Implemented` or `Verified` (`MILE-138`): a decision not yet made cannot have been built. The accepted statuses are the type's declared `statuses.success`; a type declaring none uses the rule's `accepted_statuses` option, and with neither the type is not examined. `Specified`, `Not started`, `Inactive` and `Drift detected` are not flagged, and a record missing either field is left to the header rules. Opt-in; the population counts the records the rule could judge.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["accepted_statuses"],
        phase: Phase::Main,
        run: |c| embodiment_requires_accepted_status(c.records, c.config),
    },
    RuleDef {
        id: RULE_EMBODIMENT_BACK_POINTER_RESOLUTION,
        description: "A source comment line `Implements: ADR-12` (a comment leader such as `//`, `#`, `--` or `*`, then the marker, then comma- or space-separated record ids, each optionally followed by a note, as in `Implements: ADR-1 (why), ADR-2`) in a file under a `back_pointer_paths` entry counts as code-tier realization evidence for the record it cites, in `embodiment.consistency` and `urzua fix` (`MILE-128`, narrowing `RFC-5` section 2), and a back-pointer citing a record that does not exist is reported here. The marker is read at the start of a comment line or in a trailing comment after code (`//`, `#`, `--`, `/*`, `<!--` after whitespace); the marker is case-sensitive; reading continues past a note to the next comma-separated id and stops at a segment that does not open with one; ids are compared numerically. Opt-in and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs); evidence is collected only while this rule is enabled. A declared path covering no tracked file is itself a finding, and a non-text file is counted as unreadable, never as clean. The optional `test_paths` option (path prefixes, each inside a `back_pointer_paths` entry, else a load error, or globs such as `**/*.test.*` matched among the scanned files; a glob that cannot compile is a load error and one matching no scanned file is a finding) declares the test class: a `Verifies: ADR-12` comment in a file under it is test-tier evidence for the record it cites, and is checked for resolution like an `Implements:` one; a `Verifies:` comment elsewhere is not read (`MILE-146`).",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is source files, not records"),
        options: &["back_pointer_paths", "test_paths"],
        phase: Phase::Main,
        run: |c| {
                let (files, paths, tests): (&[_], &[String], &[String]) =
                    match c.back_pointer_sources {
                        Some(s) => (&s.files, &s.back_pointer_paths, &s.test_paths),
                        None => (&[], &[], &[]),
                    };
                embodiment_back_pointer_resolution(
                    files,
                    paths,
                    tests,
                    c.config,
                    c.index,
                )
            },
    },
    RuleDef {
        id: RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED,
        description: "A source back-pointer (`Implements: ADR-12`, read exactly as `embodiment.back-pointer-resolution` reads it) citing a record whose `Status` is not one at which implementation is permitted is an error: code that says it implements a decision is landing ahead of that decision (`MILE-133`). Permitted means the cited type's declared `statuses.implementable` (a terminal `success` status is not assumed to permit it); a type that declares none uses the rule's `accepted_statuses`, and with neither the status cannot be judged, which is reported rather than passed. Needs `back_pointer_paths`; `accepted_statuses` is optional. On a `--base` run only back-pointers in files the diff changed are judged (file granularity). A citation of no record is `embodiment.back-pointer-resolution`'s, not this rule's. A declared path covering no tracked file is itself a finding. Mechanical fixes carry no back-pointer and are outside it. It checks the cited record's status, never whether the code matches the decision.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is source files, not records"),
        options: &["back_pointer_paths", "accepted_statuses"],
        phase: Phase::Main,
        run: |c| {
                let (files, paths): (&[_], &[String]) = match c.decision_gate_sources {
                    Some(s) => (&s.files, &s.back_pointer_paths),
                    None => (&[], &[]),
                };
                let fallback = c.config
                    .rule_opts::<DecisionAcceptedOpts>(RULE_EMBODIMENT_BACK_POINTER_DECISION_ACCEPTED)
                    .accepted_statuses;
                embodiment_back_pointer_decision_accepted(
                    files,
                    paths,
                    &fallback,
                    c.changed_paths,
                    c.config,
                    c.index,
                )
            },
    },
    RuleDef {
        id: RULE_EMBODIMENT_LOCATOR_EXISTS,
        description: "A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, or is empty. Tracked alone passes a staged deletion; on disk alone passes a gitignored file.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            // Tracked *and* on disk: tracked alone passes a staged deletion,
            // since discovery unions `ls-files` with the cached diff, and on
            // disk alone passes a gitignored file that then fails in CI.
            let tracked: std::collections::HashSet<&std::path::Path> =
                c.tracked_paths.iter().map(|p| p.as_path()).collect();
            let present = |p: &str| {
                let path = std::path::Path::new(p);
                if !tracked.contains(path) {
                    return false;
                }
                // `is_file()` folds every error into `false`, so an unreadable
                // path would be reported as missing. This rule is declared
                // `error` in repositories that enable it, and a blocking
                // finding the tool cannot substantiate is worse than silence --
                // so only a definite absence counts.
                (c.may_exist)(path)
            };
            embodiment_locator_exists(c.records, c.config, &present)
        },
    },
    RuleDef {
        id: RULE_EMBODIMENT_LOCATOR_PROMOTION_CANDIDATE,
        description: "The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| embodiment_locator_promotion_candidate(c.records, c.config),
    },
    RuleDef {
        id: RULE_HEADER_FIELD_SET_CONSISTENCY,
        description: "Every header field belongs to its type's `required_fields` \u{222a} `known_fields`, when `known_fields` is declared (ADR-39). The optional `allow_extension_prefix` option (`MILE-141`), off unless declared `true`, exempts header keys beginning `x-` in any case; an unprefixed undeclared key is still a finding.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["allow_extension_prefix"],
        phase: Phase::Main,
        run: |c| {
            header_field_set_consistency(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_FIELD_UNTRIMMED_VALUE,
        description: "A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            field_untrimmed_value(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_FIELD_LEADING_RESERVED_INDICATOR,
        description: "A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| field_leading_reserved_indicator(c.records, c.config),
    },
    RuleDef {
        id: RULE_HEADER_FIELD_CASE_MISMATCH,
        description: "A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            header_field_case_mismatch(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_HEADER_CASE_VARIANT_KEY,
        description: "One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            header_case_variant_key(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_NARRATIVE_FIELD_STALE,
        description: "A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: true,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["terminal_statuses"],
        phase: Phase::Main,
        run: |c| {
            let opts: NarrativeStaleOpts = c.config.rule_opts(RULE_NARRATIVE_FIELD_STALE);
            narrative_field_stale(c.records, c.config, &opts.terminal_statuses, c.index)
        },
    },
    RuleDef {
        id: RULE_TYPE_DIR_MATCHES_NOTHING,
        description: "A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. Also reports a type\u{2019}s `exclude_files` entry that names no tracked file directly in its `dir` (`MILE-150`), so a stale exclusion is visible. Reports each `extra_dirs` entry that does not exist (`MILE-130`).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            let mut matched: HashMap<String, usize> = HashMap::new();
            for r in c.records {
                *matched.entry(r.record_type.clone()).or_insert(0) += 1;
            }
            let dir_exists = |d: &str| (c.dir_exists)(d);
            type_dir_matches_nothing(c.config, c.config_path, &matched, &dir_exists, c.live_paths)
        },
    },
    RuleDef {
        id: RULE_IDENTITY_COLLISION,
        description: "Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads the filename identifier only"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            identity_collision(c.records, c.collisions)
        },
    },
    RuleDef {
        id: RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR,
        description: "A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise.",
        scope: Scope::Files,
        anchor: Anchor::ChangedPath,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges tracked paths against the declared directories"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
                // A staged deletion is going away, not a real ownership
                // question to raise (`BUG-71`/`BUG-82`'s shape, one level up:
                // those fixed a staged deletion being wrongly judged *inside*
                // its dir; this is the same file wrongly judged *outside* one
                // it's leaving).
                type_record_outside_declared_dir(c.config, c.live_paths)
            },
    },
    RuleDef {
        id: RULE_TYPE_NO_DECLARED_SPEC,
        description: "Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            type_no_declared_spec(c.config, c.config_path)
        },
    },
    RuleDef {
        id: RULE_HEADER_POINTER_FIELD_CLEAN,
        description: "A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            header_pointer_field_clean(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_CONFIG_POINTER_DECLARATION_MISSING,
        description: "A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| config_pointer_declaration_missing(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_POINTER_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            config_pointer_field_not_known(c.config, c.config_path)
        },
    },
    RuleDef {
        id: RULE_CONFIG_POINTER_NARRATIVE_OVERLAP,
        description: "A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| config_pointer_narrative_overlap(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_HEADER_NONE_HAS_NO_REQUIRED_FIELDS,
        description: "A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| config_header_none_has_no_required_fields(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_CONFIG_RELATION_FIELD_NOT_KNOWN,
        description: "Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`), including the `date` and `author` fields `urzua new` fills (`MILE-176`), must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`.",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            config_relation_field_not_known(c.config, c.config_path)
        },
    },
    RuleDef {
        id: RULE_CONFIG_KNOWN_FIELDS_DECLARATION_MISSING,
        description: "A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s \"declared, not voted\" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`).",
        scope: Scope::Config,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges the configuration, never a record's header"),
        options: &[],
        phase: Phase::Main,
        run: |c| config_known_fields_declaration_missing(c.config, c.config_path),
    },
    RuleDef {
        id: RULE_FILENAME_IDENTITY_PATTERN,
        description: "A file directly in a type's `dir` whose name does not match the type's declared `identity.pattern` (or, for a type declaring none, the `TYPE-NNNN-slug.md` convention) is reported rather than left out of every rule's population (`MILE-120`). When every file in the directory is off-pattern, such as a bare-number corpus (`0001-slug.md`), it is one finding per type naming the expected file name for each file, since renaming is the supported route (`RFC-46` was rejected). The census counts each file examined, and omits a file the type declares in `exclude_files` (`MILE-150`).",
        scope: Scope::Records,
        anchor: Anchor::Directory,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads the filename only"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            filename_identity_pattern(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_UNDECLARED_VALUE,
        description: "A record's `Status` is not one of the values its type declares in `statuses.values` (`MILE-132`). A type declaring no `statuses` is not examined, and a record with no `Status` is left to `header.required-fields`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            status_undeclared_value(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_REQUIRED_FIELD_MISSING,
        description: "A field the type's `statuses.require` ties to the record's current `Status` is blank, unedited template text, or (when the type declares `statuses.pending`) one of that declared placeholder vocabulary's whole values such as `Unknown` or `none recorded`, or a value matching one of its `{pattern: ...}` entries for a placeholder family (`MILE-132`, `MILE-153`, `MILE-162`), for example `Deciders` on an `Accepted` record. A type declaring no `pending` keeps the old reading, so a value that only contains the word is still a real value. A type declaring no `require` entries is not examined. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["field_levels"],
        phase: Phase::Main,
        run: |c| {
            status_required_field_missing(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_FORBIDDEN_FIELD_PRESENT,
        description: "A field the type's `statuses.forbid` bars at the record's current `Status` carries a value (`MILE-132`), for example an acceptance date on a record that is not accepted. A type declaring no `forbid` entries is not examined.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            status_forbidden_field_present(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_STATUS_DELIVERY_AGREEMENT,
        description: "A record's `Status` agrees with its delivery evidence, in both directions (`MILE-183`): the tier computed from its `Realized-by` locators and source back-pointers (drift ignored) is one of the type's `statuses.delivery.delivered_tiers` while the `Status` is one of its `not_done` statuses, so a built record still reads as unbuilt; and a `success` status whose evidence computes to one of its `unevidenced_tiers`, so a record claims to be done with nothing behind it. The mapping is the type's own declaration (`delivered_tiers`, `unevidenced_tiers`, `not_done`, `set_status`); the rule knows no status or type by name. `urzua fix --apply` writes `set_status` for the first direction; the second has no single correct value and is repaired by adding evidence. A type declaring no `delivery` is not examined. The population counts the records of types that declare it.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &[],
        phase: Phase::Main,
        run: |c| {
            status_delivery_agreement(c.records, c.config, c.back_pointed)
        },
    },
    RuleDef {
        id: RULE_DATE_FIELD_VALID,
        description: "A value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date, or is well-formed but not a real calendar date such as `2026-02-30`, is an error (`MILE-137`). A blank, placeholder or pending value is not a date value and is left to `field.quality` and `status.required-field-missing`. Needs `date_fields`. The population counts the date values examined.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["date_fields"],
        phase: Phase::Main,
        run: |c| {
            date_field_valid(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_DATE_FIELD_ORDER,
        description: "Within one record, a date field that is later than the field `date_order` says must not be later is an error: `Date <= Accepted-on` reports a record accepted before it was written (`MILE-157`). Equal dates pass. A value that is blank, a placeholder, pending or not a real `YYYY-MM-DD` date is not compared and is left to `date.field-valid`, `field.quality` and `status.required-field-missing`, which own those states. Applies to every record type that declares both fields of an entry. Needs `date_order`. The population counts one slot per record and entry; a record missing either date is eligible and unexamined.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::OutOfScope,
        options: &["date_order"],
        phase: Phase::Main,
        run: |c| {
            date_field_order(c.records, c.config)
        },
    },
    RuleDef {
        id: RULE_CITATION_RESOLUTION,
        description: "A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in. The optional `external_ids` option (`MILE-156`) lists ids that share a record prefix but are not records, each an exact id (`RFC-4122`) or an inclusive numeric range written `PREFIX-LOW..HIGH` under a declared prefix; a matching citation is counted as eligible and unexamined, never resolved or reported, and not as out of scope, so `config.scope-matches-nothing` does not read a declared id as a misconfiguration. An entry under no declared prefix is a config error. Its population counts the citations, so a run over none is distinguishable from a clean one, and external citations are visible as eligible and unexamined. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that cites an id as it was is not reported; it is `false` by default (`MILE-156`).",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads body text only; a header-less record is read whole"),
        options: &["external_ids", "skip_revision_log"],
        phase: Phase::Main,
        run: |c| {
            citation_resolution(c.records, c.config, c.full_text, c.index)
        },
    },
    RuleDef {
        id: RULE_CITATION_PATH_EXISTS,
        description: "A repository path cited in backticks inside a `reference_sections` section (a `##` heading, matched exactly) exists in the working tree, tracked and on disk (`MILE-143`). A cited path is a code span with no whitespace, containing a `/` and ending in a file name with an extension or in `/`; a URL, a host-led path (`example.com/x`), an absolute or `~` path, a glob or placeholder (`*?<>{}$|`) and anything else is not a path and is not read. A trailing `:line` or `#anchor` is dropped, and `./` or `../` resolves from the record's directory. Fenced code blocks are not read. Needs `reference_sections`; a record without such a section is out of scope, and the population counts the paths cited. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that names a path as it was is not reported. It is `false` by default. Offline and deterministic. Cited issue and pull request numbers are `citation.issue-exists`'s.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads body text only; a header-less record is read whole"),
        options: &["reference_sections", "skip_revision_log"],
        phase: Phase::Main,
        run: |c| {
            // A file is present when tracked and on disk (as for
            // `embodiment.locator-exists`); a directory when a tracked file
            // sits below it.
            let tracked: std::collections::HashSet<&std::path::Path> =
                c.tracked_paths.iter().map(|p| p.as_path()).collect();
            let directories: std::collections::HashSet<&std::path::Path> = c
                .tracked_paths
                .iter()
                .flat_map(|p| p.ancestors().skip(1))
                .filter(|a| !a.as_os_str().is_empty())
                .collect();
            let present = |p: &str| {
                let path = std::path::Path::new(p);
                if directories.contains(path) {
                    return true;
                }
                // Only a definite absence counts, as for `embodiment.locator-exists`.
                tracked.contains(path) && (c.may_exist)(path)
            };
            citation_path_exists(c.records, c.config, c.full_text, &present)
        },
    },
    RuleDef {
        id: RULE_CITATION_ISSUE_EXISTS,
        description: "An issue or pull request cited inside a `reference_sections` section (a `##` heading, matched exactly) is in the committed snapshot of the repository's numbers (`MILE-143`). `issue_refs` declares the `snapshot` file (relative to the repository root), the `repository` (`owner/name`) it describes and the citation `forms` read: `number` is `#123` (outside inline code, not `owner/repo#123`, not an anchor link `](#123)`), `url` is a `https://github.com/owner/name/issues/123` or `/pull/123` link to that repository; a form not declared is not read. A number is judged against the snapshot's issues and pull requests together, since GitHub shares one number space and redirects either URL. Offline and deterministic: `check` never reaches the network, and only `urzua refs refresh` writes the snapshot. A snapshot that is absent, unreadable, of another schema or of another repository is one finding on the snapshot path, and every citation is counted as eligible and unexamined: the rule never passes for want of its input. Each finding names the date the snapshot was generated, and `check` prints a notice with that date and the snapshot's size, so staleness is visible. The optional `skip_revision_log` option behaves as for `citation.path-exists`. The optional `issue_refs.fields` names header fields (as a type declares them, such as `Found-in`) whose values are read for the same forms, each finding naming the field and its line; a name no record type declares is refused at load, and a record without the field is not examined. Body prose outside `reference_sections` is deliberately not readable. The population's unit is the citation.",
        scope: Scope::Records,
        anchor: Anchor::RecordOrDeclaredFile(|config| {
            config
                .rules
                .get(RULE_CITATION_ISSUE_EXISTS)
                .filter(|s| s.level != crate::config::RuleLevel::Off)
                .and_then(|s| s.issue_refs.as_ref())
                .map(|r| PathBuf::from(&r.snapshot))
        }),
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads body text only; a header-less record is read whole"),
        options: &["reference_sections", "skip_revision_log", "issue_refs"],
        phase: Phase::Main,
        run: |c| {
            citation_issue_exists(c.records, c.config, c.full_text, c.refs_snapshot)
        },
    },
    RuleDef {
        id: RULE_BOUNDARY_PUBLISHED_PATH_REFERENCE,
        description: "A record identifier (any identifier carrying a configured type prefix, resolvable or not) is cited from inside a `published_paths` entry (a path prefix, normalised at config load; globs are not supported and a glob is treated literally), which publishes a pointer no external reader can resolve (`RFC-11`/`ADR-9`, `MILE-125`). An identifier split across lines is not detected. A declared path matching no tracked file is itself a finding, never a clean pass; the rule's level is the boundary's severity.",
        scope: Scope::Files,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is published files, not records"),
        options: &["published_paths"],
        phase: Phase::Main,
        run: |c| published_path_reference(c.published_files, c.published_paths, c.config),
    },
    RuleDef {
        id: RULE_SECTION_REQUIRED,
        description: "Each `##` section a type's declared `sections.required` names is present in every record of that type (`MILE-4`, `RFC-17`), under its `name` or one of its declared `aliases` (`MILE-165`; `check` discloses each alias in use as an info notice), or, for a `y-statement` declaring `preamble: true` and a `marker`, in the text before the first `##`. A type that declares none is out of scope; a record whose status is in the declaration's `exempt_statuses` (a record that made no decision, `MILE-7`) is counted eligible and unexamined, not asked for the sections, and not out of scope, so `config.scope-matches-nothing` does not read a declared exemption as a misconfiguration (`BUG-187`). Presence only: it never judges what a section says.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads body sections; the header only for exempt_statuses, which a header-less type declaring them is counted out of scope for (unit-tested)"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            section_required(c.records, c.config, c.full_text)
        },
    },
    RuleDef {
        id: RULE_SECTION_SHAPE,
        description: "A required section that declares a `shape` has that shape (`MILE-5`, `MILE-121`): `y-statement` needs its TL;DR block to carry the five phrases `In the context of`, `facing`, `we decided`, `to achieve`, `accepting`, word-boundary matched, and reports a marker with no block as its own error; `table-min-rows` needs a table with at least `min_rows` data rows. A section found under an alias is checked too, and a `preamble: true` Y-statement is judged from the text before the first `##`. A missing section is `section.required`'s to report, not this rule's. Shape only, never meaning.",
        scope: Scope::Records,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("reads body sections, as section.required does"),
        options: &[],
        phase: Phase::Main,
        run: |c| {
            section_shape(c.records, c.config, c.full_text)
        },
    },
    RuleDef {
        id: RULE_INDEX_FRESHNESS,
        description: "A committed index of one record type (a file, or a marked list inside a README, in a declared `format` with `{id}`, `{file}`, `{title}` and declared-field placeholders such as `{Status}`) lists exactly the records: a record with no entry, an entry naming no record, a duplicated entry, an entry whose file or title differs from the record's, and a line the format does not match are each an error (`MILE-139`). A `format` may span several lines (written with `\\n`), reading one entry over that many non-blank lines; it needs `{id}` or `{file}` (a link list that names no id is identified by its file); `{title}` is the H1 without its leading number or the record's own id (`MILE-152`). A missing artifact, or one whose declared `begin`/`end` markers are absent, is an error too. Needs `indexes` (`path`, `type`, `format`, optional `begin`/`end`); `check` only compares, and `urzua index` regenerates the artifacts from the records. The population counts the artifacts compared, so a run over none is distinguishable from a clean one.",
        scope: Scope::Files,
        anchor: Anchor::Outside,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is index artifacts, not records"),
        options: &["indexes"],
        phase: Phase::Main,
        run: |c| {
            index_freshness(c.records, c.full_text, c.index_artifacts)
        },
    },
    RuleDef {
        id: RULE_DIFF_SOURCE_REQUIRES_RECORD,
        description: "A `--base` diff changes a path under a declared `source_paths` entry (a prefix or a glob) but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`, `MILE-158`). A changed path matching `exclude_paths` is not source and stays in the population unexamined, so a diff touching only such files (tests beside the code) is not a trigger. One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed.",
        scope: Scope::Diff,
        anchor: Anchor::Outside,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is changed paths, and only on a --base run"),
        options: &["source_paths", "exclude_paths"],
        phase: Phase::Main,
        run: |c| {
            let opts: SourceRequiresRecordOpts =
                c.config.rule_opts(RULE_DIFF_SOURCE_REQUIRES_RECORD);
            source_requires_record(c.changed_paths, &opts.source_paths, &opts.exclude_paths, c.config)
        },
    },
    RuleDef {
        id: RULE_DIFF_SOURCE_AND_RECORDS_MIXED,
        description: "A `--base` diff changes both a governed record and a path under a declared `application_paths` entry, a prefix or a glob (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean.",
        scope: Scope::Diff,
        anchor: Anchor::Outside,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("population is changed paths, and only on a --base run"),
        options: &["application_paths", "exclude_paths"],
        phase: Phase::Main,
        run: |c| {
            let opts: SourceAndRecordsMixedOpts =
                c.config.rule_opts(RULE_DIFF_SOURCE_AND_RECORDS_MIXED);
            source_and_records_mixed(
                c.changed_paths,
                &opts.application_paths,
                &opts.exclude_paths,
                c.config,
            )
        },
    },
    RuleDef {
        id: RULE_CONFIG_SCOPE_MATCHES_NOTHING,
        description: "Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`).",
        scope: Scope::Run,
        anchor: Anchor::Config,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges other rules' populations"),
        options: &[],
        phase: Phase::AfterRules,
        run: |c| config_scope_matches_nothing(c.run.executions, c.config_path),
    },
    RuleDef {
        id: RULE_WAIVER_MATCHES_NOTHING,
        description: "A waiver that does nothing is reported on the waiver record, naming the rule and scope it claimed (`MILE-168`). Three cases: the waiver is expired (its `Expires` has passed or is not a `YYYY-MM-DD` date, so it already covers nothing); it names a rule this build does not ship; or it is active, its rule ran, and no finding of that rule falls in its scope. A waiver matching a finding is not reported, and an expired waiver is reported once, as expired. The unmatched case is judged only on a whole-corpus run in which the target rule ran: under a path scope or `--base`, or with the target rule off, an absent finding proves nothing, so the waiver is counted as eligible and unexamined (never out of scope). The expired and unknown-rule cases need only the waiver record to have been examined. Opt-in. The population counts waiver records.",
        scope: Scope::Run,
        anchor: Anchor::Record,
        needs_identity_index: false,
        requires_relation_pairs: false,
        header_none: HeaderNone::Exempt("judges waiver records against the run's findings, not a configured header"),
        options: &[],
        phase: Phase::AfterScoping,
        run: |c| waiver_matches_nothing(
            c.run.waivers,
            c.run.findings,
            c.run.executions,
            c.run.whole_corpus,
            |p| (c.run.reached)(p),
            c.run.today,
        ),
    },
];

const fn ids<const N: usize>(table: &[RuleDef]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut i = 0;
    while i < N {
        out[i] = table[i].id;
        i += 1;
    }
    out
}

/// Every rule this build ships, by id. One canonical name per rule: the config
/// is validated against this list, so a typo is a load-time error naming the
/// valid set rather than a check that silently never runs. Derived from
/// `RULE_TABLE`.
pub const ALL_RULES: &[&str] = &ids::<{ RULE_TABLE.len() }>(RULE_TABLE);

/// Whether `rule` accepts the option named `option`.
pub fn rule_accepts_option(rule: &str, option: &str) -> bool {
    rule_def(rule).is_some_and(|d| d.options.contains(&option))
}

/// The table entry for a rule id.
pub fn rule_def(id: &str) -> Option<&'static RuleDef> {
    RULE_TABLE.iter().find(|r| r.id == id)
}

/// Ids of the rules whose `RuleDef` satisfies `pred`.
pub fn rule_ids_where(pred: impl Fn(&RuleDef) -> bool) -> impl Iterator<Item = &'static str> {
    RULE_TABLE.iter().filter(move |r| pred(r)).map(|r| r.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_has_a_unique_id_and_a_description() {
        let mut seen = std::collections::HashSet::new();
        for def in RULE_TABLE {
            assert!(seen.insert(def.id), "{} is in the table twice", def.id);
            assert!(
                !def.description.trim().is_empty(),
                "{} has an empty description",
                def.id
            );
        }
        assert_eq!(ALL_RULES.len(), RULE_TABLE.len());
    }

    /// A rule that does not look at records has no header to read, so the
    /// table must say so; a record-scoped rule is exempt only with a reason.
    #[test]
    fn a_rule_outside_the_records_is_exempt_from_the_header_none_check() {
        for def in RULE_TABLE {
            if def.scope != Scope::Records {
                assert!(
                    matches!(def.header_none, HeaderNone::Exempt(_)),
                    "{} does not look at records, so it must carry a header_none exemption",
                    def.id
                );
            }
            if let HeaderNone::Exempt(reason) = def.header_none {
                assert!(!reason.trim().is_empty(), "{} has an empty reason", def.id);
            }
        }
    }

    /// A rule id constant with no table entry is a rule that was declared and
    /// never registered: it would not load from a config, and no check would
    /// run it.
    #[test]
    fn every_rule_id_constant_has_a_table_entry() {
        let source = include_str!("catalog.rs");
        let mut declared = 0;
        for line in source.lines() {
            let Some(rest) = line.strip_prefix("pub const RULE_") else {
                continue;
            };
            if !rest.contains(": &str") {
                continue;
            }
            declared += 1;
            let name = format!("RULE_{}", rest.split(':').next().unwrap());
            let literal_on_this_line = rest.split('"').nth(1);
            // A constant whose value wraps onto the next line is read from it.
            let id = match literal_on_this_line {
                Some(id) => id.to_string(),
                None => {
                    let next = source
                        .lines()
                        .skip_while(|l| *l != line)
                        .nth(1)
                        .unwrap_or_default();
                    next.split('"').nth(1).unwrap_or_default().to_string()
                }
            };
            assert!(
                RULE_TABLE.iter().any(|d| d.id == id),
                "{name} (\"{id}\") has no entry in RULE_TABLE"
            );
        }
        assert_eq!(declared, RULE_TABLE.len());
    }

    /// A rule that judges the configuration reports on the configuration; one
    /// that reads files outside the corpus is anchored there. Declared once, in
    /// the entry, which is what a scoped run reads.
    #[test]
    fn a_rule_anchor_agrees_with_its_scope() {
        for def in RULE_TABLE {
            match def.scope {
                Scope::Config => assert!(
                    matches!(def.anchor, Anchor::Config),
                    "{} judges the configuration, so its findings sit on it",
                    def.id
                ),
                Scope::Diff => assert!(
                    matches!(def.anchor, Anchor::Outside),
                    "{} judges changed paths, which are no record",
                    def.id
                ),
                Scope::Records | Scope::Files | Scope::Run => {}
            }
        }
    }
}
