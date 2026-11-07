---
default: patch
closes: []
---

`urzua init` no longer writes a config that cannot load. It never proposes a hidden directory such as
`.changeset`, a directory none of whose files has a header, or a directory of headed pages that carry no
`status` field; it names the ones it left out, gives every type a name that is a legal prefix, drops a
type whose prefix another already holds, and refuses to write a config that does not parse.
`init --dry-run` now works when a config already exists. `harness install` reports a refused install
honestly (every item `would-*`, nothing `created`, `merged` or `written`) and warns, as `doctor` does,
about a hand-copied first-generation editor hook wired beside the bundle's. `doctor` decides that a git
hook or workflow runs urzua from an actual call, not from the word in a comment, and no longer knows a
`make records` target. A tracked record deleted from the working tree is judged as removed and named in a
notice, where it used to make every `check` exit 2. A `--rules` selection no longer reads the source
files under `back_pointer_paths` unless a selected rule needs them.
