---
default: minor
closes: [COND-0007]
---

Every `not-run` report now carries `error_kind` (`usage`, `config`, `environment` or `internal`), as the published agent CLI contract defines it. A usage error's `error` names the argument in one sentence, without the parser's `Usage:` block. A panic reports its text under `error` with `error_kind` `internal`, and the `panic` key is gone. The vendored contract is now the `v1.1.0-rc.3` release candidate, and `make conform` runs its conformance runner against a built binary.
