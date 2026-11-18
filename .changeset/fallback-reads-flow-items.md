---
default: patch
closes: [BUG-201]
---

The hook command's fallback when `urzua` is missing or too old no longer refuses every edit on a configuration whose guardrails are open because a quoted value sits in a one-line flow map elsewhere in the file. It now scans the configuration as YAML reads it, in linear time: quoted scalars and flow collections on one line or several, comments and block scalars are ordinary, and so are anchors and aliases outside `harness`. It reads keys with blanks before the colon and an indented document, and refuses what it cannot place: line breaks awk does not see (a lone carriage return, U+0085, U+2028, U+2029), a byte order mark after the start, bytes that are not UTF-8 and control characters, a tab in leading whitespace on any line, a duplicate top-level key, a directive without `---`, a merge key at the root, deep nesting, a line longer than 64 KiB, and a complex `? harness` key. Run `urzua harness update` to install the new hook command.
