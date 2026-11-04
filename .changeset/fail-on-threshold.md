---
default: minor
---

Add a `fail_on` setting (`error` or `warning`, default `error`) to `.urzua/config.yaml`, and a
matching `urzua check --fail-on` flag that overrides it. With `fail_on: warning`, `urzua check` and
`urzua audit` exit 1 on any unwaived warning, so a repository can make warnings fail its CI. The JSON
report is identical either way; only the exit code changes.
