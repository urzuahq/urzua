---
"urzua": minor
---

New rule `claim.status-agreement`: a file outside the corpus that claims to
close a record, while the record itself says otherwise.

Written after a changeset in this repository announced it closed `BUG-36`. It
had not -- it fixed a hazard recorded *beside* that bug -- and nothing noticed,
because the claim and the record it contradicted live in different files and
only one of them was ever read.

Opt-in like every rule, with both options declared rather than inferred:

```yaml
rules:
  claim.status-agreement:
    level: error
    claim_paths: [".changeset"]
    closed_statuses: ["Fixed", "Done", "Accepted"]
```

A built-in status list would make the rule stop applying the moment a
repository used a status it did not know.

It matches on verb and proximity, not meaning, so it can be wrong in both
directions: a present-tense sentence *discussing* a claim will match, and a
claim phrased without one of the verbs will not. Only the present tense is
matched -- "closes X" is how a claim is written, while "announced it closed X"
is prose about one, and including the past tense made the rule fire on its own
changeset. A waiver (`ADR-11`) is the intended escape, deliberately a record so
an exception is visible rather than a silent pattern tweak.
