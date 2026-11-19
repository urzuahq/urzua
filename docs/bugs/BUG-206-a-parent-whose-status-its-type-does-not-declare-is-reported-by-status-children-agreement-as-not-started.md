---
Stable-Id: 01M7SKSSZY7NCN7TN6R8RHJGG3
Status: Fixed
Found-in: "an undeclared parent status in `status.children-agreement`, 0.4.12"
Realized-by: test:rust/crates/urzua-cli/tests/children_agreement_integration.rs#a_parent_whose_status_is_not_declared_is_not_taken_for_one_not_started@sha256:7187e691d5bb95f1086bf54fdcc402f93867a9244818df80f4e32c8976675859
---
# 206 — A parent whose status its type does not declare is reported by status.children-agreement as not started

## What was wrong

A parent whose `Status` is not one its type declares was classified not started, so `status.children-agreement` reported it as not started while a child had begun, and the export set `state_disagrees_with_children`. A value nobody declared was read as a claim about the work.

## Why nothing caught it

The state classification puts every status that is neither success, failure nor progress into not started, and the disagreement check read that state without asking whether the status was declared. The rule's tests used only declared statuses.

## References

A parent with an undeclared status now yields no disagreement, in the rule and in the export flag; the export's own `state` for the record is unchanged. `children_agreement_integration::a_parent_whose_status_is_not_declared_is_not_taken_for_one_not_started` uses `Someday`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
