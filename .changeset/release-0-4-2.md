---
default: patch
closes: []
---

0.4.1's macOS binaries could not be built because the CI runner image it used was retired, so no binaries were attached. 0.4.2 publishes the same changes with Linux and Apple Silicon macOS binaries; install 0.4.2. Intel Macs build from source with `cargo install`.
