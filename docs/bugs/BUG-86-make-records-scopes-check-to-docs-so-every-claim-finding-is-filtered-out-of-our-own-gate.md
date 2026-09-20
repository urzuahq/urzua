---
Stable-Id: 01M2YQ9308YAYTQT4S7FWMY295
Status: Fixed
Found-in: "Consolidating the release changesets, when a deliberately-planted false claim did not fail the gate"
Regression-test: "not yet written -- a false claim in a claim_paths file must fail `make records`"
---
# 86 — make records scopes check to docs so every claim finding is filtered out of our own gate

## What was wrong

`make records` runs `urzua check docs/`. `claim.status-agreement`'s findings name the claim file,
which lives in `.changeset/`, outside that scope -- so `BUG-67`'s scoping, which is correct, filtered
every one of them out of this repository's own gate.

Demonstrated by planting a false claim. With `BUG-38` set to `Open` while the release changeset said
it was fixed:

```text
urzua check         -> claims to close BUG-38, but BUG-38 has Status Open
urzua check docs/   -> no findings
```

The rule works. The invocation could not see it. `claim.status-agreement` was written after a
changeset in this repository announced it had closed `BUG-36` when it had not, and the gate meant to
stop that recurrence has never been able to fail on it.

The same blindness applies to any rule whose findings name a file outside `docs/`.

## What it caught immediately

Run unscoped, the rule rejected the consolidated changeset on its first execution: it claimed to
close `BUG-40`, which is `Open`, and `BUG-47`, which is `Rejected`. Both were removed. The claim was
written by hand from a list of record numbers, which is exactly the error the rule exists to catch.

## Why nothing caught it

The scope was introduced for a different reason and the gate was never re-derived from it. `docs/` is
the right scope for record rules and wrong for every rule that reports on a file elsewhere, and
nothing enumerated which rules those are.

`records_examined: 1` was visible throughout -- the rule was reading the claim file on every run and
reaching a verdict that was then discarded. The count was honest; the finding never arrived.

## References

- `BUG-67`, whose scoping this interacts with -- correct in itself.
- `BUG-36`, the incident `claim.status-agreement` was written for.
- `ADR-55`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
