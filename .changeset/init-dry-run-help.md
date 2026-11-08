---
default: patch
closes: []
---

`urzua init --help` and the command's documentation no longer say the `--dry-run` output is
byte-identical to the real run's. The config a dry run proposes is byte-identical to the file the real
run writes; the report differs in `dry_run`, `written` and in carrying the config text only on a dry run.
