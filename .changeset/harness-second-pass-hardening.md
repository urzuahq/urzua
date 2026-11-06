---
default: patch
closes: []
---

The post-command check now reports a shell command that changed a protected path (an `authority` path gate, such as `.claude` or `.urzua/config.yaml`), and covers a command that committed its own change: it remembers the commit it last saw (seeded at session start) and examines the files changed since. Both are reports, never blocks. An edit the adapter could not hand to the engine (an unreadable payload, an edit it cannot place, an engine that cannot be run) is now refused where `acceptance` or `protected-paths` is on with `on_unjudged: deny`, and still allowed with a notice where the configuration allows. An edit from a working directory outside the repository is judged by the repository that holds the file, protected paths and record directories match without regard to letter case, and an edit through a symlink is judged as the file it resolves to. `urzua export` and `urzua import` now require `--format`, and the `urzua fix --help` text matches what `fix` does.
