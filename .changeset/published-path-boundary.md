---
default: minor
---

Adds `boundary.published-path-reference`, an opt-in rule that reports a record identifier cited from inside a path your repository publishes (a mirror, a published package, a vendored copy). Declare the paths with `published_paths`, a list of path prefixes (globs are not supported; a glob is treated literally; a leading `./` and trailing `/` are ignored, and an absolute path, a `..` component or a backslash is rejected) and the severity with the rule's `level`:

```yaml
rules:
  boundary.published-path-reference:
    level: error
    published_paths: ["packages/sdk"]
```

Any identifier carrying one of your record types' prefixes counts, in prose as well as in an `Implements:` marker, whether or not it resolves to a record. An identifier split across lines is not detected. A declared path that covers no tracked file is itself reported rather than passing, and a file that is not text is counted as unreadable, not clean. The rule's `population` is counted in a new `file` unit.
