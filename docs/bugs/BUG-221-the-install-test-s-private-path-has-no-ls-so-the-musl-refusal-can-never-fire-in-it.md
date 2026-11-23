---
Stable-Id: 01M85B7TX15M6ESQQ28V63Z1HX
Status: Open
Found-in: 'reviewing scripts/install-test.sh: every scenario runs install.sh under a private PATH built from a fixed tool list'
Realized-by:
---
# 221 — The install test's private PATH has no `ls`, so the musl refusal can never fire in it

## What was wrong

`scripts/install.sh` refuses a musl system with `ls /lib/ld-musl-* >/dev/null 2>&1` (line 110).
`scripts/install-test.sh` runs the installer under a private `PATH` of symlinks to a fixed list of tools
(line 22: `mktemp tar gzip sed grep tr cut head rm mv cp chmod mkdir cat sha256sum dirname`) plus stubs.
`ls` is not in it, so inside the test the `ls` in the installer's condition is "command not found", the
condition is false and the refusal cannot fire whatever the host. The test also never sets
`URZUA_SKIP_LIBC_CHECK` and has no musl case, so nothing asserts the refusal or its wording.

Planted run, with a `PATH` built the way the test builds it plus a stub `uname` reporting Linux x86_64 and
`sh scripts/install.sh --version v1.2.3 --dir <d>`:

- private `PATH` only: the installer goes past the libc check and stops at "neither curl nor wget is installed".
- the same `PATH` with a stand-in `ls` that exits 0 (a musl loader present): "this is a musl system (Alpine
  or similar) and the published Linux build needs glibc; build from source".

So the refusal is reachable only when `ls` resolves, and the test's `PATH` is built so that it does not. Adding
`ls` alone is not enough: the glob is the absolute `/lib/ld-musl-*`, which the test cannot create on a glibc
host without writing outside its sandbox, so the case needs a seam (for example a stub `ls`, or a variable
naming the loader directory) that is a small design call.

## Why nothing caught it

The refusal is a guard on a path the test environment does not have, and the test's tool list was written
for what the installer needs on a working system. A refusal that nothing exercises reads as covered and is
not (the failing-before, passing-after standard in `AGENTS.md`).

## References

- `scripts/install.sh` (line 110), `scripts/install-test.sh` (line 22, the `suite` function), `docs/reference/install.md` (line 34).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
