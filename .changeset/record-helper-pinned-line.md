---
default: patch
closes: []
---

The installed record helper (`urzua-record.sh`) no longer occasionally omits its `Pinned:` line, or
reports "nothing was re-pinned", after a successful `reverify` or `repin`. It tested the engine's
JSON through a pipe into `grep -q` under `pipefail`, where `grep` exiting on its first match could
turn a match into a failure; it now reads the text directly.
