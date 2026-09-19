---
default: major
---

Config is now YAML: `.urzua/config.yaml` replaces `.urzua/config.toml` (ADR-52).

Records were already YAML and the config was TOML, so an adopter met two formats
on day one. They are now one. `urzua-core` carried both parsers; dropping TOML
removes a dependency rather than adding one, and `toml` leaves the purity
allowlist.

**Breaking.** An existing `.urzua/config.toml` is not read, and nothing reports
that it is there: `ADR-54` decided against carrying migration diagnostics for a
format the tool no longer parses. Convert it to `.urzua/config.yaml` before
upgrading; a leftover TOML file is ignored, not removed.

`urzua init` writes `config.yaml`, rendered through the real serializer instead
of string concatenation, so a directory name carrying the format's
metacharacters can no longer escape its value.
