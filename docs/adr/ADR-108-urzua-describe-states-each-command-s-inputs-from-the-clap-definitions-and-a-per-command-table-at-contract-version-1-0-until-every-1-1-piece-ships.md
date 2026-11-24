---
Stable-Id: 01M85S5G8B3NY8QHMDBSEC4XGQ
Status: Proposed
Date: 2026-11-24
Author: beauwilliams
Deciders: beauwilliams
Embodiment:
Supersedes / Superseded-by:
---
# 108 — urzua describe states each command's inputs from the clap definitions and a per-command table, at contract version 1.0 until every 1.1 piece ships

## Context

`MILE-359` asks for `urzua describe` and for `schema_version` on every report, and its first Done
means item asks for their shape to be recorded in a decision. The published agent CLI contract owns
`describe`'s field list (its RFC-4, section 6, accepted by the maintainer: "I accept RFC, and yes
allow option can be outside rule"), and its describe schema at `v1.1.0-rc.4` fixes the keys.
What the contract leaves to each tool is where the values come from and a few classifications:
which commands are `safe`, which `profile` each has, which `protocol` word an exempt command
names, which environment variables count, and which `contract_version` to print.

urzua's command line is 40 runnable commands under clap (`Cli` in `main.rs`), with global options,
nested groups (`migrate`, `harness`, `harness run`, `hooks`, `refs`), `requires` and
`conflicts_with` between arguments, and value enums. 21 of its arguments had no help text when this
was written, so `means` could not have been read from them. clap exposes `conflicts_with`, the
possible values, the action and the value parser's type, and has no public getter for `requires`.

The contract's runner (`scripts/conform.py`) runs every command `describe` marks `safe` and not
`exempt`, with the command's `example` or no arguments, under a 60-second limit. On this
repository's corpus a debug build's `check` with no arguments takes 33 seconds and `audit` 26.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Generate `args` from `Cli::command()`, and keep one table in `describe` for what clap cannot know (profile, safe, kind, scopes, metrics, stdin, example, protocol) | An argument cannot be added without being declared; the table is small and a test fails when it misses a command | `requires` is read by parsing a probe command line, since clap has no getter; relations refused in code need a second, hand-kept table, checked by running each pair |
| B. Hand-write the whole `describe` answer | Any shape is expressible | Written twice; drifts from the parser, the failure `MILE-359` item 7 exists to prevent |
| C. Annotate each clap command with its profile and safety (custom attributes or `mut_arg` metadata) | One place per command | clap has no free-form metadata slot; it would mean a macro or a parallel enum touching every variant in `main.rs` |

## Decision

In the context of `describe` being the one place an agent reads urzua's inputs, facing a parser
that knows the arguments and nothing about safety or profiles, we decided option A, to keep the
arguments impossible to forget and the classifications in one reviewed table, accepting that a
relation urzua refuses in code is declared from a hand-kept table.

1. **Arguments.** Every argument is generated from `Cli::command()`, walking subcommands
   recursively. A command's `name` is its full path; only commands that run are listed (a group is
   not) and `describe` does not list itself, since its answer is not a report. The top-level
   `args` is empty: no option is accepted and honoured by every command. `--as-of` is declared on
   `check`, `audit`, `fix`, `reverify` and `waive`, the commands that accept it, and `--config` on
   every command but `rules`, `import`, `harness adapters`, `harness hook` and `harness record`,
   which never read it. `--help` and `--version` are not listed. `kind` is `positional` for a positional, `option` for an
   argument that takes a value, and `flag` otherwise. `type` is `integer` for an integer value
   parser, `path` for `PathBuf`, else `string`; `format` is `date` for a value named `YYYY-MM-DD`.
   `means` is the argument's help text; an argument without one fails a test. `requires` is read by
   parsing the command with the argument and reading the arguments clap then reports missing,
   less those it reports missing without it. A relation a command refuses in code (`check
   --stdin-path` with `paths` or `--base`, `fix --force` without `--apply`, `--as-of` with
   `--apply`) is declared from a table in `describe`, a conflict on both arguments; a test runs
   every declared pair and expects a refusal naming both. A "needs one of two" rule (`fix --apply`
   and `reverify --apply` need `--ids` or `--force`) is declared as `requires_any`, and a "required
   unless" rule (`explain <path>`, unless `--rule`, `--types`, `--type` or `--full`) as
   `required_unless` with `required: false`; the contract's rc.4 added both keys.
