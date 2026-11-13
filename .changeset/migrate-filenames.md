---
default: minor
closes: []
---

New command `urzua migrate filenames`. It renames every record file whose name does not match its
type's filename convention, the declared `identity.pattern` or `TYPE-NNNN-slug.md` when none is
declared, keeping the directory, number and slug so a record's id and `Stable-Id` do not change. It
rewrites relative links and path mentions of the renamed files in records and other tracked markdown
files, and lists in `unrewritten` any it finds in other tracked files. A file it cannot rename safely
(no number, a name the pattern rejects, a new name that already exists) is listed in `skipped` and
left alone. It prints one JSON document and writes nothing unless `--apply` is given; a second applied
run changes nothing.
