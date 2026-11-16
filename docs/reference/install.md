# Install

## Install a release

Releases are published at <https://github.com/urzuahq/urzua/releases> for two targets:
`x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` (Apple Silicon). The install script
[`scripts/install.sh`](../../scripts/install.sh) picks the target, downloads the archive, verifies it
and installs it to `$HOME/.local/bin` (no `sudo`). It installs nothing, and runs nothing it
downloaded, until the archive has verified.

The script is fetched at the release tag you install, so the script you run is the one that release
ships. It exists from the first release that includes it; for earlier releases (up to `v0.4.4`) use
[the manual steps](#install-by-hand).

### On a machine with `gh`

`gh` verifies the archive's build provenance, so no checksum is needed. This installs the latest
release:

```sh
tag=$(gh release view --repo urzuahq/urzua --json tagName --jq .tagName)
curl -fsSL --proto '=https' --tlsv1.2 "https://raw.githubusercontent.com/urzuahq/urzua/$tag/scripts/install.sh" | sh -s -- --version "$tag"
```

To install an exact release instead, set `tag` to its tag and run the second line. To
read the script before running it, save it with `curl -fsSLO` and run `sh install.sh`.

### In CI, or anywhere without `gh`

Pin the version and the archive's sha256, so the install fails if the archive is anything else. Take
the checksum from the release you pin (see [checksums](#checksums)):

```sh
tag="${URZUA_TAG:?set URZUA_TAG to an exact release tag, such as vX.Y.Z}"
sha256="${URZUA_SHA256:?set URZUA_SHA256 to the sha256 of this platform's archive at that tag}"
curl -fsSL --proto '=https' --tlsv1.2 "https://raw.githubusercontent.com/urzuahq/urzua/$tag/scripts/install.sh" \
  | sh -s -- --version "$tag" --sha256 "$sha256"
```

For a CI job that checks a repository's records, [the gate action](ci.md) does this for you.

### What the script does

Run `sh scripts/install.sh --help` for the flags and environment variables.

| Flag | Environment | Meaning |
|---|---|---|
| `--version vX.Y.Z` | `URZUA_VERSION` | An exact release tag. Default: the latest release. |
| `--dir PATH` | `URZUA_INSTALL_DIR` | Install directory, created if missing. Default: `$HOME/.local/bin`. |
| `--sha256 HEX` | `URZUA_SHA256` | The archive's expected sha256. A mismatch refuses the install. |
| `--no-attestation` | | Skip the provenance check; `--sha256` is then required. |
| `--no-verify` | | Install with no verification, with a warning. Use it only knowingly. |

- **Verification.** With `gh` installed and logged in, the build provenance attestation must verify
  (`gh attestation verify`, pinned to this repository's release workflow). Without `gh` (or not logged
  in) the script fails unless you pass `--sha256`, or `--no-verify`. A wrong checksum or a failed
  attestation installs nothing and leaves an existing `urzua` untouched.
- **Result.** The binary is written next to its destination, checked, then renamed into place with
  mode `0755`. The script then confirms the binary reports exactly the requested version
  (`urzua <version>`); any other string, including one that merely contains it, installs nothing.
- **Output.** The last stdout line is
  `installed urzua=<tag> target=<target> sha256=<hex> verified=<how> path=<file>`, where `<how>` is
  `attestation`, `attestation+sha256`, `sha256` or `none`. Everything else goes to stderr. A directory
  that is not on `PATH` is noted there. Exit codes: `0` installed, `1` refused or failed, `2` bad usage.
- **Network.** `curl` (or `wget`) over https only; with `curl`, redirects are held to https. The
  latest release is resolved from the `releases/latest` redirect, with no API call or token.
- **Platforms.** x86_64 Linux (glibc) and Apple Silicon. Anything else fails naming the supported
  targets and pointing at [build from source](#build-from-source).

### Install by hand

With the GitHub CLI, this downloads a release, checks its build provenance and puts the binary on
your `PATH`. It installs the latest release; to install an exact one, add its tag after `download`
(`gh release download vX.Y.Z ...`):

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
archive that workflow did not build.

### Checksums

A release has no checksum file. Compute the sha256 of an archive from the release you pin:

```sh
gh release download v0.4.4 --repo urzuahq/urzua --pattern 'urzua-*.tar.gz'
sha256sum urzua-*.tar.gz # on macOS: shasum -a 256 urzua-*.tar.gz
```

For `v0.4.4`, computed from the downloaded release assets:

| Archive | sha256 |
|---|---|
| `urzua-x86_64-unknown-linux-gnu.tar.gz` | `173e59244c353d332fa49e9c5f1bdc720eef52c8bcf76382c5d789c8f7a3f110` |
| `urzua-aarch64-apple-darwin.tar.gz` | `c222358e853f1e4804aed9db8caf18ab853e0a3ef4131be0aeb3194497afe34b` |

From a checkout of this repository, on x86_64 Linux, that release installs with the script on disk:

```sh
sh scripts/install.sh --version v0.4.4 --sha256 173e59244c353d332fa49e9c5f1bdc720eef52c8bcf76382c5d789c8f7a3f110
```

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
