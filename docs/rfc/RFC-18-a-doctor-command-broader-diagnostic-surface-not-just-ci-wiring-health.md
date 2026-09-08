---
Stable-Id: 01M20SH7KH9GXM31VZ5F5ZMFEV
Status: Draft
Date: 2026-09-08
Author: '@beauwilliams'
---
# 18 — A doctor command: broader diagnostic surface, not just CI-wiring health

## Summary

`urzua doctor` ([SPEC-15](../specs/SPEC-15-urzua-doctor.md)) checks six narrow things today, all
about whether the tool is *wired up* correctly (config exists, parses, CI mentions `check`). Propose
widening its job to match what a "doctor" command means in other CLIs (`brew doctor`, `flutter
doctor`, `rustup doctor`): a first stop for "is this install healthy," covering environment
preconditions and adopter onboarding, not just this repo's own CI-invocation sanity.

## Motivation

Found live, twice, in the same conversation: (1) `doctor`'s current framing reads as an internal
safety net for this repo's own CI, not something an adopter would reach for after `init` — despite
the README's whole pitch being "adopt without a rewrite." (2) SPEC-15 already names three real gaps
as aspirational-not-built (which config resolved and from where, which rules are off, actual
invocation context) — evidence the current scope was never claimed to be the ceiling, just what
shipped first.

Two more gaps surfaced independently this session, neither named in SPEC-15 at all:

- **Template/config agreement** (already tracked as [MILE-6](../milestones/MILE-6-add-a-doctor-check-for-template-config-section-agreement.md),
  Planned): a type declaring a non-`yaml-frontmatter` `header_shape` with no matching
  `.urzua/templates/<type>.md` fails `urzua new <type>` outright with exit 2 — exactly what `spec`
  did before [MILE-74](../milestones/MILE-74-add-a-spec-template-so-urzua-new-spec-works.md) fixed
  it. Nothing catches this *before* a user hits the failure.
- **Silent degradation of git-blame-dependent features.** `embodiment.consistency`'s drift detection
  (ADR-32) depends on real git history. In a shallow clone or an environment with mostly-uncommitted
  changes, it doesn't error — it just finds fewer drift findings than it should, indistinguishable
  from "nothing drifted." Observed directly this session: the same corpus's `embodiment.consistency`
  count dropped from 22 to 3 findings purely because ~170 files were uncommitted at the time, with no
  warning that the numbers were unreliable.

## Proposal

Widen `doctor`'s checks along two new axes, in addition to what SPEC-15 already ships:

1. **Onboarding framing.** Position `doctor` as the command `init` itself suggests running next —
   its report becomes "what's configured, what's missing, what to do about it," not only a CI gate.
2. **Template/config agreement** (folds in MILE-6): for every configured type with `header_shape !=
   "yaml-frontmatter"`, does `.urzua/templates/<type>.md` exist? `warn` if not — `urzua new <type>`
   will otherwise fail loud only when someone actually tries it. **This is narrower than MILE-6's
   full scope**: MILE-6's own title names template/config *section* agreement — does the template
   actually contain a heading for every `required_fields`/section the config declares, not just
   whether the file exists at all. A present-but-incomplete template passes the existence check
   above and still violates that fuller contract. Either this proposal's first cut is existence-only
   (leaving section-completeness for a later pass), or MILE-6 itself should be read as two separate
   checks — worth resolving explicitly rather than letting "folds in MILE-6" overstate what's
   actually being proposed here.
3. **Environment preconditions**, not just config preconditions:
   - Is `gh` authenticated? `resolve_identity()` silently falls through to `git config user.name`
     otherwise — worth surfacing before a write attributes to the wrong identity source.
   - Does the repo have real git history (not a shallow clone, not a fresh `git init` with one
     commit)? If not, `warn` that blame-dependent checks (`embodiment.consistency`,
     `blocked-on.stale`) will under-report, rather than let the silence read as "clean."
4. **The three SPEC-15-named gaps**, now in scope rather than deferred indefinitely: resolved config
   path (including a `--config` override), which rules are actually off (blocked on
   [MILE-80](../milestones/MILE-80-configurable-rule-severity-per-spec-1-s-own-unbuilt-promise.md)'s
   severity config existing at all), and real invocation-context detection beyond grepping the
   workflow file's text.

## Open questions

- **Where's the boundary with `check`?** `check` validates records; this proposal keeps `doctor`
  strictly to tool/environment health, never record content — but "does every type have a template"
  is arguably about the corpus's *configuration*, closer to `check`'s territory than `doctor`'s
  original "is the binary wired up" framing. Worth stating precisely before scope creeps further.
- **Exit-code semantics for an onboarding-facing report.** Today a `warn`-only report exits `0` —
  fine for a CI gate nobody wants to fail on advisory notices. Does that still hold if `doctor`
  becomes the primary first-run experience, where a user might expect a non-zero exit to mean
  "you're not fully set up yet"?
- **Does git-history-depth detection have a reliable, cheap check?** `git rev-list --count HEAD` is
  cheap but a low count doesn't distinguish "shallow clone" from "genuinely new repo" — worth getting
  right before shipping a check that cries wolf on every brand-new adopter.
- **Should MILE-6 be absorbed into this RFC's implementation, or stay a separate, narrower unit of
  work?** It's already `Planned` independently; this RFC could just be the "why" that makes it worth
  picking up, without claiming ownership of it.

## Non-goals

- **Record content validation** — stays `check`'s job entirely, no matter how broad `doctor` gets.
- **Automatic repair** — stays `fix`'s job; `doctor` only ever reports, per every command's existing
  read-only-by-default posture (ADR-19).
- **Deciding MILE-80's severity-config shape** — referenced as a blocker for one specific check
  above, not reopened here.

## References

- SPEC-15 — `doctor`'s current, shipped scope; the three gaps it already names as aspirational.
- MILE-6 — the template/config agreement check this proposal folds in.
- MILE-80 — configurable rule severity, blocking the "which rules are off" check.
- BUG-4 — `doctor`'s own bug history (plain-text output), evidence it's a real feature area with
  real defects, not just a detail of configuration.
- ADR-32 — git-blame-based drift detection, the mechanism whose precondition this RFC proposes
  `doctor` should check for.
- RFC-20 — how `doctor` relates to `check`/`fix`/the rest of the command surface, the boundary
  question this RFC's first open question depends on.
