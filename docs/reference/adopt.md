# Adopt an existing corpus

This walkthrough takes a repository that already has decision records (ADRs, RFCs, specs or similar
markdown files in a directory) and puts them under urzua without rewriting any of them.

Every command below prints JSON. Examples are trimmed to the fields that matter; the full shapes are
in [`check` and its JSON output](check.md) and the [command reference](commands.md).

## 1. Check that your records have a header

Urzua reads a record's metadata from YAML frontmatter at the very start of the file:

```markdown
---
Status: Accepted
Date: 2026-01-10
Author: Alex
Deciders: Alex
---
# 1 - Use Postgres

## Context
...
```

The older blockquote (`> Status: Accepted`) and bold-list (`- **Status:** Accepted`) forms are not
read. A record still written that way is reported by `check` as unreadable, naming the form. Convert
those records to frontmatter by hand; `init` counts them for you in a `header-shape` notice so you
know how many there are.

A type whose records carry no header at all can be declared `header_shape: none`. Such a type has
nowhere to store a stable ID, so `urzua new` refuses it.

## 2. Preview the config

From the repository root:

```sh
urzua init --dry-run
```

`init` looks at your tree, proposes one record type for each directory that holds numbered markdown
records, and prints the config it would write. It moves, renames and rewrites nothing. The output is:

```json
{
  "status": "ok",
  "dry_run": true,
  "config_path": ".urzua/config.yaml",
  "proposed": [
    {"name": "adr", "dir": "docs/adr", "record_count": 2, "prefix": null, "header_shape": "yaml-frontmatter"}
  ],
  "written": false,
  "config_yaml": "...the file it would write, as one string...",
  "notices": [ ... ]
}
```

`notices` tell you what `init` could not infer. Two matter most:

- `header-shape`: some records are in a removed header form or have no readable header.
- `rule-applicability`: rules `init` left out because each needs a declaration only you can make
  (for example `required_fields`, `known_fields`, `pointer_fields`). Declare it on the type, then
  enable the rule.

## 3. Write it

```sh
urzua init
```

This writes `.urzua/config.yaml`. It never overwrites one: if the file exists, `init` reports what
exists and exits 2. Run it again with `--dry-run` at any time to see what a fresh adoption would
propose.

The starter config has every record type with `required_fields: []`, and each shipped rule that can
examine your corpus set to `warn`. A rule `init` withheld is simply absent. Commit the file.

## 4. Run the first check

```sh
urzua check
```

Expect findings, and expect most of them to be about naming rather than content. For example, urzua
identifies a record from its file name using the pattern `TYPE-NNNN-slug.md`, so a directory of
`0001-use-postgres.md` files produces a `filename.identity-pattern` finding listing the rename to
make for each file. Rename the files with `git mv` and update links to them; urzua does not rename
anything for you.

`check` exits 0 when nothing blocks. By default a finding blocks only at severity `error`; set
`fail_on: warning` in the config to make warnings block too. See [`check`](check.md).

Work through the findings in this order:

1. Config findings (`config.*`, `type.*`): they say the config does not match your tree.
2. `filename.identity-pattern`: rename files so every record has an identity.
3. `header.required-fields` once you declare `required_fields` for a type.
4. Everything else, rule by rule. `urzua explain --rule <id>` says what a rule judges and which
   options it has; `urzua rules` lists every rule the binary ships.

For findings you cannot fix today, file a [waiver](waivers-and-drift.md) with a reason and an expiry
date rather than turning the rule off. `urzua waive --from-report` previews the waivers it would
draft for you.

## 5. Give every record a stable ID

A record's file name can change; its `Stable-Id` does not. To add one to every record that lacks it:

```sh
urzua migrate ids          # a dry run: lists the records that would get one
urzua migrate ids --apply  # writes them
```

The file name's number stays as the record's display number, so existing references keep working.

## 6. Declare what you rely on

The starter config checks very little. Over the first week, add what your records already mean:

- `required_fields` and `known_fields` per type, so a missing or misspelled field is a finding.
- `statuses` per type: the allowed values, which are successful or failed, and the lawful
  transitions. See [the record model](record-model.md).
- `pointer_fields` for fields that name other records (`Implements`, `Derives-from`), so a dangling
  reference is a finding.
- `embodiment` and `Realized-by` evidence, if you want to track whether a decision was built.
  See [waivers and drift](waivers-and-drift.md).

Run `urzua doctor` after each change to the config. It reports configuration problems, including any
key urzua does not recognize, which `doctor` treats as an error.

## 7. Create new records with `new`

```sh
urzua new adr "Use S3 for blobs"
```

This picks the next number (skipping numbers used on other local branches and on remote-tracking
refs, which it names on stderr), writes the file with a fresh `Stable-Id`, today's date and the
author filled in, and prints its path. Put a template at `.urzua/templates/<type>.md` to give
new records of a type your own sections.

## 8. Run it in CI

Add the gate action so every pull request is checked. See [CI](ci.md).

## 9. Optional: agent hooks

If agents write records in your repository, install the harness so their edits are judged before they
land. See [the agent harness](agent-harness.md).
