# The board

`urzua board` shows every record in one page, for a person. Every other command prints JSON for an agent and
keeps doing so; this one writes a file or answers a request, and its stdout is still JSON.

```sh
urzua board
urzua board --out board.html
```

The first writes `.urzua/cache/board.html` (derived state, ignored by git); the second the path you name. Either
prints `status`, `file`, `bytes`, `record_count` and `read`. The file is one complete HTML document: its style and
script are inline, it makes no network request of any kind, and its `Content-Security-Policy` allows nothing but
those two inline blocks, by hash. Open it from disk.

## Serving it

```sh
urzua board --serve --port 8787
```

It prints one JSON line (`{"status": "ok", "listening": "http://127.0.0.1:8787/", "address": ..., "port": ...}`) when
it is listening, then runs in the foreground until interrupted. Without `--port` it takes a free port and says which.

- It listens on 127.0.0.1 only and answers `GET` only. Any other method gets `405`, any path but `/` (and
  `/record/<id>`, below) `404`, a request line or head that is too long `400`. It serves no file from disk, and
  a client that stalls is dropped after a few seconds.
- It re-reads the records on every request, so a second request shows a record changed since the first. Every
  response carries `Cache-Control: no-store`.
- It does not check the `Host` header, since a proxy presents its own name; every address in the page is relative.
- It writes nothing. To expose it to another device, put a proxy in front of the port.
- Interrupt it (Ctrl-C or `SIGTERM`) to stop it. It holds no state, so there is nothing to flush; the process ends
  by the signal's default action.

Each request runs the same read as the file: the records, the export's view model and a `urzua check`. On a large
corpus that takes as long as `urzua check` does.

## What the page shows

In this order:

1. **What needs attention.** *Waiting on a person*: records whose current status has a declared transition
   (`statuses.transitions`) to a status that an `authority.gates` entry for their type names. A type with a gate and no
   declared transitions lists nothing, and a line says its next move cannot be read. Then *under way*, then *ready*.
2. **The read stamp**: the commit, the branch, whether any record file has uncommitted changes, and the time, and that
   the page shows the working tree. And one line from `urzua check`: its status and the unwaived errors, warnings
   and infos.
3. **Work**: the records of the types in `roadmap.work_types` (every type when it is absent) that are not finished or
   dropped. Where a type declares a [`part_of` field](config.md#containment-part_of) they are a tree: each parent a
   collapsible section with its own status, the count of its children by state beside it, and a mark where its status and
   its children disagree. Records with no parent are in a group of their own, and a record whose parent is
   shown elsewhere (finished, dropped, of another kind or unreadable) says so. Without containment, work is flat groups by
   type.
4. **Other records**: the other types, as one compact row per record (id, title, status, how many records refer to it),
   grouped by type.
5. **Finished and dropped**, collapsed under their count.
6. **Headers that could not be read**, shown with their text; their state and links are unknown.

Each record shows its id, title, status and state as a word and a mark (finished, dropped, under way, not started; the
mark *waiting* beside it when it waits on records, named as links; *ready*), its checklist count where its type
declares one, the fields its type gives declared values (`field_values`), and its reference fields as links within the
page. No other header field is shown. Opening a record shows what refers to it and its text, as plain text. Nothing is
left off the page: its entries number as many as `urzua export` returns records.

A search over id and title and a filter by state run as a small inline script. With script disabled the page reads
the same, without them. It is readable at 360 px wide, and has a light and a dark palette chosen by the viewer's
setting. The states are written as words, so nothing depends on colour.

## Where the page reads from

The page is a template over [`urzua export --format json`](commands.md#export): its states, waits, tree, counts,
references and read stamp are keys of the export, computed once. It reads the configuration's declarations and no
name: no type, status or workflow word is in it. A repository that wants a different view builds it from the
export, and gets the same answers the page shows.

## Size

Everything from a record that reaches the page is HTML-escaped; a record's text is shown as text, not rendered, and
no address is made from a record's content. A repository whose records hold more than 2 MiB of text between them gets
a page without the text: a served page links each record's text at `record/<id>` (plain text), and a file says that
the text is left out. This repository's records (about 720) make a page of about half a megabyte that way.
