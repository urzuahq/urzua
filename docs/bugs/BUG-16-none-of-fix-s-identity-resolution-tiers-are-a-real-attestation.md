---
Stable-Id: 01M268P3T0P9RDA78AWBXAJ4HR
Status: Open
Found-in: 'raised as "shouldn''t an explicit --by win as an intentional override" while reviewing this changeset''s own semver classification -- checking RFC-2''s actual stated reasoning (rather than assuming the raised concern was correct) found the opposite: RFC-2 exists specifically because the old --by-wins behavior was identified as indistinguishable from a rubber stamp, and deliberately downgraded --by''s priority for that reason'
Regression-test: 'not applicable yet -- this record documents a finding to be resolved by design (an RFC amending RFC-2), not a code defect with an obvious fix'
---
# 16 — None of fix's identity-resolution tiers are a real attestation

## What was wrong

Re-examining `RFC-2`'s identity-resolution tiering (`gh api user` → `git config user.name` →
explicit `--by`) surfaced that none of the three tiers is actually a cryptographic attestation --
each is just "what does this local environment currently claim," with meaningfully different ease
of spoofing:

- `git config user.name` is unsigned local config -- trivially set to any value, exactly as
  spoofable as `--by` itself.
- `gh api user` proves *a* session is authenticated to *some* GitHub account, but not that the
  person invoking `urzua fix --apply` in this exact moment is the one who actually reviewed the
  repair being attested.
- `--by` is free text, acknowledged as such since `RFC-2` itself was written to fix exactly this.

The real, standard, already-existing attestation primitive this project already relies on
everywhere else is a **signed git commit** (`git commit -S`, verified against a registered public
key) -- proof of who committed what, that predates and doesn't need `urzua` to reinvent it. `fix
--apply`'s identity field is written to the file *before* that commit happens, so structurally it
can only ever be a provisional label, never itself a security control, regardless of which of the
three tiers wins.

## Why nothing caught it

`RFC-2`'s own tiering was a genuine, deliberate improvement over the literal predecessor bug
(unverified `--by` always winning) -- reordering the three tiers relative to each other was real
progress, so nothing about shipping it was wrong. What wasn't examined at the time is whether any
tier in the *new* ordering actually rises to "verified" in a security-meaningful sense, versus just
being harder-to-spoof-by-accident than the one below it. The question only surfaced now because a
semver-classification review asked "is this change actually safe/correct," which prompted rereading
`RFC-2`'s own stated rationale closely enough to notice it never claims cryptographic verification --
only that each tier is progressively less like free-form guessing.

## References

- RFC-2 -- the identity-resolution tiering this finding examines; not proposed for reversal, since
  its own reasoning (fixing the literal `--by`-always-wins predecessor bug) remains correct on its
  own terms.
- ADR-31 -- accepted RFC-2's tiering as shipped.
- A new RFC (not yet filed), `Amends: RFC-2` -- the planned vehicle for resolving this: either
  reframing `Realized-by`'s identity field explicitly as best-effort attribution rather than
  attestation, or exploring whether `fix --apply` should eventually check for a signed containing
  commit as a stronger tier.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not a code defect with an obvious fix -- disposition is a new RFC amending RFC-2, not yet drafted. | **structural** |
