---
default: minor
---

`urzua check --stdin-path <path>` reads the proposed content of one record from stdin and judges it against every enabled rule without writing, reporting on that record alone (`scope.source` is `stdin`). A rule that cannot report on a record is listed `not-applicable`. The editor hook now relays it: an unwaived error the edit introduced blocks, a warning is shown, an error already on the file is listed and does not block, and the hook says plainly when it did not judge.
