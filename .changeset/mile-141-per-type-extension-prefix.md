---
default: minor
closes: [MILE-141]
---

`header.field-set-consistency` takes the optional `extension_prefixes` option, a map from a record type to the prefix its extension header keys begin with, in place of the fixed `x-`, so a type can admit `ext-owner` while an unprefixed unknown key still fails. A type the map does not name keeps `allow_extension_prefix` exactly as before. A prefix that is empty, holds whitespace or starts a field the type declares is refused at load, as is a type that is not declared or declares no `known_fields`, and the option is off unless declared.
