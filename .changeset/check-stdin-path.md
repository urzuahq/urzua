---
default: minor
---

`urzua check --stdin-path <path>` reads the proposed content of one record from stdin and judges it against every enabled rule that can judge one record without writing, comparing a status change or a new revision-log row with the committed copy of the record, reporting on that record alone (`scope.source` is `stdin`). A rule that cannot report on a record is listed `not-applicable`, and the hook's message names those rules instead of claiming every rule ran. The editor hook now relays it: an unwaived error the edit introduced blocks, a warning is shown, an error already on the file is listed and does not block, and the hook says plainly when it did not judge.
