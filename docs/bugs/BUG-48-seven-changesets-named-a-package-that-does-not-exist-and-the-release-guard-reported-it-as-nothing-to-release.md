---
Stable-Id: 01M2VJYJPSGXWM7YB2E0T53DEN
Status: Fixed
Found-in: 'Cutting 0.4.0 -- the release branch was three days stale while `prepare release` reported success on every push'
Regression-test: '.github/scripts/release-guard.test.sh -- plants an unmatched-package fragment and asserts a nonzero exit, with the empty and successful cases asserted alongside it. Observed failing: neutering the guard makes the planted case report `want exit 1, got 0`.'
---
# 48 — Seven changesets named a package that does not exist, and the release guard reported it as nothing to release

## What was wrong

`knope.toml` declares a **single unnamed** `[package]`, so a changeset addresses it as `default`:

```text
---
default: major
---
```

Every fragment written on 2026-09-18 addressed `"urzua"` instead -- a package knope has never heard
of. A fragment naming an undeclared package is not an error; it simply applies to nothing. So seven
accumulated, `PrepareRelease` found no version change, and exited `releases::no_release`.

The workflow treats that exit as normal:

```text
::notice::nothing to release -- no changeset fragment and no releasable commit since the last tag
```

Which is true after a release merge, and false here. `prepare release` reported **success on three
consecutive pushes** while the `release` branch sat three days stale at 0.3.0, and the only visible
symptom was a release PR that never appeared.

## Why it matters more than the typo

The typo is mine and took one `sed` to fix. The guard is the defect: it cannot tell *"nothing is
waiting"* from *"seven things are waiting and none of them work"*, and it reports both as success.

That is the same shape this project has found nine times in two days -- a step that reports success
without doing the work -- in the one place where the symptom is invisible, because a release that does
not happen looks exactly like a release that was not due.

## Fix (shipped)

Both halves:

- The seven fragments address `default`.
- The guard counts `.changeset/*.md` before excusing a `no_release` exit. Fragments present and none
  applied is now a **hard failure** naming the likely cause, because that combination is never
  legitimate.
- The guard is **extracted from the workflow into a script**, so it can be run. A check that lives only
  in a YAML `run:` block cannot be observed failing, and `AGENTS.md` requires that it can be --
  `make ci` now exercises it.

`ADR-48` records that a Conventional Commit alone must not cut a release; this is the mirror -- a
changeset that cannot cut one must not pass silently.

**The named form is not wrong in general.** `"urzua": major` is what a fragment looks like under
`[packages.urzua]`, and this repository would use it the day it declares more than one package. It is
wrong against `[package]` singular, where the only key is `default` -- and the failure mode is the
problem regardless of which form is correct, because knope treats an unmatched package name as
applying to nothing rather than as a mistake. A multi-package config would hit the same silence from a
misspelled name.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed and fixed. **Why:** found while cutting 0.4.0, when the release PR that should have existed did not. The fragments were the typo; the guard treating seven inert fragments as "nothing to release" is the defect, and it is why three green runs hid it. | **substantive** |
