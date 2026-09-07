# 38 — Detect staleness when a code comment's cited record is amended

> Status: Planned
> Stable-Id: 01M1YKX7MDS9A2HF5GSEZ5FTWK
> Phase: 1
> Track: embodiment-model

## What

A mechanism for the reverse direction of the existing claim graph: today `Realized-by: code:path`
lets a record cite a file, and drift detection (ADR-32) flags when that *file* changes after the
citation was last touched. Nothing today detects the opposite -- a code comment that cites a record
by number (e.g. `# ... (ADR-29)`, a pattern already used pervasively in this repo's own workflows
and config) going stale because that *record* was later amended. Needs a design decision first
(likely an RFC, given it's a new schema-level direction on the claim graph, not a config tweak) on
what "amended" means for staleness purposes and how a comment declares which record revision it
assumed.

## Why

Raised directly while discussing an upcoming change to `prepare-release.yml`'s own `(ADR-29)`
citation: amending ADR-29 to add an auto-opened release PR (MILE-39) will leave that exact citation
carrying an assumption from before the amendment, with nothing to catch it. Building this first
means MILE-39's own ADR-29 amendment becomes the first real dogfood test of whether the detector
actually fires on real content, rather than a synthetic fixture.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
