# 0002 — The re-verification/refresh workflow: detect is read-only, apply requires review or an explicit bypass

> Status: Draft
> Date: 2026-07-30
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

Design the re-verification/refresh mechanism RFC-0005 §2 explicitly deferred as a non-goal: what
actually happens when a `realized_by` locator's content changes since it was last verified. Propose
splitting **detect** (read-only, structured JSON output, safe to run anytime) from **apply**
(mutating, requires either a reviewed, per-claim ID list or an explicit `--force` bypass), and
resolving the attestor identity behind an apply from a verified source (an authenticated GitHub
login) rather than accepting free-text as the default.

## Motivation

Consider a representative failure: a re-verification tool
(`audit --refresh-hashes --by "<name>"`) mutates a decision record's `Realized by:` hash
the moment it's invoked, on the strength of the operator's own claim to have looked at the diff —
with no tool-enforced separation between "I re-hashed this because I re-verified the claim still
holds" and "I re-hashed this because I know I'm the one who touched the file." Those are not the
same act, and a tool that can't tell them apart can't actually deliver the accountability its own
`--by` flag is trying to establish. The `--by` value itself was also plain free text — nothing
stopped it from being wrong, unattributable to a real account, or simply absent under
`--no-attestation`.

This is precisely the "re-verification trigger mechanism" and "staleness-cadence policy"
RFC-0005 §2 named as deferred, non-goal follow-up work once the record-side pointer model
(RFC-0005 §2) and claim-graph shape (RFC-0005 §3) existed to build it against. Both now exist.
This RFC is that follow-up.

## Proposal

### 1. Detect and apply are two different operations, not one flag

**Detect** (`refresh` with no mutating flags): walks every record's `realized_by` claim graph,
re-hashes every `code`-type leaf locator, and diffs against the recorded hash. Writes nothing.
Output is a JSON array on stdout — not prose — one entry per drifted leaf:

```json
[
  {
    "id": "adr-0069/realized_by/0",
    "record": "ADR-0069",
    "locator": "docs/rfc/README.md",
    "recordedHash": "4a9d01f0...",
    "currentHash": "8f9c1d66...",
    "graceExpiresAt": "2026-08-27",
    "pastGrace": false
  }
]
```

This is deliberately agent-native, not human-report-native: a structured list is what a reviewing
agent (or a human scripting against it) can act on without re-parsing prose. A human-readable
summary line count (`N locators drifted, M past grace`) prints alongside it on stderr, but the JSON
on stdout is the actual contract.

**Apply** (`refresh --ids <id,id,...> --by <identity>`): re-hashes and rewrites the record's
`Realized by:`/`Last audited:` fields, but **only for the listed IDs** — every ID must come from a
prior `detect` run's output (the tool doesn't accept arbitrary IDs; a real detect pass has to have
produced them). This is the point where a reviewer has actually opened the record and the file,
compared the two, and concluded the claim still holds — the fuzzy, non-mechanical judgment call
§3a's tier 4 already scoped to "the agent visits the locator and makes the actual judgment call the
mechanical tiers were never claiming to make on their own." The tool cannot verify that judgment
happened; it can only make skipping it require a separate, explicit, named action instead of being
the default path.

**Force apply** (`refresh --force --by <identity>`): applies every currently-drifted claim without
per-ID review — the explicit "I am choosing to skip individual review for this batch" bypass.
Never the default; always requires the same identity resolution as scoped apply.

### 2. Attestor identity resolves from a verified source, not free text

`--by`'s value currently accepts anything typed on the command line — indistinguishable from a
truthful attestation to the tool. Revised resolution order:

1. **`gh api user`'s authenticated login**, when the `gh` CLI is installed and authenticated —
   cannot be locally spoofed the way a string literal can; it's tied to whatever credential the
   invoking session actually holds.
2. **`git config user.name`/`user.email`**, when `gh` isn't available/authenticated — still
   self-reported and spoofable (`git config` accepts any string), but at least sourced from the
   operator's own configured identity rather than typed fresh per invocation, and consistent with
   how git itself already attributes commits.
3. **Explicit `--by "<name>"` override**, for either case, or for an operator with neither
   configured — the honest fallback, not silently disallowed, but no longer the *default* path.

None of these tiers make the attestation cryptographically provable — a determined operator can
still lie at any tier, including tier 1 (using someone else's authenticated session). That's out of
scope here, same as RFC-0005 leaves promotion-trigger mechanics and `NOT`-operator semantics
explicitly open rather than false-solving them. The goal is raising the cost of an accidental or
careless rubber-stamp, not achieving unforgeable attestation.

### 3. Why detect/apply, not a single confirm-prompt flag

An interactive `[y/N]` per-drifted-claim prompt was considered and rejected: it doesn't compose
with CI, doesn't produce a durable artifact a second reviewer (or an agent, per this repo's own
audience) can look at independently of the terminal session where it ran, and conflates "I am
choosing to review right now" with "review happened" — a fast-fingered `y` is exactly the rubber-
stamp failure mode this RFC exists to prevent, just moved into a different UI. Separating detect's
output as a standing artifact means a review can happen asynchronously, by someone other than
whoever ran detect, using whatever tooling they prefer.

## Open questions

- Should `detect`'s JSON output be written to a file (e.g. `docs/adr-audit/drift.json`, mirroring
  the existing `summary.json` convention) in addition to stdout, so a review can reference a stable
  path rather than re-running detect to reproduce it? Leaning yes, undesigned here.
- Does a scoped `apply --ids` run need to re-verify each ID still shows as drifted at apply time (in
  case the underlying file changed again between detect and apply), or is a stale ID simply an error?
  Leaning toward re-verify-and-error-on-mismatch, undesigned here.
- `gh api user` requires a network call and an authenticated session; what's the right timeout/failure
  behavior when it hangs rather than cleanly failing (fall through to tier 2 immediately, or wait)?
  Undesigned.

## Non-goals

- Cryptographically provable attestation (signed commits, hardware keys) — raising the cost of a
  careless mistake, not defeating a determined adversary.
- The claim-graph promotion/sharing mechanics from RFC-0005 §3 — this RFC only touches the
  detect/apply mechanism for a single record's own claims, not cross-record shared-node behavior.
- Automatic (unattended, cron-triggered) apply — every apply path here requires a named identity by
  design; a fully automated re-verification loop is a different, not-yet-proposed feature.

## References

- RFC-0005 §2 (tier 4: "Locator changed since last verification... triggers a periodic,
  LLM-assisted re-verification pass") and its explicit non-goal naming this exact follow-up.
- SOC 2/CODEOWNERS attestation-cadence precedent informing the identity-resolution tiering here.
- The motivating scenario: a `--refresh-hashes --by "<name>"` invocation
  where the attestation was accurate in substance but the tool had no way
  to distinguish that from a rubber stamp, and `--by` accepted unverified free text.
