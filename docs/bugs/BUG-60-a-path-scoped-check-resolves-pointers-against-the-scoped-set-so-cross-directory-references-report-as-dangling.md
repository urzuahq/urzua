---
Stable-Id: 01M2WCN789ABR543084EZ90ENX
Status: Fixed
Found-in: "Round 6 of the 0.4.0 release review, then reproduced on this repository's own corpus"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_path_scope_narrows_what_is_reported_on_not_what_a_pointer_resolves_against@sha256:0aa71b69206eb2ac9998587e601e512d4b43f17776498ec002a632834f7f00fa
---
# 60 — A path-scoped check resolves pointers against the scoped set so cross-directory references report as dangling

## What was wrong

`check` accepts a path argument to narrow what it reports on. It also narrows the record set that
`pointer.resolution` builds its index from, so a reference whose target lives outside the requested
path is indistinguishable from a reference to nothing.

On this repository, with its committed configuration:

```text
urzua check docs/adr/   ->  36 findings "does not resolve to any discovered record", blocking, exit 1
urzua check docs/       ->   0
```

Every one of those 36 targets exists. `docs/adr` holds records whose `Derives-from` names an RFC, and
`pointer.resolution` is declared `error`, so narrowing the scope turns a clean corpus into a failing
one. The documented CI invocation is a path-scoped check.

`relation.supersession-reciprocity` reports the same way. `narrative-field.stale` and
`pointer.target-status` fail in the opposite direction: an out-of-scope target is silently skipped, so
they report a pass over a reference they could not read.

The scope argument should select which records are *reported on*. It should not decide which records
exist for the purpose of resolving a pointer.

## Why nothing caught it

`check_scopes_to_the_requested_path_not_the_whole_corpus` covers the scoping behaviour, but its
fixture contains no reference that crosses a directory, and it asserts on `files_examined` alone --
never on the findings. A test that counts files cannot see a rule reaching the wrong verdict about
them.

The narrowing was added so that `check` on a subdirectory does not report on the whole tree. Nothing
distinguished the two things the record set is used for: the population to judge, and the population
to resolve against.

## References

- `BUG-0001`, which introduced the scoping this defect rides on.
- `ADR-53` -- every rule is a declared policy; a policy that inverts its verdict based on an argument
  unrelated to the policy is not one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#a_path_scope_narrows_what_is_reported_on_not_what_a_pointer_resolves_against` (lines 1458-1496) 8216a7cf657a -> 0aa71b69206e. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#a_path_scope_narrows_what_is_reported_on_not_what_a_pointer_resolves_against: the output assertions now read the JSON field with json_field instead of substring-matching the printed text, with the same expected values. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
