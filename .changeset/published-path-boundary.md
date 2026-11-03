---
default: minor
---

Adds `boundary.published-path-reference`, an opt-in rule that reports a record identifier cited from inside a path your repository publishes (a mirror, a published package, a vendored copy). Declare the paths with `published_paths` and the severity with the rule's `level`:

```yaml
rules:
  boundary.published-path-reference:
    level: error
    published_paths: ["packages/sdk"]
```

Any identifier carrying one of your record types' prefixes counts, in prose as well as in an `Implements:` marker, whether or not it resolves to a record. A declared path that covers no tracked file is itself reported rather than passing, and a file that is not text is counted as unreadable, not clean. The rule's `population` is counted in a new `file` unit.
