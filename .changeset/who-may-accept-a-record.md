---
default: minor
closes: [MILE-200]
---

Add `authority`, a top-level configuration key that says who may accept a record and who may change a protected path: named decider groups of `logins`, the `evidence` that may name one (`pr-author`, the default, or `pr-approver`), and `gates` by record type and status or by path. A group with no login, a login in no group, and a gate that names a type, status or group the configuration does not declare are load errors naming the entry; so is enabling the `acceptance` or `protected-paths` guardrail with no gate of its kind.

With those guardrails enabled, `urzua harness run edit` refuses an agent's Write, Edit or MultiEdit that moves a record to a gated status, or writes a gated path. The refusal is final: a named override does not lift it and the output says so. A person editing outside the agent harness is not affected. If the configuration does not load, an agent's edit to the configuration or a gated path is still refused, with the way out named.

Add the opt-in rule `diff.decider-required` and `urzua check --actors FILE`. Given the pull request's author and current approvers as JSON, it reports a gated move or changed gated path whose author or approvers are not in the gate's group, judged against the merge base, and it is `not-applicable` without `--base`. With no actors file a local run reports `info`, and the option `require_evidence: true` makes that an error. It proves authorship, not who decided: with a single maintainer it cannot tell an agent acting under the maintainer's token from the maintainer. The `urzua-gate` action takes an `actors: "true"` input that writes the file from the GitHub API, listing only approvals given on the head commit. `urzua doctor` says that branch protection, the required status check and `CODEOWNERS` are the host's and are not verified.
