---
default: patch
closes: []
---

The editor hook now also judges a NotebookEdit and any MCP tool whose input carries a file path and the file's content, as a write of that file; an MCP call it cannot read as a whole-file write is refused under `on_unjudged: deny` only when its file is a protected path or a gated record. The post-command check reports a shell command that changed a gitignored protected file (such as `.claude/settings.local.json`) by keeping a content hash of every file under a protected path. Protected-path and record-directory matching use Unicode case folding, and on Unix a write through a hard link is judged as the protected path or gated record it shares an inode with. A change is still reported once; the documentation now says so.
