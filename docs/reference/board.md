# The board

`urzua board` shows every record in one page, for a person. Every other command prints JSON for an agent and
keeps doing so; this one writes a file or answers a request, and its stdout is still JSON.

```sh
urzua board
urzua board --out board.html
```

The first writes `.urzua/cache/board.html` (derived state, ignored by git); the second the path you name. Either
prints `status`, `file`, `bytes`, `record_count`, `read` and any `notices` (see *Under way* below). The file is one complete HTML document: its style and
script are inline, it makes no network request of any kind, and its `Content-Security-Policy` allows nothing but
those two inline blocks, by hash. Open it from disk.

## Serving it

```sh
urzua board --serve --port 8787
```

It prints one JSON line (`{"status": "ok", "listening": "http://127.0.0.1:8787/", "address": ..., "port": ...}`) when
it is listening, then runs in the foreground until interrupted. Without `--port` it takes a free port and says which.

- It listens on 127.0.0.1 only and answers `GET` only. Any other method gets `405`, any path but `/`, `/stamp`, `/check` and
  `/record/<id>` `404`, a request line or head that is too long `400`. It serves no file from disk. Each connection is handled on
  its own thread, up to 32 at once; past that the answer is `503` at once, and a client that has not sent its request head
  in 2 seconds is dropped, so idle clients cannot starve anyone.
- The page and the export behind it are cached against the `/stamp` digest: an unchanged repository answers `/` from
  memory, and a change to a record file or the configuration makes the next request rebuild it. `/record/<id>` reads that
  one record's text without building the page. Every response carries `Cache-Control: no-store`.
- `/check` runs one `urzua check` at a time. A request that arrives while a run is in flight waits for that run's result, the
  result is kept until the digest changes, and no `/check` run delays `/` or `/stamp`.
- It does not check the `Host` header by default, since a proxy presents its own name; every address in the page is relative.
  With `--allowed-host <name>` (repeatable) a request whose `Host` is none of those names, `localhost` or `127.0.0.1` (any
  port) is answered `403`. Use it whenever the address is reached through a proxy with a known name.
- It writes nothing, and no path changes anything: every one is a read. No `Access-Control-Allow-Origin` is sent, so an
  ordinary cross-origin page cannot read its responses. What is left without `--allowed-host`: while the server runs, a web
  page the person visits that rebinds its own name to the local address can read the board and every record's text, because
  the server answers whatever `Host` it is given. The address is local only: to reach it from another device, put a proxy in
  front of the port.
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
modification time without reading a record (about 35 ms on this repository's 729). When the digest differs it fetches `/` and
swaps the content in place, keeping the search text, the state filter, the open rows and the scroll position. The header shows
*updated N s ago* (the exact time in its tooltip) and a *Refresh* button; if a request fails the page keeps its content and says
*not updating: last read <time>* until one succeeds. A static file does not update: it says when it was generated.

On this repository's 729 records in a release build, `/` takes 1.7 s the first time and 0.04 s from the cache, `/record/<id>` 0.18 s cold and 0.04 s warm, and `/stamp` about 0.035 s; most of the page's build is judging the checklist items. `urzua check` takes ten times as long, so the served page does not run it:
it links `GET /check`, which computes the summary for that request and says when it did and how long it took.
The file `urzua board` writes carries the summary, computed when it was written.

## What the page shows

A plain, dense list. The header is one line: the name, then the read stamp in monospace (branch, short commit, time, and
that the page shows the working tree, with a note when a record file has uncommitted changes and, separately, when the
configuration file has (the key is `null` in the export when the file cannot be placed inside the repository): the configuration changes what the page shows even when no record did). Under it, one line from
`urzua check` (its status and the unwaived errors, warnings and infos) in the file; a served page links `/check`
for it instead. Then a search box over id and title, and a filter by state with the count each selects: All, Needs you,
Under way, Ready, and the states records are in (a scrolling row at phone width).

Every record is one row: its id, its title (one line cut with an ellipsis, wrapped to up to three lines at phone width), and at the right small muted
text: an optional detail, a dot for its state, and its status exactly as written. The dot is the only colour on a row:
finished green, under way amber, ready blue, dropped red, open or not started a hollow ring; each state is also a word, in the
dot's tooltip, in the filter labels and in the status beside it. The details, in the order they give way for room, are
*waits on* ids (amber, as in-page links), a chip for each field the type gives declared values (`field_values`),
the checklist count (`2/3`), how many records refer to it, and a parent's *N of M done* (and *disagrees* where its status
and its children's do not agree). Below 460 px nothing is dropped: the title wraps to up to three lines and the details, the dot and the status move to a line under it. A parent's row, and its triangle, opens and closes its children only. A record's own detail is a second, separate
disclosure at the right of the row (the word *details*, or › on a phone), closed until asked for, for a parent and a leaf
alike, and it works without script: its reference fields as links, what refers to it, its checklist items, what it
waits on in full, its text (a link to it on a served page) and, when the configuration declares an `export.link_prefix`, a link to the record's file (the export's `link`, opened in a new tab). Nothing on the page shows a record's text until that
record's own detail is opened.

