---
default: patch
closes: [BUG-170]
---

Outside a git repository, commands that read one now fail with `urzua needs a git repository: no .git
directory found above the given path` (exit 2). The documented directory-walk fallback never existed and
is removed from the specification; discovery is the tracked set.
