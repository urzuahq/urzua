---
default: minor
closes: []
---

`harness.limits` in `.urzua/config.yaml` sets the editor hook's timeout (`hook_timeout_seconds`,
10 to 600, default 120) and the caps on how much of one agent call the harness reads: files and
record checks per call, loose strings, input depth, path and string length, symlink hops, settings
file depth, the `urzua mcp` request line and the path-gate walk. Each cap is a security floor:
today's value is the default and the minimum, so you may raise it but a lower value, an unknown key
or a non-number stops the config from loading with a message naming the key and the floor. A call
beyond a cap is still an edit that was not judged, decided by `on_unjudged`. A config that declares
none behaves exactly as before.

`urzua harness limits` prints the values in force; the Claude Code translator reads them from it
and keeps the built-in values when it cannot. The installed hook entry's timeout comes from the
config at `urzua harness install`/`update`, and `harness status`, `doctor` and the session-start
check report it out of date after the config changes it.
