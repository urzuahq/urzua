---
default: patch
closes: [BUG-214]
---

The hook fallback that runs when `urzua` cannot (not on PATH, or a release without `harness hook`) no longer validates `.urzua/config.yaml` for the session-start and post-command events, which never used the result. Only the edit fallback reads the configuration now, so the other two answer at once instead of running the YAML check, which was slow enough on some platforms to time out the agent host.
