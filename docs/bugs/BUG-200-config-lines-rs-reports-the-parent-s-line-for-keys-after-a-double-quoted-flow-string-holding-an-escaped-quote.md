---
Stable-Id: 01M7MZ3SRFC9WWCCF63WA1T72E
Status: Open
Found-in: 'review of the explain --types summary pull request, 2026-11-17, on a scratch configuration'
Realized-by:
---
# 200 — config_lines.rs reports the parent's line for keys after a double-quoted flow string holding an escaped quote

## What was wrong

The line scanner behind the `at` of `urzua explain --types` and `--type` (`rust/crates/urzua-cli/src/commands/config_lines.rs`) reads a double-quoted string inside a flow sequence without honouring a backslash escape: a quote preceded by a backslash ends the string. A string with an odd number of quotes (`"a\"b"`) leaves the scanner inside a string at the end of the line, so it treats the flow sequence as still open and gives the keys that follow the line of their parent instead of their own.

Repro, in a scratch repository whose `.urzua/config.yaml` is:

```yaml
schema_version: 2
record_types:
  t:
    dir: docs/t
    required_fields: [Status, Stable-Id]
    statuses:
      values: ["a\"b", B, C]
      success: [C]
      failure: [B]
      initial: [B]
      transitions: {B: [C]}
    revision_log:
      marker: "**Revision log**"
```

`urzua explain --type t` reports `statuses.success` at line 6, `failure` at 6, `initial` at 6, `transitions.B` at 6 and `revision_log` at 3; the keys are on lines 8, 9, 10, 11 and 12. `statuses.values` is right (line 7). A string with an even number of quotes (`"say \"hi\""`) is placed correctly by accident.

## Why nothing caught it

The position tests use configurations whose flow strings hold no escaped quote, and the scanner's own comment names the multi-line flow collection as the one place it falls back to the enclosing key's line, not an escape inside a string.

## References

`rust/crates/urzua-cli/src/commands/config_lines.rs` (`FlowSeq::feed`). Found while reviewing the pull request that adds gates, field values and a checklist marker to the `urzua explain --types` summary; that change does not touch the scanner.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Open`. **Why:** found in review of the explain summary pull request on 2026-11-17, not fixed in it; decided by the builder on 2026-11-17, pending maintainer review. | **structural** |
