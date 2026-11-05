---
default: patch
closes: [BUG-188]
---

`urzua new` writes the H1 id padded exactly as the file name is, so a type whose identity pattern requires
zero-padded numbers no longer gets an unpadded H1 beside a padded file name. Header fields a type's template
lists that the type declares but does not require are kept on the new record, blank.
