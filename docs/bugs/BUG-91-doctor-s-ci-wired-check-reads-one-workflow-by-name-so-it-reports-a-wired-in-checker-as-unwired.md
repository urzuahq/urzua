---
Stable-Id: 01M2YYV1MPC935EX3YT93N4JHT
Status: Fixed
Found-in: "Measuring whether `doctor` was worth wiring into CI, while planning the population work"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::ci_wired_finds_the_invocation_in_any_workflow_not_just_ci_yml"
---
# 91 — doctor's ci-wired check reads one workflow by name so it reports a wired-in checker as unwired

## What was wrong

`doctor`'s `ci-wired` check read `.github/workflows/ci.yml` by name and reported a warning when that
one file did not mention `urzua check` or `make records`:

```text
ci-wired | warn | .../ci.yml does not invoke `urzua check` or `make records`
                  -- a check that exists but is never run reports nothing to anyone
```

This repository has **four** workflows. `ci.yml` exists and does not invoke the checker; `checks.yml`
does. So the check ran green CI while asserting the checker was not wired in.

A check whose entire subject is *"is the checker actually wired in?"* reported a falsehood about
whether the checker was wired in, on a repository where it was. Nothing downstream consumed it --
`doctor` is in no Makefile target and no workflow -- so the wrong answer cost nothing until someone
went to trust it.

Now scans every `*.yml`/`*.yaml` under `.github/workflows/`.

## Why nothing caught it

`doctor` is not run by `make ci` or by any workflow, so no gate has ever read its output. The only
test touching this check (`check_integration.rs:246`) asserts the *warning* path and a zero exit
code, which is the behaviour the defect produces -- a test pinning the wrong answer.

The hardcoded filename also predates this repository having more than one workflow. It became wrong
when `checks.yml` was added, and nothing re-derived the check from that change.

## Also fixed here

`doctor`'s `required_fields` warning claimed *"field-quality/header rules will never fire for it"*.
Measured: with `required_fields: []`, `header.required-fields` fires on every record and reports
`no header-shaped region found -- required fields [] cannot be checked`. The header half was false.
Corrected to name `field.quality` and `field.pending` only.

## References

- `ADR-55`, on a check reporting success without looking -- this is its inverse, a check reporting
  failure without looking.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
