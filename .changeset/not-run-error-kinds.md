---
default: minor
closes: [MILE-365, COND-0007]
---

Every `not-run` report now carries `error_kinds`, an array holding the kind of its cause (`usage`, `config`, `environment` or `internal`), as the published agent CLI contract `v1.1.0-rc.4` defines it. urzua stops at the first cause, so the array always holds one kind. A usage error's `error` names the argument in one sentence, without the parser's `Usage:` block. A panic reports its text under `error` with `error_kinds` `["internal"]`, and the `panic` key is gone. `make conform` runs the contract's conformance runner against a built binary, with `--require-run`.
