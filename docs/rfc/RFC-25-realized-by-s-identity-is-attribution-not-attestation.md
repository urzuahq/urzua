---
Stable-Id: 01M26CZYSJH627W6GSMHZGWHWP
Status: Draft
Date: 2026-09-10
Author: '@beauwilliams'
Amends: RFC-2
---
# 25 — Realized-by's identity is attribution, not attestation

## Summary

RFC-2 already explicitly disclaims cryptographic attestation: *"None of these tiers make the
attestation cryptographically provable... The goal is raising the cost of an accidental or careless
rubber-stamp, not achieving unforgeable attestation."* That caveat is correct and doesn't need
revisiting. The problem is narrower: the caveat lives only inside RFC-2's own Non-goals section, and
every downstream artifact built from it (`ADR-31`, `MILE-66`, and the changeset currently sitting in
`.changeset/identity-verified-first.md`) repeats the bare word **"verified"** without carrying that
nuance forward. A reader who only ever sees the changelog entry, not RFC-2 itself, has no way to know
the tool never claimed real attestation in the first place. This RFC proposes making the "attribution,
not attestation" framing travel with the word "verified" everywhere it appears in outward-facing text,
without changing any of RFC-2's actual mechanism (the `gh api user` → `git config` → `--by` priority
order stays exactly as `ADR-31` fixed it).

## Motivation

Grepping every place "verified" appears in connection with this identity-resolution mechanism finds
four hits: `RFC-2` itself (which states the caveat), `ADR-31`, `MILE-66`, and
`.changeset/identity-verified-first.md` — none of the latter three restate or link back to RFC-2's
own "not cryptographically provable" disclaimer. The revision-log text `fix --apply` actually writes
(`"Tool-authored (by {by}): recomputed..."`) is careful and accurate — it never claims verification,
only authorship. The gap is specifically in the human-facing prose surrounding the mechanism, not in
what the tool itself asserts when it writes.

This has real, immediate stakes: `.changeset/identity-verified-first.md` is one of the fragments
about to be compiled into `CHANGELOG.md` by this project's own first real release (`ADR-45`'s
`prepare-release` flow). A changelog entry is effectively permanent once published — this is the
last point where its wording can be corrected before it ships to whoever reads that release's notes.

Found live (`BUG-16`) while reviewing that same changeset's severity classification — the review
asked "is this actually safe" and re-reading RFC-2's own text closely enough to answer that
surfaced the wording gap.

## Proposal

1. **Revise `.changeset/identity-verified-first.md`'s wording** before it's consumed: replace bare
   "verified `gh` login" framing with language that names what's actually delivered (e.g., "the
   strongest locally-available identity signal" or an explicit one-clause caveat), matching RFC-2's
   own Non-goals rather than overclaiming past it.
2. **Adopt a standing convention**: whenever this project's own docs describe `gh api user`/`git
   config`-sourced identity as "verified," the sentence (or an adjacent one) states plainly that this
   is attribution-strength, not cryptographic attestation — not a one-time fix to three files, but a
   pattern to check for in future writing about this mechanism, the same way `AGENTS.md`'s
   doc-drift-sweep guardrail already asks for a whole-file grep before calling a wording change done.
3. **`ADR-31`/`MILE-66` stay as historical record, uncorrected** — per this project's own
   amendment-not-silent-edit model, their text isn't retroactively rewritten; this RFC's existence
   and its own reference back to them is how the correction is recorded.

No code change. `resolve_identity`'s actual priority order (`gh api user` → `git config` → `--by`)
is correct as shipped and is not reconsidered here.

## Open questions

- Does `urzua fix --apply --help` (or its man-page-equivalent, if one is ever built) need its own
  explicit one-line caveat, or is the changelog/RFC-level fix sufficient since `--help` text doesn't
  currently use the word "verified" at all? Leaning toward: sufficient as-is, revisit if `--help`
  text ever grows to describe identity resolution in more detail.
- Is there a single better replacement term for "verified" to standardize on across this project's
  future writing (e.g., "attributed," "locally-resolved"), or is a per-instance caveat clause
  preferable to a vocabulary change? Undesigned here.

## Non-goals

- Building real cryptographic attestation (signed commits, hardware keys, `NOT`-operator semantics)
  — explicitly out of scope per RFC-2's own Non-goals, unchanged by this RFC.
- Reversing or re-litigating `resolve_identity`'s actual tier ordering — `ADR-31`'s fix stands.
- A general audit of every other place this corpus uses words like "verified"/"attested" outside
  this specific identity-resolution mechanism — scoped to this one mechanism, found from one real
  changeset review, not a corpus-wide terminology sweep.

## References

- RFC-2 — the mechanism this RFC amends; its own Non-goals section already states the exact caveat
  this RFC asks to be carried forward consistently.
- ADR-31 — accepted RFC-2's tiering; uses "verified" without restating RFC-2's caveat.
- MILE-66 — built the fix; same pattern.
- BUG-16 — the finding that prompted this RFC.
- `.changeset/identity-verified-first.md` — the specific, time-sensitive artifact this RFC's
  Proposal §1 asks to be corrected before release.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial RFC, `Status: Draft`. | **structural** |
