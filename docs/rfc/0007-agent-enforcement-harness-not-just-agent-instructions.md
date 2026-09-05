# 0007 — Agent enforcement harness: an audit trail independent of the editing agent's own compliance

> Status: Draft
> Date: 2026-07-30
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

Every mechanism this repo has designed so far — RFC-0002's detect/apply split, RFC-0003's
agent-native JSON output, RFC-0005's Embodiment/`realized_by` model — assumes an agent *chooses*
to run the audit tool before editing a decision record, and that a PR's own stated scope reliably
describes its actual diff. Neither assumption is safe to rely on: an agent can edit an `embodiment`
field directly, based on its own possibly-wrong reading of prose, without ever consulting computed
evidence; a branch scoped as documentation-only can carry a real production behavior change that
nothing flags. Propose that Urzua ship a real enforcement *harness* — not one hook, but the
combination of a pre-write field-level check and a branch/PR-level scope-mismatch check, both
running automatically rather than depending on an agent's own instructions or discipline, which is
exactly where every comparable tool available today stops short.

## Motivation

Consider a representative scenario: an agent reads an ADR's `Embodiment: Verified` header field,
sees it contradicted by the ADR's own Context prose ("no table, service, or accessor exists"), and
edits the field to `Not started` — without ever running the audit tool, and without checking the
codebase for the `Implements:`/`Verifies:` comments that would show the field was already correct.
The tool that exists specifically to answer this question is one command away and simply isn't run.
Nothing catches this except a reviewer with independent domain knowledge happening to notice.

**This is not a one-off carelessness problem; it's a gap in the closest real-world precedent for
this failure mode.** A Cursor `.cursor/rules` MDC rule pairing AgDR with an always-applied
instruction — "you MUST create an ADR before modifying source code" — is best classified as **"a
prompt-injection/enforcement mechanism, not a governance system — no validation beyond the agent's
own compliance, no audit trail independent of the agent itself."** Claude Code ADR-capture skills
and Stop-Hook pipelines fit the same verdict for a different tool: **"single-agent, single-session
capture tools"** that solve "the agent forgot to document its own decision," never "an independent
check confirms the agent's claim is actually true." Every comparable mechanism is instruction-shaped,
not enforcement-shaped. This RFC proposes closing that gap.

## Proposal

### 1. Instructions are not enforcement — treat every "agent must X" as unenforced until proven otherwise

The premise this RFC starts from: an instruction living in a system prompt, an AGENTS.md-style
file, a Cursor rule, or a CLAUDE.md convention is a *request*, not a *guarantee* — an agent can
misread it, forget it under context pressure, or simply skip the step while reasoning from
something that feels like sufficient evidence (prose) when it isn't. Every convention that
currently lives only as an instruction — "run the audit tool before editing Embodiment," "verify
against real code before trusting an ADR's own claims" — should be treated as **unenforced** until
there's a mechanism that fires independent of whether the agent remembered to follow it.

### 2. A pre-write hook, not a bigger instruction

The fix is not "write a more emphatic instruction" — it's a **pre-write hook**: a mechanism that
runs automatically before an edit to a governed field is allowed to be treated as final,
independent of which agent (or human) made the edit and whether they remembered the convention.
Concretely, for an Embodiment-field edit specifically:

1. **Detect** (RFC-0002's mechanism) computes the real evidence-backed value for the record being
   edited.
2. If the edit's new stated value **disagrees with the computed value in the direction of claiming
   less than what's actually evidenced**, the hook surfaces the disagreement immediately, inline,
   before the edit is treated as committed — not silently overwritten, not deferred to the next
   scheduled audit run.
3. If the edit's new stated value **disagrees in the direction of claiming more than what's
   evidenced** (the more dangerous direction — a false `Verified` claim), the hook treats this the
   same way, surfaced immediately rather than waiting for a drift report days or weeks later.
4. The agent (or human) then has to *explicitly* acknowledge the disagreement to proceed —
   mirroring RFC-0002's own `--force`/`--ids` pattern (an explicit, named, logged override) rather
   than silently blocking forever or silently allowing the edit through.

This is deliberately **not** "run a full CI pipeline on every keystroke" — it's a narrow, fast check
(the same `detectRealizedByDrift`-shaped computation RFC-0002 already designed) scoped specifically
to the field being edited, cheap enough to run synchronously in an editing session.

### 3. Two real integration points exist today; this RFC targets both, not a hypothetical third

The *mechanism* this RFC needs already has two real, existing homes to live in — this RFC is about
wiring detection into them, not inventing a new enforcement substrate:

- **Editor/agent-runtime hooks** — Claude Code's own hook system (`PreToolUse`/`PostToolUse`,
  matched by tool name, able to block via exit code) is a direct, already-existing mechanism for
  exactly this: a `PreToolUse` hook matched on `Edit`/`Write` targeting a decision-record path
  could run detect and block or warn before the write lands. Cursor's `.cursor/rules` MDC
  mechanism is the closest existing cross-editor precedent, but it's instruction-shaped (a rule the
  agent is told to follow), not enforcement-shaped (a hook that runs regardless of whether the
  agent remembers). The gap this RFC targets is specifically closing that instruction-vs-enforcement
  distinction, not building a new hook mechanism from nothing.
