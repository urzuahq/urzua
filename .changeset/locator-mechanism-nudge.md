---
default: minor
---

The opt-in rule `embodiment.locator-on-back-pointer-path` warns on a `Realized-by` locator that names a file the back-pointer scan already reads, where an `Implements:` or `Verifies:` comment would say the same thing and is found automatically. It flags a `code:` locator under `back_pointer_paths` and a `test:` locator under `test_paths`, leaves what a comment cannot say (a `spec:` locator, a `#scope` or an `@sha256` pin), and offers an `edit-field` action that drops the locator from `Realized-by`.
