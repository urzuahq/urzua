---
default: minor
closes: [MILE-44]
---

`urzua check` findings can now carry a `suggested_action`: what to do about the finding, as data an
agent can act on without reading `message`. It names a `kind` (`edit-field`, `edit-section`,
`edit-config`, `add-record` or `run-command`), an `op` where the kind has several forms (`set`,
`remove`, `append`, `rename`, `add`), a `target` (the file, field, section, configuration key path or
command) and, only where the tool can derive one, a `value`. A key that was not computed is omitted,
never `null`. A finding with no sensible action, such as an identifier collision, has no
`suggested_action` key. A `run-command` action is offered only when running that command would
remove the finding.

`urzua rules` lists, for every rule, each finding variant with its action or the reason it offers
none. A consumer should treat an unrecognised `kind` or `op` as no guidance and read `message`.
