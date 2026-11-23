---
Stable-Id: 01M859GC4NTVJWZ06D753PK54X
Status: Fixed
Found-in: "Running the documented command with a nonexistent tag in the URL: `curl` printed a 404 and the pipeline exited 0."
Realized-by: test:scripts/install-test.sh@sha256:cd8b216d2a197c9e9e18585fae8f55730e2c83c1ae2c608efaa18e2c491e37f1
---
# 218 — The documented install command pipes the script into sh, so a failed download exits 0 with nothing installed

## What was wrong

`docs/reference/install.md` documented `curl ... | sh -s -- --version "$tag"` for both the `gh` and the CI install. When the download fails, `sh` reads empty input and the pipeline exits `0` having installed nothing, so a setup script or CI step that ran it believed `urzua` was installed. Both commands now save the script with `curl -o urzua-install.sh` and run it with `&& sh urzua-install.sh`, so a failed download exits `22`.

## Why nothing caught it

`scripts/install-test.sh` ran the installer itself with stubbed downloads, and nothing examined the commands the documentation tells a person to run. It now fails if any documented command pipes into `sh` or `bash`.

## References

`docs/reference/install.md`; the `docs: no documented command pipes a download into a shell` check in `scripts/install-test.sh`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: scripts/install-test.sh@sha256:deea59418d7d Reason: Pin the install-script test that fails when a documented command pipes a download into a shell **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-23 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
> | 2026-11-23 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `scripts/install-test.sh` deea59418d7d -> 23a366374a00. Reason: scripts/install-test.sh: the no-pipe scan's regex also matches a pipe into sudo and into a path-qualified or dash/zsh shell; no other line changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-23 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `scripts/install-test.sh` 23a366374a00 -> cd8b216d2a19. Reason: scripts/install-test.sh: the no-pipe scan also matches a pipe continued onto the next line, with a comment saying why a bare trailing pipe is not matched; no other line changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
