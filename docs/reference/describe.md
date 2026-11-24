# `urzua describe`

```text
urzua describe
```

`describe` prints one JSON object that says how to call urzua: every command, every argument each
command takes, the environment variables and the configuration file urzua reads. An agent or a
consuming gate reads it instead of `--help`. It follows the `describe` shape of the published agent
CLI contract (its RFC-4, section 6) and validates against that contract's `describe.schema.json`.

It is not a report: it has no `status` and no `next_action` keys, and it exits 0. If it cannot
describe the command line it prints a `not-run` report and exits 2, like any other command. It reads
no configuration and no records, so it answers in any directory.

## Top level

| Key | Value |
|---|---|
| `name` | `urzua` |
| `version` | The binary's version, the same as `urzua --version` prints after the name. |
| `contract_version` | The contract version urzua conforms to: `1.0`. It moves to `1.1` once urzua meets every rule of 1.1 (`MILE-370`); the inputs below are printed already. |
| `system` | `0`: urzua's own work is deterministic. |
| `args` | The options every command accepts: `--config` and `--as-of`. `--as-of` is refused by the commands that compare no dates; its `means` names the ones that accept it. |
| `env` | `{name, means}` for each environment variable that changes what urzua does. `PATH` and `HOME` are not listed: urzua only uses them to find programs and the home directory. |
| `config` | `[{"path": ".urzua/config.yaml", "format": "yaml"}]`. No schema is given; the configuration is validated by `urzua doctor` and on every load. |
| `log` | `null`: urzua has no diagnostic log. |
| `commands` | One entry per command that runs, below. |

## A command

| Key | Value |
|---|---|
| `name` | The full path, words separated by one space: `migrate ids`, `harness run edit`. A group such as `harness` is not listed, only the commands under it, and `describe` does not list itself. |
| `profile` | `gate` for `check` and `audit`, whose reports carry `blocking`, `findings` and `examined`; `exempt` for a command that speaks a host's protocol; `base` for every other. |
| `kind` | `records`, on `check` and `audit`. |
| `safe` | `true` only for a command that writes nothing whatever its arguments and finishes quickly. A command whose `--apply` writes is not safe, and neither is `audit`, which runs every declared rule and cannot be narrowed. |
| `scopes` | `paths` for a command that takes record paths, `base-ref` for `check --base`. |
| `metrics` | `files` for `check` and `audit`, the count in their `examined`. |
| `args` | Each argument: `name` (`--base`, or a positional's name), `kind` (`flag`, `option` or `positional`), `type` (`string`, `integer` or `path`; a flag has none), `required`, `repeatable`, `means`, and where they apply `values`, `format` (`date` for `YYYY-MM-DD`), `conflicts_with`, `requires` and `after_separator`. |
| `stdin` | `null`, or `{format, means}` for a command that reads standard input: `check` and `fix` with `--stdin-path`, `init` on a terminal, and the exempt commands. |
| `example` | For a safe command that needs arguments to do real work, the arguments of one quick run. |
| `protocol` | On an exempt command: `mcp` (`urzua mcp`), `harness-hook` (`harness hook`), `harness-run` (each `harness run` event, a neutral request on stdin and a verdict on stdout) and `git-hook` (`hooks run`). A reader that does not know the word does not call the command. |

## How it is kept true

Every argument is read from the same definitions that parse the command line, so a flag cannot be
added without appearing here. What the parser cannot know, a command's profile, whether it is safe,
what it reads on stdin and its example, is stated once per command, and a command added without that
entry makes `describe` fail its tests (and print `not-run` if it ever shipped). The tests also fail
when an argument has no help text to use as its `means`, and when the source reads an `URZUA_`
variable that `env` does not list.

Two things the contract can say are not stated for urzua's arguments, because the parser does not
hold them: an exclusion checked in code rather than declared to the parser (`check --stdin-path`
cannot be combined with paths or `--base`, which `check` refuses itself) and a requirement that
depends on another argument's value (`fix --apply` needs `--ids` or `--force`).
