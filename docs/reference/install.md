# Install

## Which version these pages describe

These pages describe the `main` branch. The latest published release is `v0.3.0`, and it is older
than `main` in ways that matter: its config format is different (a `v0.3.0` config does not load on
`main`, and the other way round), and it has none of `waive`, `reverify`, `harness`, `roadmap`,
`rules`, `index` or `refs`. The commands and flags in these pages that `v0.3.0` does not have are
available only in a build from `main` until the next release is published.

If you want what these pages describe today, build from source. Once the next release is out, a
release archive will match them. Run `urzua --version` to see what you have, and `urzua --help` to
see which commands your build ships.

## Build from source

You need a Rust toolchain. The repository pins one in `rust/rust-toolchain.toml` (currently 1.87.0),
which `rustup` selects for you inside `rust/`.

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

## A release archive

Releases are published at <https://github.com/urzuahq/urzua/releases> for three targets:
`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`. Each archive is named
`urzua-<target>.tar.gz` and holds `urzua-<target>/urzua`. Each is built by the repository's release
workflow with a build provenance attestation, which you can check with the GitHub CLI:

```sh
gh attestation verify urzua-x86_64-unknown-linux-gnu.tar.gz \
  --repo urzuahq/urzua \
  --signer-workflow urzuahq/urzua/.github/workflows/publish-release.yml
tar -xzf urzua-x86_64-unknown-linux-gnu.tar.gz
./urzua-x86_64-unknown-linux-gnu/urzua --version
```

There is no Windows build.

## Requirements at run time

- `urzua` must run inside a git repository. Outside one it exits 2 with a `not-run` report. Drift
  detection, `check --base` and `new` all read git.
- `git` must be on `PATH`.
- `refs refresh` needs `curl`, and reads `GITHUB_TOKEN` or `GH_TOKEN` when set. It is the only
  command that uses the network.
- The agent harness hooks need `python3` (see [the agent harness](agent-harness.md)).

## Telemetry

Urzua sends none. See the "Telemetry" section of the [README](../../README.md).

## Next

[Adopt an existing corpus](adopt.md).
