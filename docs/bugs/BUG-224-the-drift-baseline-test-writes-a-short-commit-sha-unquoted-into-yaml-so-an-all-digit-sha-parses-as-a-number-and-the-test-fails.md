---
Stable-Id: 01M85TW3ECH7RSX3BCZRGKJAPJ
Status: Fixed
Found-in: "A CI run of the urzua-cli tests on an unrelated pull request, where `a_baseline_ref_that_is_a_tag_or_a_commit_sha_is_accepted` failed with `could not parse config: rules: invalid type: integer 8869738366, expected a string`."
Realized-by: test:rust/crates/urzua-cli/tests/drift_baseline_integration.rs#a_baseline_ref_that_is_a_tag_or_a_commit_sha_is_accepted@sha256:15dfdb1fc6bff2edef0152482b835d0d89d3bc10cff7e41977c4198f1e4e9ff1
---
# 224 — The drift-baseline test writes a short commit sha unquoted into YAML, so an all-digit sha parses as a number and the test fails

## What was wrong

`a_baseline_ref_that_is_a_tag_or_a_commit_sha_is_accepted` writes `drift_baseline: {ref: <sha>}` with the sha unquoted. A ten-character abbreviated sha made only of digits (about one in a hundred commits) parses as a YAML integer, and one of the form `NeN` as a number too, so the config fails to load and the test fails on a run whose commit happens to have such a sha. The engine's refusal is correct: a ref must be a string. The defect is the test's.

## Why nothing caught it

The test's input depends on a random commit sha, and every sha it had met contained a letter. Nothing exercised the value as YAML rather than as a ref.

## References

The test quotes the ref. With an all-digit ref planted, the unquoted form fails to parse the config and the quoted form parses it (the run then reports that the ref names no commit, as it should for a planted value).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/tests/drift_baseline_integration.rs#a_baseline_ref_that_is_a_tag_or_a_commit_sha_is_accepted@sha256:15dfdb1fc6bf Reason: Pin the test that quotes the ref **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
