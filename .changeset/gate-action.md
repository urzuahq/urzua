---
default: minor
closes: [MILE-180]
---

Add `urzuahq/urzua/.github/actions/urzua-gate`, a reusable GitHub Action that installs a pinned
release, verifies its build provenance, checks the checkout has full history, runs `urzua check`
and, on a pull request, `urzua check --base`, and reports findings as inline annotations and one
pull request comment that is updated in place. `urzua doctor` recognises a workflow that uses it
as wiring the checker into CI.
