---
default: minor
---

Adds `block_on: introduced | all`, a top-level configuration key that sets which errors the editor hook blocks on. `introduced`, the default and what a configuration without the key gets, blocks only on an error the edit introduces, as before. `all` blocks on any error in the edited record, including one it already has, and skips the second `check` run. The named override and its log work under both values. Any other value is a configuration load error that names the key and the value.
