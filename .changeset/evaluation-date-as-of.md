---
default: minor
closes: []
---

Add a global `--as-of YYYY-MM-DD` option that fixes the date every date comparison reads: waiver
`Expires`, claim age (including ages taken from git history), status expiry and the audit grace.
`urzua check --as-of <date>` previews which waivers and claims will have lapsed by then, and a verdict
can be reproduced on another day. The reports of `check`, `audit`, `fix` and `reverify` carry a
top-level `as_of` object, `{"date": "YYYY-MM-DD", "source": "flag" | "clock"}`, naming the date used and
where it came from. Without the option the date is the machine's UTC date, as before. A malformed date
exits 2 with a structured error. The option is refused with `--apply`, where a revision row would
carry a false date, and by commands that compare no dates.
