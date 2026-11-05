---
default: minor
---

Adds declared boundaries: an opt-in rule, `boundary.published-path-reference`, that reads the paths your repository publishes, vendors or generates, and enforces what follows from each kind. A boundary is a set of path prefixes or globs with a `kind` (`published`, `vendored` or `generated`), a required `reason` and a `severity`:

```yaml
rules:
  boundary.published-path-reference:
    level: error
    ignore_marker: "record-id-ok"
    boundaries:
      - paths: ["packages/*/src"]
        kind: published
        reason: "Published to the registry; readers cannot resolve our record ids."
        severity: error
        exclude: ["packages/*/src/fixtures"]
```

- A record identifier cited inside a `published` boundary is reported, in prose as well as in an `Implements:` marker, whether or not it resolves to a record.
- A back-pointer (`Implements:` or `Verifies:`) inside a `generated` boundary is reported; a plain record identifier there is not.
- A `vendored` boundary is declared and prohibits nothing.
- A boundary's `severity` (`error` or `warn`) is the level of its findings; the rule's `level` only turns the rule on.
- A boundary's `exclude` globs remove paths from its check, and the `ignore_marker` text you declare excuses the one line it is on. An `exclude` entry, a boundary path or a marker that matches or suppresses nothing is itself reported, as is a file that is not text.
- A back-pointer inside a `published` boundary never counts as realization evidence.
- `published_paths`, a list of path prefixes or globs, is read as one published boundary with no reason required.

Every finding carries a `suggested_action` where an edit to the config can fix it. The rule's `population` is counted in a `file` unit.
