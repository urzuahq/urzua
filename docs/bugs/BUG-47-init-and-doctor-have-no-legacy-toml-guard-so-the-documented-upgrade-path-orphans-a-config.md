---
Stable-Id: 01M2VH55E4TSK6N8D0FC8D5ZCA
Status: Rejected
Found-in: 'A code review of the unreleased diff since v0.3.0, following the upgrade path a v0.3.0 repository would actually take'
Regression-test: 'not yet written -- verified live against a synthetic v0.3.0 repository: `init` refuses and leaves `config.toml` intact, and `doctor` reports the rename rather than advising `init`'
---
# 47 — `init` and `doctor` have no legacy-TOML guard, so the documented upgrade path orphans a config

## What is wrong

`ADR-52` moved the config from TOML to YAML. `load_config` handles the transition well: it detects a
sibling `.toml` and explains the rename rather than reporting the YAML file as merely missing.

`init` and `doctor` do not.

On a repository upgrading from v0.3.0, `doctor` reports:

> config.yaml does not exist -- run `urzua init` to adopt this corpus

with no mention of the `config.toml` sitting next to it. Following that advice, `init` checks only
whether `config.yaml` exists, finds it does not, and writes a freshly auto-detected config -- leaving
the existing one, **with its `required_fields`, `known_fields`, `pointer_fields` and `spec` pointers**,
orphaned on disk.

The tool's own upgrade instructions discard a hand-tuned configuration, and report success.

## Why it was missed

`ADR-52`'s migration was verified by converting this repository's config by hand. Nobody ran the path
an adopter would take, because this repository never took it.

## Fix

`init` refuses when a legacy config is present, naming it. `doctor` carries the same hint `load_config`
already has -- one message, three call sites, which is the argument for it living in one place.

`ADR-52` recorded the migration as *"undecided here; it affects exactly one repository today, which is
the cheapest this will ever be."* That was true and it stayed undecided, so the cheapest moment passed
without the decision being made.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** review of the unreleased diff. `load_config` detects a legacy TOML config and explains the rename; `init` and `doctor` do not, so the upgrade path the tool itself recommends writes a fresh config and orphans the existing one. `ADR-52` left the migration undecided on the grounds that it was cheapest now -- and now passed. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** `legacy_config` and `legacy_config_message` are shared, and `load_config`, `init` and `doctor` all use them. `init` refuses when a legacy config is present rather than writing a fresh one beside it, and `doctor` no longer sends the reader to the command that would. One message, three call sites -- carrying it in one was how two commands came to disagree with the third. | **substantive** |
> | 2026-09-19 | `Status: Fixed` → `Rejected`, and the fix removed before it shipped (`ADR-54`). **Why:** the defect was real and the fix worked. The question was whether to carry four lines describing a format the tool no longer reads, and that turns on how many repositories hold one. v0.3.0 has one asset download three days after release; the population is this repository, whose config was converted by hand. Nothing is destroyed either way -- a leftover `config.toml` is ignored, not deleted -- so what the guard bought was a notice for nobody. `ADR-54` makes it a general rule so the next two breaks do not each accrete their own shim. | **substantive** |
