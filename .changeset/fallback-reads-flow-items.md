---
default: patch
closes: [BUG-201]
---

The hook command's fallback when `urzua` is missing or too old no longer refuses every edit on a configuration whose guardrails are open because a quoted value sits in a one-line flow map elsewhere in the file. It now scans the configuration as YAML reads it (quotes, comments, flow collections across lines, block scalars) and looks for keys only where they can change the guardrails; a complex `? harness` key, which it used to read as open, is now refused. Run `urzua harness update` to install the new hook command.
