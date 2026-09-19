---
Stable-Id: 01M2WDKB3Y7SCHKRWPN87BXVZ6
Status: Open
Found-in: "Five bug records shipped as Status: Open in the same branch that fixed them, with a changeset naming all five; claim.status-agreement ran over the changeset and reported nothing"
Regression-test: "not yet written -- a changeset naming a record that is still open must not report a clean run"
Blocked-on: RFC-36
---
# 66 — A changeset that names a record without a closing verb is invisible to claim.status-agreement

## What was wrong

`claim.status-agreement` exists to stop a changeset announcing a record closed while the record says
otherwise -- the defect `BUG-36` shipped. It recognizes a claim only when a reference follows a
closing verb.

A changeset that names the record without one is invisible to it. The round-6 changeset reads

```text
`check`'s path argument narrows what is reported on, not what a pointer resolves against (BUG-60).
```

for each of five bugs, all of which were still `Status: Open`. The rule read the changeset, examined
2 records, and reported no findings.

The inverse of `BUG-36`: that was a claim to close an open record, caught. This is the same
disagreement, stated in a form the rule does not parse, and reported as agreement.

## Why nothing caught it

The rule's tests supply changesets written with a closing verb, because that is the shape the rule
recognizes -- so they exercise the parser's successes and never its silence. `records_examined: 2` was
visible in every run on this corpus and read as "there are two changesets" rather than "the claims in
them were not found".

Deciding what counts as a claim is the open question, not an oversight to patch: a changeset may
legitimately name a record it does not close, so treating a bare mention as a claim trades this
silence for false positives. What should not survive is the rule reporting agreement when it found no
claims at all.

## References

- `BUG-36`, the defect this rule was built for, and the shape it does catch.
- `BUG-56`, which stopped this same rule going inert when its input was unreachable. This is the same
  rule going inert with its input in hand.
- `RFC-36`, which takes the open question here as its subject and measures it: 44 of 49 references in
  this repository's changeset history are bare, and most are citations rather than unstated claims.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
