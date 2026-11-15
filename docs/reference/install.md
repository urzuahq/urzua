# Install

## Install a release

Releases are published at <https://github.com/urzuahq/urzua/releases> for two targets:
`x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` (Apple Silicon). Each archive is named
`urzua-<target>.tar.gz` and holds `urzua-<target>/urzua`. With the GitHub CLI, this downloads the
latest release, checks its build provenance and puts the binary on your `PATH`:

```sh
target=x86_64-unknown-linux-gnu # or aarch64-apple-darwin
gh release download --repo urzuahq/urzua --pattern "urzua-$target.tar.gz"
gh attestation verify "urzua-$target.tar.gz" \
  --repo urzuahq/urzua \
  --signer-workflow urzuahq/urzua/.github/workflows/publish-release.yml
tar -xzf "urzua-$target.tar.gz"
install -m 0755 "urzua-$target/urzua" /usr/local/bin/urzua
urzua --version
```

Each archive is built by the repository's release workflow, and the attestation check fails for an
archive that workflow did not build. To install an exact version, pass its tag:
`gh release download v<version> ...`. In CI, use [the gate action](ci.md), which installs a pinned
release for you.

There is no Intel Mac build and no Windows build. On an Intel Mac, build from source.

## Which version these pages describe

These pages describe the `main` branch, which can be ahead of the latest release. A rule, config
key, command or flag added since that release exists only in a build from `main` until the next
release is published. A release reports what it does not know and does not guess: a config that
names a rule it does not ship fails to load with `unknown rule '<id>'` and the list of rules the
build ships, and every command then exits 2 with a `not-run` report. `urzua --version` shows what
you have, and `urzua --help` shows which commands your build ships.

A release's own config always loads in that release. The mismatch arises when a config is written
from these pages, or copied from a repository that tracks `main`, and read by an older release.

## Build from source

Build from source for an Intel Mac, or for something on `main` that no release has yet. You need a
Rust toolchain. The repository pins one in `rust/rust-toolchain.toml`, which `rustup` selects for
you inside `rust/`.

```sh
git clone https://github.com/urzuahq/urzua
cd urzua
make build
```

`make build` builds the release binary at `rust/target/release/urzua` and wires this repository's own
git hooks. To build only the binary:

```sh
cd rust && cargo build --release -p urzua-cli
```

Put the binary on your `PATH`, or call it by its full path:

```sh
install -m 0755 rust/target/release/urzua /usr/local/bin/urzua
urzua --version
```

## Requirements at run time

- `urzua` must run inside a git repository. Outside one it exits 2 with a `not-run` report. Drift
  detection, `check --base` and `new` all read git.
- `git` must be on `PATH`.
- `refs refresh` needs `curl`, and reads `GITHUB_TOKEN` or `GH_TOKEN` when set. It is the only
  command whose job is to use the network.
- `gh` is optional. `new`, `waive` and the `--apply` modes of `fix` and `reverify` ask it who is
  logged in (`gh api user`) and fall back to `--by` and `git config user.name` when it is absent or
  not logged in. `new` with `new.check_open_prs: true` also runs `git fetch`. Everything else,
  including every `check`, makes no network call. See [what uses the
  network](commands.md#what-uses-the-network).
- The agent harness hooks need `python3` (see [the agent harness](agent-harness.md)).

## Telemetry

Urzua sends none. See the "Telemetry" section of the [README](../../README.md).

## Next

[Adopt an existing corpus](adopt.md).
