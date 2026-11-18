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

- It listens on 127.0.0.1 only and answers `GET` only. Any other method gets `405`, any path but `/`, `/stamp`, `/check` and
  `/record/<id>` `404`, a request line or head that is too long `400`. It serves no file from disk, and
  a client that stalls is dropped after a few seconds.
- It re-reads the records on every request, so a second request shows a record changed since the first. Every
  response carries `Cache-Control: no-store`.
- It does not check the `Host` header, since a proxy presents its own name; every address in the page is relative.
- It writes nothing. The address is local only: to reach it from another device, put a proxy in front of the port.
- Interrupt it (Ctrl-C) or stop it (`SIGTERM`) to end it. It holds no state, so there is nothing to flush, and the
  process ends by the signal's default action rather than by exiting 0 (the standard library installs no handler, and
  this crate forbids unsafe code). A service manager treats `SIGTERM` as a clean stop of a service it asked to stop.

As a user service, in `~/.config/systemd/user/urzua-board.service`, with the repository as the working directory:

```text
[Service]
WorkingDirectory=%h/code/your-repository
ExecStart=/usr/local/bin/urzua board --serve --port 8787
```

Then `systemctl --user enable --now urzua-board`.

A served page updates itself. Every 15 seconds while its tab is visible (and once when it becomes visible again) it asks
`GET /stamp`, which answers the commit, the branch and a digest of every record file's and the configuration's path, size and
modification time without reading a record (about 20 ms on this repository's 722). When the digest differs it fetches `/` and
swaps the content in place, keeping the search text, the state filter, the open rows and the scroll position. The header shows
*updated N s ago* (the exact time in its tooltip) and a *Refresh* button; if a request fails the page keeps its content and says
*not updating: last read <time>* until one succeeds. A static file does not update: it says when it was generated.

A request for `/` reads the records and builds the page from the export alone: on this repository's 722 records
that is a second or two in a release build. `urzua check` takes ten times as long, so the served page does not run it:
it links `GET /check`, which computes the summary for that request and says when it did and how long it took.
The file `urzua board` writes carries the summary, computed when it was written.

## What the page shows

A plain, dense list. The header is one line: the name, then the read stamp in monospace (branch, short commit, time, and
that the page shows the working tree, with a note when a record file has uncommitted changes). Under it, one line from
`urzua check` (its status and the unwaived errors, warnings and infos) in the file; a served page links `/check`
for it instead. Then a search box over id and title, and a filter by state with the count each selects: All, Needs you,
Under way, Ready, and the states records are in (a scrolling row at phone width).

Every record is one row: its id, its title (cut with an ellipsis, never wrapped), and at the right small muted
text: an optional detail, a dot for its state, and its status exactly as written. The dot is the only colour on a row:
finished green, under way amber, ready blue, dropped red, open or not started a hollow ring; each state is also a word, in the
dot's tooltip, in the filter labels and in the status beside it. The details, in the order they give way for room, are
*waits on* ids (amber, as in-page links), a chip for each field the type gives declared values (`field_values`),
the checklist count (`2/3`), how many records refer to it, and a parent's *N of M done* (and *disagrees* where its status
and its children's do not agree). Below 460 px only the dot and the status remain. Tapping a row opens it in place, with no script: its
reference fields as links, what refers to it, its checklist items, what it waits on in full, and its text (a link to it
on a served page).

In this order:

1. **Needs you**: records whose current status has a declared transition (`statuses.transitions`) to a status that an
   `authority.gates` entry for their type names, as *From → To* with the group as a chip. A type with a gate and no
   declared transitions lists nothing, and a line says its next move cannot be read.
2. **Under way**, then **Ready**: short lists of links, with the rest under *N more*.
3. **Work**: the records of the types in `roadmap.work_types` (every type when it is absent). Where a type declares a
   [`part_of` field](config.md#containment-part_of) they are a tree: a parent is a raised row with a disclosure triangle,
   its children are indented 18 px a level, and a parent starts open only when a child is under way, ready or waiting. A
   parent's finished and dropped children are one muted row, *N finished, collapsed*, that opens in place. Records with
   no parent are a list of their own; a record whose parent is shown elsewhere (finished, dropped, of another kind or
   unreadable) is another. Finished and dropped work whose parent is not in the tree is a last list. Without containment,
   work is one list per type.
4. **One list per other type**, labelled with the type's name as the configuration declares it. A type with more than ten
   records folds its finished and dropped ones under a count.
5. **Headers that could not be read**, with their text; their state and links are unknown.

A containment field and the field a record states its waits through appear once, as *part of* and *waits on*, not
again among the reference fields.

The state comes only from what the configuration declares. A type that declares no `statuses.progress` cannot say work
has begun, so its records that are neither finished nor dropped are *open*, and the *Under way* label says once which
types declare no progress set, so nothing can be listed for them: add `statuses.progress` under the type's `statuses` to
name the statuses that mean work has begun. *Not started* is used only for a type that declares a progress set, for a
status outside it. Nothing is left off the page: its rows number as many as `urzua export` returns records.

The search and the state filter are a small inline script; with script disabled the page reads the same, without them.
It is readable at 360 px wide, with no sideways scroll, and has a light and a dark palette chosen by the viewer's setting.

## Where the page reads from

The page is a template over [`urzua export --format json`](commands.md#export): its states, waits, tree, counts,
references and read stamp are keys of the export, computed once. It reads the configuration's declarations and no
name: no type, status or workflow word is in it. A repository that wants a different view builds it from the
export, and gets the same answers the page shows.

## Size

Everything from a record that reaches the page is HTML-escaped; a record's text is shown as text, not rendered, and
no address is made from a record's content. A repository whose records hold more than 2 MiB of text between them gets
a page without the text and the checklist items: a served page links each record's text at `record/<id>` (plain text), and a file says that
the text is left out. This repository's records (about 720) make a page of about half a megabyte that way.
