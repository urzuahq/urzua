---
Stable-Id: 01M7KQY7SR8Z081MQBXH3JQ8TN
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 331 — A declared question about a record is answered by an external judge and kept as a claim

## What

Let a configuration declare a question about a record's meaning, have a separate command put it to an external judge the configuration names, and keep the answer in the record's evidence as a claim that goes stale when its input changes. Advisory, off by default, first in shadow.

## Why

The engine verifies a record's structure and relationships: fields exist, a state is allowed, a reference names a real record of the right type. It cannot verify that the prose supports what the structure claims. On 2026-11-17 an agent in an adopting repository was about to move a record to its finished state; two of that record's three acceptance checks asked for runs that had not been done; `urzua check` would have passed, because the state was allowed and the evidence section existed. In the agent's words: "We have the record that's verifiable, but is what we're verifying even making sense?" Every header field is a claim the body should support: a state, a reference to a parent, a declared dependency, a field's value. Hosted decision models answer a fixed yes-or-no, choice or score question about a supplied text with a probability, cheaply and in about a second. Three were examined from their own documentation on 2026-11-17. All three accept one common request shape (a state, and named questions each with a type and instructions) and return a probability per question, the resolved model version and token usage. They differ in input limit (32 thousand to 384 thousand tokens), questions per call, whether an abstention is returned, and data retention (one deletes within 60 days with zero retention on request, one offers zero retention to enterprise customers, one states nothing). None offers a seed; one vendor's own measurements show the same request varying between calls, and the adopting agent saw 0.71 and 0.85 for one prompt. So an answer is not repeatable and cannot be a gate. The mission (`ADR-101`) says the tool never judges the change; this proposal has it record another party's judgement, with what was asked and of what, and that boundary needs a decision record before any build.

## Done means

- A decision record states how this fits the mission in `ADR-101`: the engine never forms a judgement of its own; it puts a declared question to a declared external judge and records the answer as a claim. The record fixes that an answer never changes an exit code, and it is accepted by the maintainer before anything else here is built.
- A configuration can declare a question: its text, its type (yes-or-no first), the event it attaches to (a move into a state, or a field), which parts of the record and of the records it refers to form the input, and the judge to ask. The engine contains no question of its own, shown by a test with a fixture configuration.
- A judge is declared in configuration behind one interface that carries the question, the input and the options, and returns the probability, the resolved model identifier, usage and an optional abstention. No vendor's name, address or request format is in the engine: the interface is satisfied by a declared external command, and adapters for hosted services live outside the core. A credential is read from an environment variable the configuration names and never from the configuration or a record.
- Asking is a separate command that is never run by `urzua check`; `check` makes no network call and stays repeatable, shown by a test that `check` passes with no network and no credential.
- The answer is written into the record as a claim holding the question, a hash of the exact input sent, the judge and resolved model version, the probability, the date and the commit. While the input hash is unchanged the stored answer is reused and the judge is not asked again; when the input changes the claim is reported stale by the existing staleness machinery, shown by a test of each.
- Nothing leaves the machine unless a configuration declares a judge and a question, shown by a test. A record or a section can be marked as never sent, and the command's dry run prints exactly what would be sent and to which judge without sending it.
- The first delivery is shadow mode for one question, whether the evidence for a move into a success state shows each acceptance item was met: the answer is recorded and nothing is reported. Reporting it as an info-level finding is a later configuration switch, never above info.
- The reference page states for a reader deciding whether to turn this on: answers are not repeatable; text is sent to a third party; and what each declared judge's retention is remains the adopter's to check.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Proposed on 2026-11-17 by an agent in an adopting repository at the maintainer's request, who asked for a milestone and for the hosted decision models to be examined; the case that prompted it is quoted in Why. | **structural** |
