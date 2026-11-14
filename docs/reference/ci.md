# CI

Run urzua in CI so no change merges with a blocking finding. The repository ships a composite GitHub
Action for this, and one extra step is worth adding if your records pin evidence.

## The gate action

```yaml
name: urzua
on: [pull_request]
permissions:
  contents: read
  pull-requests: write   # for the comment; without it the comment is skipped with a warning
jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha>
        with:
          fetch-depth: 0   # drift reads history; a shallow checkout fails with that reason
      - uses: urzuahq/urzua/.github/actions/urzua-gate@<sha>
        with:
          version: v0.3.0                      # an exact release tag
          base-ref: ${{ github.base_ref }}     # empty outside pull requests
```

Pin the action itself to a commit sha, as you would any third-party action.

**Which version.** `version` must name a release whose config format matches yours. The latest
published release (`v0.3.0`) reads an older config than `main` does (see [Install](install.md)), so
for a config written from these pages use the `binary` input until the next release is published:
build urzua in an earlier step and pass its path.

```yaml
      - run: git clone --depth 1 https://github.com/urzuahq/urzua /tmp/urzua && (cd /tmp/urzua/rust && cargo build --release -p urzua-cli)
      - uses: urzuahq/urzua/.github/actions/urzua-gate@<sha>
        with:
          binary: /tmp/urzua/rust/target/release/urzua
          base-ref: ${{ github.base_ref }}
```

### What it does

1. Installs the pinned release. It downloads `urzua-<target>.tar.gz` from the release, verifies its
   build provenance attestation (and its sha256 when you pass `sha256`), unpacks it, and refuses a
   binary that does not report the pinned version. Published builds are x86_64 Linux and arm64 and
   x86_64 macOS.
2. Checks the checkout has the history drift reads. A shallow checkout fails the job with that
   reason, because drift detection would otherwise read as clean. With `base-ref` it also fetches
   the base branch as `origin/<base-ref>`.
3. Runs `urzua check`, and when `base-ref` is set, `urzua check --base origin/<base-ref>`. Both are
   always run and both are reported before the verdict.
4. Reports. Each open finding becomes an inline annotation (`error` as an error, `warning` as a
   warning, `info` as a notice), and one summary comment on the pull request is created and then
   updated in place, never stacked. Waived findings are counted, not listed. The summary is also
   written to the job summary.
5. Gives the verdict. Exit 1 from either run (a blocking finding) fails the job. A run that could not
   run (exit 2, or no JSON report) also fails the job and is shown as **NOT RUN** with its reason. A
   gate that did not run is never a pass.

A comment that cannot be posted (a fork's read-only token, a missing permission) is a warning and
never changes the verdict. The comment is found by a marker on a bot-authored comment, so use the
workflow's `GITHUB_TOKEN` or a GitHub App token; with a personal access token a new comment is
posted on every run.

### Inputs

| Input | Default | Meaning |
|---|---|---|
| `version` | none | The exact release tag to install, such as `v0.3.0`. Not a branch and not `latest`. Ignored when `binary` is set. |
| `sha256` | none | Optional sha256 of the release archive for this platform, checked in addition to the attestation. |
| `repository` | `urzuahq/urzua` | The repository the release and its attestation come from. |
| `binary` | none | Path to an already-built `urzua`, used instead of installing a release. |
| `base-ref` | none | The pull request's base branch name. When set, also runs `check --base origin/<base-ref>`. |
| `working-directory` | `.` | The directory holding the checkout to check. |
| `actors` | `false` | `true` writes the pull request's author and current approvers from the GitHub API and passes them to `check --base --actors`, for the `diff.decider-required` rule. Only for a repository that declares `authority`. Needs `pr-number`. |
| `annotate` | `true` | Emit inline annotations. |
| `comment` | `true` | Post the summary as a pull request comment. Acts only when `pr-number` is set. |
| `pr-number` | the event's pull request | The pull request to comment on. |
| `github-token` | the workflow token | Token for the attestation check and the comment. |

## Fail on warnings

The gate fails the job on whatever your config says blocks. To make warnings block, set
`fail_on: warning` in `.urzua/config.yaml`. Both the whole-repository run and the `--base` run read
it. A repository that adopted urzua with a backlog usually starts at `error`, waives or fixes the
backlog, and moves to `warning` when it is clean.

## The stale-pin step

If your records pin evidence with `@sha256:` locators (see [Waivers and
drift](waivers-and-drift.md)), add a step that fails when any pin is stale. `check` already reports a
stale pin on any record, whatever the record states, as the warning `embodiment.pin-stale`, which is
on by default (a repository sets another level, or `off`, under `rules:`); a record that states an
`Embodiment` also shows drift. A warning blocks only under `fail_on: warning`, so this step is how
a repository with the default `fail_on` makes a stale pin fail the job. It names each stale pin and
the commands to fix it, and fails on a pin that cannot be read at all.

```yaml
      - name: no stale evidence pins
        shell: bash -eo pipefail {0}
        run: |
          report="$(mktemp)"
          urzua fix --rebaseline > "$report" || true
          if ! jq -e '.status == "ok"' "$report" > /dev/null; then
            echo "::error::stale or unreadable evidence pins: re-pin each record with a per-file reason" >&2
            echo "Stale pins (record  locator):" >&2
            jq -r '(.rebaselines // [])[] | .record as $r | .pins[] | "\($r)  \(.locator)"' "$report" >&2 || true
            echo "Pins that cannot be re-pinned (record  locator  reason):" >&2
            jq -r '(.failed_pins // [])[] | "\(.record)  \(.locator)  \(.reason)"' "$report" >&2 || true
            echo "Re-pin with: urzua fix --rebaseline --apply --ids <record> --by <you> --reason '<what changed in the pinned code>'" >&2
            exit 1
          fi
          echo "no stale evidence pins"
```

This step needs a build that has `fix --rebaseline`; the published `v0.3.0` does not, so with `version: v0.3.0` use the `binary` input instead.

`fix --rebaseline` without `--apply` only reads. The step passes only when the report says
`"status": "ok"`: any other status, a missing key, empty or unparseable output fails it. `urzua` must
be on `PATH` (with the gate's `binary` input, call it by its path), and `jq` is preinstalled on
GitHub-hosted runners. This is the step this repository's own CI runs.

## Other things worth a step

- `urzua doctor` fails (exit 1) on a configuration error, including an unrecognized key. It also
  warns when no workflow runs `urzua check`, which a CI job like the above silences.
- `urzua harness status` fails (exit 1) when an installed agent harness has drifted from what
  `urzua harness install` generates. See [the agent harness](agent-harness.md).
- `urzua refs refresh` is for a maintainer to run and commit the result; it uses the network, so it
  does not belong in the gate. (`check` and `check --base` make no network call.)

## Local hooks

A pre-commit hook that runs `urzua check` catches findings before they reach CI. `urzua doctor`
checks for `pre-commit` and `pre-push` hooks in `.git/hooks` that invoke it and warns when they are
missing. CI remains the guarantee: a hook can be skipped.
