---
default: minor
closes: []
---

Add an opt-in `ignore_marker` to `embodiment.back-pointer-resolution` and `embodiment.back-pointer-decision-accepted`, the same option `boundary.published-path-reference` already takes. Declare the text once and a line carrying it, such as a record-shaped line inside a multi-line string in a test file, is no longer read as a back-pointer: it is not reported as citing a missing record, not judged for the cited record's status, and not counted as evidence. A real citation on any other line of the file still counts. The count of excused lines is shown in the rule's population (`excused_lines`) and in an `info` notice, and a marker on a line with no back-pointer is reported. Configurations that declare no marker are unchanged.