2. **The table.** `safe` is true only for a command that writes nothing whatever its arguments
   (the contract's RFC-1 section 3), so a command whose `--apply` writes is not safe; a safe command
   that is slow gets a quick `example` where it has one (RFC-4 section 8). `audit` is safe: it writes
   nothing, and runs in about 25 seconds here, under the runner's 60. `profile` is `gate` for `check` and `audit`, whose
   reports carry `blocking`, `findings` and `examined`; `exempt` for `mcp` (protocol `mcp`),
   `harness hook` (`harness-hook`), each `harness run` event (`harness-run`, a word of urzua's own:
   a neutral request on stdin and a verdict on stdout) and `hooks run` (`git-hook`); `base` for every
   other. A command with no table entry, or an entry with no command, fails a test, and makes
   `describe` print `not-run`.
3. **Environment.** `env` lists every variable that changes what urzua does. `PATH` and `HOME` are
   left out (only used to find programs and the home directory), and so is every variable only a
   test build reads; a test fails when the source names an `URZUA_` variable that is neither.
4. **Version.** `describe` prints `contract_version` `"1.0"`. The contract's RFC-4 section 9 takes
   1.1 whole, and `MILE-370` is the one milestone that changes the claim. The 1.1 keys (`args`,
   `env`, `config`, `log`, `stdin`, `protocol`) are printed already, and a test validates
   `describe` against the 1.1 branch of the schema as well as the 1.0 one.
5. **`schema_version`.** Every report that carries `status` carries `schema_version`, an integer (the
   type the contract's `report.schema.json` gives it), starting at `1` and raised only when a
   report's existing keys change meaning; it is added in the one serialisation every report goes
   through (`with_next_action` in `main.rs`), beside the `next_action` keys. Not built with
   `describe`: it touches every report, and `MILE-367` is changing that serialisation now.

## Reversibility

Easy. `describe` is new and nothing reads it yet besides the contract's runner. The table is one
array in `commands/describe.rs`; reclassifying a command is a one-line change. Generating the
arguments is the hard part to give up, and nothing argues for that.

## Consequences

- Adding a clap argument without help text, a command without a table entry, or reading a new
  `URZUA_` variable without declaring it fails `cargo test`.
- The 21 arguments that had no help text have one, so `--help` changes for them.
- A relation refused in code is declared from a table kept beside the code that refuses it; a test
  fails when a declared pair is accepted, but not when a new refusal is added without an entry.
- `migrate schema` has no `example`: any `--field` would name a record field, which the engine
  never hardcodes, so the runner gets its usage `not-run`.
- `check`'s `example` names `header.required-fields`, a rule this repository declares. In a
  repository that does not, the runner's `check` gets a `not-run` with exit 2, which it accepts.

## References

- `MILE-359`, `MILE-370`, `MILE-365`, `MILE-367`
- `ADR-46` (one shared output contract), `ADR-23` (stdout is always JSON), `ADR-106`
- The published agent CLI contract, RFC-4 sections 5, 6, 8 and 9, and its describe schema at `v1.1.0-rc.4`, vendored as `rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/schemas/describe.schema.json`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as Proposed with the `describe` built in the same change; the decision is the maintainer's. | **structural** |
> | 2026-11-24 | Revised while Proposed, after the published agent CLI contract's maintainers answered review on pull request #647: the top-level `args` is empty and each global is declared on the commands that take it; relations refused in code are declared; "needs one of two" and "required unless" rules are stated in `means`; `migrate schema` loses its example, which named a record field. | **substantive** |
> | 2026-11-24 | Revised while Proposed, after an independent review of pull request #647: `safe` means writes nothing, as the contract defines it, not also quick, so `audit` is safe. | **substantive** |
> | 2026-11-24 | Revised while Proposed, after the published agent CLI contract moved to `v1.1.0-rc.4`: the vendored schemas are that tag's; "needs one of two" and "required unless" rules are declared as `requires_any` and `required_unless` instead of only in `means`. `describe` still does not list itself. | **substantive** |
