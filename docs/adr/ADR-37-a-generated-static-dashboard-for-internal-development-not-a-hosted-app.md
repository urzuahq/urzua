# 37 — A generated static dashboard for internal development, not a hosted app

> Status: Accepted
> Stable-Id: 01M1YTZ48KK60554GSFTN46E5F
> Embodiment: Verified
> Realized-by: code:scripts/generate-dashboard.py
> Date: 2026-09-07
> Author: beauwilliams
> Deciders: beauwilliams
> Supersedes / Superseded-by: —
> Derives-from: —

## Context

While reviewing the milestone/RFC/ADR/spec backlog this session, a viewer was iterated on
repeatedly to make the corpus easy to browse -- filter by phase/track/status, click into a
record's full content, see live `check`/`explain`/`graph` output. That viewer lived only as a
throwaway script in a session-scratch directory: not committed, not versioned, not reproducible by
anyone else, and one session-end away from being lost entirely -- the same "written once, never
durable" failure this project's own tooling exists to catch in decision records, now happening to
real code.

Now that it's being committed for real, the shape of it needs deciding, not just accepting by
default. The obvious wrong move is treating "we want a dashboard" as license to start building
RFC-14's forge app (a real collector/actor service) or MILE-30's build -- neither is decided to be
built yet, and this viewer doesn't need either: it's a read-only convenience for whoever is working
on this repo, not a product surface.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A generated static HTML file, one Python script, run on demand | No server, no build step, no new dependency, deletable with zero blast radius, matches "generated view, not a second maintained source" already stated in this repo's own README | Not live/hosted; has to be re-run to see new data |
| A small local web server (Flask/similar) serving the corpus live | Live updates without re-running | A running process to manage, a port to bind, a dependency to pin -- real infrastructure for what is still just a read-only convenience |
| Build toward RFC-14's forge app now | Would eventually be the real thing | RFC-14 is explicitly decided as "shape decided, build explicitly not next" (ADR-25) -- building it now because a viewer script would be convenient inverts that decision by accident, not on purpose |

## Decision

In the context of committing real, previously-uncommitted viewer code, facing the choice between a
static generator and building toward the eventual real app surface, we decided: **a single
self-contained Python script (`scripts/generate-dashboard.py`) that reads the corpus and the real
`urzua` binary's output and writes one static HTML file with the data embedded** -- no server, no
framework, no build step, nothing this project isn't already at a stage to justify.

- Data comes from two places, both already real: parsing `docs/*/` directly (the same shape `check`
  itself parses), and shelling out to the built `urzua` binary for `check`/`explain`/`graph` --
  never a third, hand-maintained data source.
- Output is one HTML file with the data as an embedded JSON blob and vanilla JS rendering it --
  filterable, clickable into each record's full body, URL-state-synced so a filtered view is a
  shareable link. No framework, no build tooling, no runtime dependency beyond a browser.
- Lives in `scripts/`, not `ts/` or `platform/` -- both of those are reserved for actual product
  surfaces (agent-harness integrations, deployment targets); this is maintainer tooling, explicitly
  not a shipped feature.
- Explicitly not this: a hosted service, a live-updating view, or a step toward RFC-14's forge app.
  If a real case for those emerges, that's its own decision, not an assumed consequence of this one.

## Reversibility

Fully reversible, cheaply: delete `scripts/generate-dashboard.py` and nothing else in the product
is affected. No schema change, no `urzua-core` change, no CI dependency. The riskiest part is
letting maintainer tooling quietly grow into a second product without a decision -- named explicitly
above as out of scope, not left to happen by accretion.

## Consequences

- Anyone working on this repo can regenerate an up-to-date corpus view with one command
  (`python3 scripts/generate-dashboard.py --output dashboard.html`) instead of reading `docs/`
  file-by-file.
- The view is only as current as the last time it was generated -- there is no watch mode, no
  auto-refresh. Acceptable for a dev convenience; would not be for anything sold as live.
- If this ever needs to be live, shared, or multi-user, that is a new decision (likely touching
  RFC-14), not a natural extension of this one.

## References

- ADR-25 -- the forge app's "shape decided, build explicitly not next" stance, which this decision
  deliberately doesn't disturb.
- RFC-14 -- the collector/actor app this is not a step toward.
- README's "generated view, not a second maintained source" principle, already stated for this
  repo's own docs, applied here to a viewer instead of a document.
