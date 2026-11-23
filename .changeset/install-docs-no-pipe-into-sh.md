---
default: patch
closes: [BUG-218]
---

The documented install commands no longer pipe the install script into `sh`. They save it with `curl -o urzua-install.sh` and run it only if the download succeeded, so a failed download exits non-zero (`22`) instead of exiting `0` with nothing installed.