- **Pre-commit / pre-push git hooks and CI** — the same detect computation, run as a blocking check
  (not a warn-only audit report) specifically on Embodiment-field diffs, catches the case an
  editor-level hook was bypassed, disabled, or simply not installed — a second, independent layer,
  matching a belt-and-suspenders posture (RFC-0002's `--force` requiring an identity even for an
  explicit override).

Both matter because they catch different failure modes: an editor hook catches the mistake before
it's ever written to disk; a git/CI-level check catches it if that first layer was skipped,
misconfigured, or run by a tool with no hook wired at all.

### 4. A second, distinct enforcement case: scope-mismatch detection between a branch's stated purpose and its real diff

Consider a second case, one layer up from a single field edit: a branch titled and scoped as a
documentation pass also carries a real production behavior change — a default parameter flipped
from unset to an explicit value across several call sites — bundled into the same PR as dozens of
doc-only diffs. Nothing in the repo's tooling flags the mismatch between the branch's stated scope
and its actual diff; a reviewer has to notice a small code change buried among many doc changes by
eye.

This is a different enforcement case from §§1-3 (which target a single field's stated-vs-computed
value) — it targets **branch/PR-level scope drift**: a governance-scoped branch's diff touching
paths outside what its own stated purpose would predict. Concretely: a pre-push/CI check that
classifies a diff's touched paths against a declared governance-path allowlist (e.g. `docs/adr/**`,
`docs/rfc/**`) and flags — not necessarily blocks outright, since a governance pass legitimately
*does* sometimes need a real code fix — when a diff touches non-test application source outside
that allowlist, surfacing "this needs elevated review beyond docs review" rather than silently
passing because the branch name and PR title said "docs."

Wired into CI only, not pre-commit — this check is inherently about a PR's *cumulative* diff
against its base branch, not a single commit, so a pre-commit equivalent would just repeat the same
warning on every commit in a long-lived branch. That asymmetry with §2's Embodiment check (which
fits pre-commit naturally, since each commit's own stated value is a real, checkable fact
regardless of what came before it) is worth carrying forward — not every enforcement check belongs
at every layer.

### 5. Mechanism-selection guidance: prefer the lighter-weight mechanism when it already covers the case

Consider a third case: an agent reaches for the heavier Realized-by + git-hash-locator mechanism (a
record-side pointer designed specifically for cases the comment-scan can't reach — YAML files,
comment-forbidden paths, infra-manual attestation) to back-point a decision record to a plain
source module, when that file already supports the lighter native mechanism (an `Implements:`/
`Verifies:` comment, picked up automatically by the existing code scan) just as well. This is the
same enforcement gap as §§1-4, one level down: not "is the stated value correct" but "was the right
mechanism used to state it at all."

