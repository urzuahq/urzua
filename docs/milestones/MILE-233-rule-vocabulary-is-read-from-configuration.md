---
Stable-Id: 01M7FVWCQYEXD0T7VB73XWWCXF
Status: InProgress
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 233 — Rule vocabulary is read from configuration

## What

Move the words rules compare against into configuration: the back-pointer markers and comment openers, the closing verbs, the Y-statement phrases, the placeholder words, and the waiver field names.

## Why

`ADR-101`'s amendment of 2026-11-15. the audit of 2026-11-15 found these vocabularies compiled in, in English, so a team with other conventions or another language cannot change them.

## Done means

- Each vocabulary has a config key whose default is today's list; a configuration that declares none behaves as before, shown by a test. — evidence: test:rust/crates/urzua-core/src/rules/options.rs#without_the_vocabulary_keys_the_effective_vocabulary_is_the_old_one@sha256:b5b1804fea16c0981167c019b27e9eb6040cff0d7aae3b362dbc666b37ffdc51, test:rust/crates/urzua-cli/tests/rule_vocabulary_integration.rs#without_the_new_keys_the_default_marker_is_read_as_before@sha256:64a84c1fe24bf79a107607d64af0fc19323affd78941c6978be513b90877956e
- A repository that declares another back-pointer marker and another placeholder list gets findings by its own words, shown by a test. — evidence: test:rust/crates/urzua-cli/tests/rule_vocabulary_integration.rs#a_declared_back_pointer_marker_and_leader_are_read_and_the_defaults_are_not@sha256:443e763bcfb3ab845151273286e0006bfa7ac854710b26a466b433d256c082d7, test:rust/crates/urzua-cli/tests/rule_vocabulary_integration.rs#declared_placeholder_words_replace_the_compiled_in_ones@sha256:5a4ee262f196c33b6fc387d1c49f1e92ce5ad16b0bd36fa0d18de16cf7189788
- `SPEC-3` lists which vocabularies stay fixed as format invariants (the pin marker, the scaffold placeholder, the override log columns) with their reasons. — evidence: spec:docs/specs/SPEC-3-configuration.md
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 4 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
