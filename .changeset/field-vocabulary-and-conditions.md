---
default: minor
closes: []
---

A record type can declare the values an ordinary header field may hold, `field_values: {Kind: [feature, bug, chore]}`, and the new rule `field.value-not-declared` reports a record whose field holds anything else, naming the field, the value and the allowed values. Matching is exact, each element of a list value is checked, and a blank value is left to the presence rules. The rule is on by default at `error` once any type declares `field_values`; a configuration without it reports exactly as before. A `statuses.require` or `statuses.forbid` entry can also name `when_field: {Kind: [bug]}`, alone or beside `when`, so a field can be required or forbidden by another field's value; the finding names the triggering field and value. A condition on an undeclared field, on the status field or on a value outside the field's vocabulary is a load error, and existing status-keyed entries keep their meaning.
