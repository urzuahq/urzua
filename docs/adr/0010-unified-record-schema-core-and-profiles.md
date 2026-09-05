# 0010 — A unified record schema: core fields plus declared profiles

> Status: Accepted
> Embodiment: Specified
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0001 (Accepted)

## Context

RFC-0001 proposed a single record schema shared by RFC, ADR, and Spec: a small fixed core (`id`,
`type`, `title`, `status`, `author`, `reviewers`/`deciders`, `date`, `supersedes`/`superseded_by`,
`related`) plus profile-specific extensions per type, with `type` declared per-profile rather than
a closed enum the tool hardcodes (§1a).

Phase 3's header parser and rule set have now been exercised against all 25 real records in this
corpus across three types (adr, rfc, spec), confirming the core/profile split holds in practice —
including finding the one place it doesn't yet (`SPEC-0001`'s header shape diverging from its
siblings, tracked by ADR-0008).

## Decision

In the context of three record types that would otherwise each need their own schema, validator,
and drift-detector, we decided to adopt **a small fixed core plus declared per-type profile
extensions**, with the profile mechanism itself declared in config (`.urzua/config.toml`'s
`record_types` table) rather than enumerated in the tool's source — an org can add a fourth record
type as a config change, not a code change.

Deferred from the RFC's fuller proposal, to be decided later rather than assumed now:

- **The `transitions` list (§1b)** and **the realization axis on Spec (§1c)** are schema extensions
  beyond what Phase 3 needed and beyond what this corpus currently exercises. Accepting the core
  schema now does not commit to these yet.
- **The relation-field grammar (§1d)** and **two-way reciprocity (§1e)** are real, needed rules —
  scoped to Phase 6 (the full rule set), not this ADR.

## Reversibility

The core-field set is the most load-bearing commitment in the schema — every rule, every future
profile, and every cross-record check assumes it. Changing it after real corpora depend on it is a
migration, not a config edit. Cheap now, at 25 records across one repository; expensive once
external adopters exist, which is exactly why this is being decided during Phase 5 rather than
deferred further.

## Consequences

- `.urzua/config.toml`'s `record_types` table is now the schema's actual extension point, not a
  future aspiration — confirmed by `urzua init`'s adopt mode building one from a real corpus.
- The three record types in this repository (adr, rfc, spec) are the profiles Urzua ships *by
  default* in its own dogfooding, not types the tool's source code special-cases.
- §1b (transitions), §1c (Spec realization axis), §1d (relation grammar), and §1e (reciprocity)
  remain open implementation work, tracked for Phase 6 rather than closed by this ADR.

## References

- RFC-0001 — the proposal this decides (core schema and §1a's profile-declaration model only).
- ADR-0008 — the header-closure model this schema's field set is validated through.
- `.urzua/config.toml` — the config shape that makes profiles declared, not enumerated.