In this order:

1. **Needs you**: records whose current status has a declared transition (`statuses.transitions`) to a status that an
   `authority.gates` entry for their type names, as *From → To* with the group as a chip. A type with a gate and no
   declared transitions lists nothing, and a line says its next move cannot be read.
2. **Under way**, then **Ready**: short lists of links. *Needs you*, *Under way* and *Ready* each show eight rows, then one muted *N more* row that opens in place. *Ready* lists leaves only: a parent that has not begun is `ready` in the export, but the board shows it in Work with its count, and the *Ready* count in the filter is the same leaves. A record under way is never ready, so no record is in both lists. Each row also says what the record waits on and, where it is not drawn inside its parent's row, what it is part of. A type that declares statuses between its start and its end but no `statuses.progress` is named in one muted line under *Under way*, and the same fact is a `notice` (`board.progress`, with the `type` and its `statuses`) in the JSON `urzua board` prints.
3. **Work**: the records of the types in `roadmap.work_types` (every type when it is absent). Where a type declares a
   [`part_of` field](config.md#containment-part_of) they are a tree: a parent is a raised row with a disclosure triangle,
   its children are indented 18 px a level, and a parent's children list starts open only when a child is under way, ready or waiting. A
   parent's finished and dropped children are one muted row that opens in place and names what it hides: *N finished, collapsed: ids (+N more past five); N records under them*. Records with
   no parent are a list of their own; a record whose parent is shown elsewhere (finished, dropped, of another kind or
   unreadable) is another. Finished and dropped work whose parent is not in the tree is a last list. Without containment,
   work is one list per type.
4. **One list per other type**, labelled with the type's name as the configuration declares it. A type with more than ten
   records folds its finished and dropped ones under a count.
5. **Headers that could not be read**, with their text; their state and links are unknown.

A containment field and the field a record states its waits through appear once, as *part of* and *waits on*, not
again among the reference fields.

The state comes only from what the configuration declares. A type that declares no `statuses.progress` cannot say work
has begun, so its records that are neither finished nor dropped are *open*. The *Under way* label names each such type that
declares a status which is none of `initial`, `success` and `failure` (a type with only those has nothing to declare, and a type that declares no `initial` is never named: without it the board cannot tell a start status from an in-between one, so it does not guess), with
those statuses, and says that a parent's state cannot disagree with its children until a progress set is declared: add `statuses.progress` under the type's `statuses` to
name the statuses that mean work has begun. *Not started* is used only for a type that declares a progress set, for a
status outside it. A parent that *disagrees* is the one `check` reports, when the repository enables `status.children-agreement` ([config](config.md#containment-part_of)), so the same finding can fail a build instead of waiting to be seen on the page. Nothing is left off the page: its rows number as many as `urzua export` returns records.

The search and the state filter are a small inline script; with script disabled the page reads the same, without them.
It is readable at 360 px wide, with no sideways scroll, and has a light and a dark palette chosen by the viewer's setting.

## Where the page reads from

The page is a template over [`urzua export --format json`](commands.md#export): its states, waits, tree, counts,
references and read stamp are keys of the export, computed once. It reads the configuration's declarations and no
name: no type, status or workflow word is in it. A repository that wants a different view builds it from the
export, and gets the same answers the page shows.

## Size

Everything from a record that reaches the page is HTML-escaped; a record's text is shown as text, not rendered, and
the only address a page carries is a record's `link`, built from the configuration's `export.link_prefix` and the record's path (percent-encoded), never from a record's content. A repository whose records hold more than 2 MiB of text between them gets
a page without the text and the checklist items: a served page links each record's text at `record/<id>` (plain text), and a file says that
the text is left out. This repository's records (about 720) make a page of about half a megabyte that way.