Concretely:
- A pre-write check (same shape as §2's) that flags a Realized-by-locator addition on a file type
  the code-scan already covers as a probable mechanism-selection mistake — a nudge, not a hard
  block, since the two mechanisms are not interchangeable in general (Realized-by is the only option
  for genuine comment-scan gaps).
- The nudge should name the specific reason inline: "this file type already supports `Implements:`/
  `Verifies:` comments, scanned automatically — add the comment there instead of a Realized-by
  locator, unless this is a genuine comment-scan gap (non-code config, a comment-forbidden path)."

### 6. Fail loud, not silent: an unhandled edge case in an audit/lint tool is a missing test, not a design choice

A governance-tooling function with a boolean/enum/optional branch should have an explicit test for
every branch, including the "this should never happen but the type system allows it" ones — and
the default for an unhandled or null case should be to fail loud (throw, or emit a blocking error)
rather than silently returning an empty/neutral result. A tool whose entire purpose is catching
missing or stale state defeats itself if its own edge cases default to "nothing to report." An
absent required document silently producing zero freshness errors, or a title field with no
matching pattern silently becoming an empty string that propagates downstream, are the shape of bug
this guards against.

### 7. Duplicated logic across near-identical tooling functions is itself an unenforced drift risk

The same filter predicate (e.g. "what counts as a real document in this directory, vs. an
index/template/README file") independently restated as an inline literal in several different
functions is the same class of problem as everything else in this RFC — a fact stated once now has
as many places to silently drift as it has copies, and nothing in the tooling itself would catch
it. A lint rule flagging a repeated non-trivial boolean expression appearing more than once is a
plausible mechanical backstop, but the more durable fix is upstream of tooling: when adding a
second copy of logic that already exists once, default to extracting a shared function immediately,
not after a third or later copy accumulates.

## Open questions

- What's the actual UX when a pre-write hook fires mid-edit for an agent (not a human)? Blocking
  the tool call outright (Claude Code's `PreToolUse` supports this) is the strongest guarantee but
  risks false-positive friction on legitimate edits; a warning surfaced back to the agent that it
  must explicitly acknowledge is weaker but less disruptive. Undesigned here.
- Does this generalize beyond Embodiment specifically — should any field with a computable ground
  truth (a `status` transition that violates a profile's declared legal-transition rules, per
  RFC-0001 §1a) get the same pre-write hook treatment? Plausible, not scoped here.
- How does this interact with RFC-0006's append-only revision log — does a hook-blocked edit ever
  get recorded as an attempted-and-rejected revision entry, or does it simply never happen? Leaning
  toward "never happens, no record" (matching how a failed compile leaves no artifact), but not
  fully reasoned through against RFC-0006's own reconstructability claim.
- §4's allowlist-classification idea needs a real design pass: what counts as "governance path,"
  how false positives (a legitimate mechanical tooling fix) are distinguished from a real scope
  mismatch, and whether this lives as a distinct check or as a variant of the same detect
  computation §§1-3 already designed.
- **Does the harness need an explicit, human-declared override for skipping a workflow stage
  entirely** (not just an absence of enforcement on a stage that happens)? Every other mechanism
  this RFC proposes follows "block by default, but allow an explicit, named, logged override" —
  applying that shape to workflow-*order* (a declared "Skip-RFC: <reason>, by=<name>") is the
  natural next design to reach for, but it doesn't obviously fit the way the other overrides do:
  every existing override sits on top of a mechanism with a computable ground truth to disagree
  with (a hash mismatch, a drifted locator); a skip-declaration would record a human's judgment
  call, not override a computed fact. Whether that's the right shape, and whether it belongs at the
  harness layer at all versus staying a pure convention, is undesigned.
- **Is there any computable proxy for "this document type was rightly skipped," or is it
  irreducibly a judgment call?** Candidate proxies worth testing: diff size/blast-radius of the
  change that shipped without the earlier document; whether any *existing* ADR/spec's stated scope
  would have needed amending (a real, computable check — this RFC's own Realized-by and
  Supersedes/Amends machinery could plausibly detect "should have referenced ADR-NNNN and didn't");
  presence or absence of a rejected alternative in the shipped artifact's own reasoning. None of
  these are designed to completion here.

## Non-goals

- Building a new, Urzua-specific hook/rule mechanism from scratch — this RFC wires existing
  editor-hook and CI mechanisms to Urzua's own detect computation, it does not invent a new
  enforcement substrate.
- Solving the multi-agent concurrent-edit/field-level-merge gap — a real, adjacent, and larger
  problem, out of scope here.
- Runtime agent-behavior governance in the broader sense (sandboxing, kill switches, policy
  enforcement across arbitrary agent actions) — a genuinely different problem from decision-record
  governance specifically; this RFC stays scoped to decision-record field edits.

## References

- Cursor's `.cursor/rules` + AgDR pairing ("a prompt-injection/enforcement mechanism, not a
  governance system") and Claude Code ADR-capture skills/Stop-Hook pipelines ("single-agent,
  single-session capture tools") — the precedent this RFC's whole premise (instructions ≠
  enforcement) is built on. No surveyed tool combines agent-authorship-awareness, semantic
  drift-checking, and lifecycle/role-modeling; this RFC's hook proposal is one piece of closing that
  combination, specifically the enforcement-timing piece.
- Terraform Sentinel tiering, Backstage ownership routing, PagerDuty escalation, and MADR's
  `Confirmation` field — prior art feeding §5's mechanism-selection framing and the
  routing/escalation ideas referenced in the open questions above.
- RFC-0002 — the detect/apply mechanism this RFC's hook reuses rather than reinventing.
- RFC-0003 — agent-native JSON output; the hook's own findings should follow the same
  stdout/status convention when surfaced back to an agent.
- RFC-0005 — the Embodiment/`realized_by` model whose fields this RFC's first concrete
  application targets.
