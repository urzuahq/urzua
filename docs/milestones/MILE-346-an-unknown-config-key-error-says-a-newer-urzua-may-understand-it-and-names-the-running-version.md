---
Stable-Id: 01M7QDN6J3ZZ1N1BSTFA5G0427
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on:
---
# 346 — An unknown config key error says a newer urzua may understand it and names the running version

## What

When the configuration carries a key the running binary does not know, the error says that a newer urzua may understand the key and names the version that is running, with the key and its location.

## Why

Today the error is only `unknown field <key>` with the list of expected fields. On 2026-11-18 the released 0.4.10 gave that message for `export`, a key the next version adds; nothing in it says the cause may be an older binary. Any adopting repository where one person or one CI job is a release behind the config meets this, and the reader cannot tell a misspelt key from a key that is newer than their binary.

## Done means

- Loading a config with an unknown top-level key or an unknown key inside a known section reports the key, the running version, and a sentence that a newer urzua may understand it; shown by a test on the error text and on the JSON error object. Where the YAML reader gives a line for the key the error carries it; the record does not promise a line the reader cannot give.
- The error says how to see the installed version and where releases are listed. A config error is not a finding and carries no suggested action today, so this is either a sentence in the message or a field added to the error object, and a test shows whichever it is.
- A misspelt key that is close to a known one still gets the existing expected-fields help, shown by a test.
- The hook command's output for a config the binary cannot load carries the same sentence, shown by a harness hook test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** the released 0.4.10 answered a config key added on main with only `unknown field export` on 2026-11-18. | **structural** |
