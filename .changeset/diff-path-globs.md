---
default: minor
closes: []
---

The path options of the diff rules now accept globs as well as prefixes, and `diff.source-requires-record` accepts `exclude_paths`. A set such as every source file except tests is written `source_paths: ["src"]` with `exclude_paths: ["**/*.test.*", "src/tests"]`; `*` stays inside one path component, `**` crosses them, and an entry with no glob character is a prefix exactly as before. A diff that touches only excluded files is not a trigger, and the report keeps those paths in the population as unexamined. A glob that cannot compile is refused at load.
