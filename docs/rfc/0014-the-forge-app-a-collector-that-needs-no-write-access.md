# 0014 — The forge app: a collector and an actor that needs no write access to code

> Status: Draft
> Date: 2026-08-18
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

RFC-0003 §5 established that governance history has to be collected from tool runs rather than
stored in the corpus, because a protected default branch with a read-only CI token is the normal
configuration. Collection needs somewhere to collect *to*, and several other features on the roadmap
— escalation, the cross-repo graph, verified attestation — need to observe or act on a forge rather
than a working tree. Propose a **forge app** as the mechanism, with one property doing most of the
work: it requires **no write access to repository contents**. It reads records, receives events, and
writes only its own outputs — checks, comments, review requests. Propose also the boundaries that
keep it from becoming the product: the CLI stays complete without it, the app is one implementation
of a forge port rather than the architecture, and it is explicitly not next.

## Motivation

Three things converge on the same missing piece.

**Collection has no destination.** RFC-0003 §5 concluded that the tool emits an event per run and
something else keeps the series, because a repository cannot maintain a history of itself under
normal branch protection. Nothing currently receives those events.

**Several roadmap features are forge-shaped, not tree-shaped.** Escalation has to
reach a person, which means knowing who reviews what and being able to request them. The cross-repo
decision graph (Phase 4) has to see many repositories at once, which no per-repo CI job can.
Attestation identity (RFC-0002 §2) resolves best against an authenticated forge account. Each of
these has been deferred partly for want of a component that observes an organization rather than a
checkout.

**The alternative mechanisms have all been examined and rejected on their own terms**: an in-repo log
needs a write exception (RFC-0003 §5), CI artifacts expire, and a per-run POST to an unspecified
endpoint is the same hosting question without the design.

What makes this worth proposing now rather than later is that the *shape* is unusually constrained
by what has already been decided, and getting the shape written down cheaply prevents a much more
expensive mistake — building a service that owns the corpus.

## Proposal

### 1. It needs no write access to code, and that is the design

Permissions: repository **contents read-only**, metadata read, plus write on the app's own surfaces
— check runs, pull-request comments, review requests. Webhook subscriptions: pull request, check
suite, push.

There is deliberately no `contents: write`. Every feature this enables works without one:

| Feature | Mechanism | Needs contents write |
|---|---|---|
| Collect governance events | webhook + the run's own output | no |
| Publish findings | check run on the commit | no |
| Escalate to a decider | review request or comment | no |
| Cross-repo graph | read many repos' records | no |
| Attestation identity | authenticated account behind the action | no |

This matters more than it sounds. An install that asks only to read code and write its own comments
is a conversation with a platform team that ends in yes. One that asks to commit to the default
branch is a conversation about branch protection, and it is the same conversation whether the write
is a log line or a code change.

### 2. The CLI remains complete without it

Everything the CLI does today must keep working with no app installed, no network, and no account:
validation, the audit, drift detection, per-record drift age. The app adds **history, reach across
repositories, and the ability to act on a forge** — never correctness.

Concretely: no check may become unrunnable offline, and no finding may exist only in the app. If a
result can only be obtained by installing something, that result has left the tool and become a
service feature, and the offline user has silently lost a capability they had.

### 3. It is one implementation of a port, not the architecture

The app is a **forge adapter** behind an interface the rest of the system holds: *observe events*,
*publish a finding*, *request a human*, *enumerate repositories*. GitHub is the first implementation
because it is where the evidence base lives; the interface is what keeps cross-vendor neutrality
answerable rather than aspirational.

The test for whether this boundary is real: naming a specific forge anywhere outside the adapter is
a defect.

### 4. What it stores, and the trust boundary

It stores **events and derived summaries**, not corpora. A decision record's text stays in the
customer's repository; the app keeps what happened and when — drift caught on this pull request,
resolved at this merge, this claim unverified for this long.

This is the line the roadmap's hosted-service non-goal is really about. "We hold your decision
records" and "we hold a log of your governance events" are different products with different risk,
different data-retention obligations, and different answers to a security review. The second is
defensible on a boundary that can be stated in one sentence; the first needs a much longer
conversation.

Reconstruction (Q18) sits awkwardly across this line and is deliberately left open here: replaying
history requires reading the corpus at many commits, which is within read-only permissions but is a
much larger read than steady-state operation.

### 5. Sequencing: this is not next, and saying so is part of the proposal

`urzua check` does not yet run against a corpus. Building a forge app before the
CLI validates records would invert the dependency — the app's entire value is distributing and
retaining results the CLI has to produce first.

The purpose of writing this now is to capture a shape while the constraint that forced it is fresh,
and to prevent the drift toward a service that owns the corpus. It should be revisited when the CLI
is real and something concrete asks for history, escalation, or cross-repo reads — not before.

## Open questions

- **Does the app need to be hosted by us at all?** A self-hostable receiver keeps the data boundary
  entirely inside the customer and removes the security conversation, at the cost of the install
  being an infrastructure project rather than a button.
- **What is the retention policy for collected events**, and does a customer expect deletion on
  uninstall to mean deletion of history they may want back?
- **How does the app authenticate a decider** in a way that improves on RFC-0002's tiering? An
  installation token proves the app acted, not that a particular human approved.
- **Does an app change the escalation answer to Q1?** Q1 is unanswered because nobody has said what
  teams do today. Having a mechanism does not supply that evidence, and building escalation because
  the mechanism now exists would be exactly backwards.
- **Where does a check run's output stop being enough?** Findings publish cleanly as check runs, but
  a cross-repo graph has no natural forge surface and implies a UI, which is Phase 5 at the earliest.

## Non-goals

- **Writing to repository contents.** §1 — the whole install rests on not asking.
- **Holding customers' decision records.** §4 — events and summaries only.
- **Making the CLI depend on it.** §2.
- **Forge-specific behaviour outside the adapter.** §3.
- **Building it now.** §5.

## References

- RFC-0003 §5 — collection rather than in-repo storage, and why a protected branch forces it.
- RFC-0002 §2 — attestation identity tiering, which an authenticated account improves on.
