---
default: minor
closes: [MILE-353]
---

With `acceptance.in_session: ask`, a person can now approve an agent's move of a record to a gated status (an ADR to `Accepted`, say) even when Claude Code shows no permission prompt, as in `bypassPermissions`. The agent's edit is refused with one question for it to ask through AskUserQuestion, worded by urzua and naming the record and the move. If the person chooses `Approve <nonce>`, the agent's retry of the same edit within five minutes is written once, with a receipt row that names the question and the host. A question whose input already carries an answer is refused. A decline, free text or an idle timeout approves nothing. The permission prompt is still used where it is shown. Run `urzua harness install` again to add the two hooks on AskUserQuestion; until then the question is asked but no answer is recorded. Like the prompt, this route trusts the host and its permission hooks to report the person's answer: it guards the normal case and is not a security boundary.
