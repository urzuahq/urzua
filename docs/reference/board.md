# The board

`urzua board` shows every record in one page, for a person. Every other command prints JSON for an agent and
keeps doing so; this one writes a file or answers a request, and its stdout is still JSON.

```sh
urzua board
urzua board --out board.html
urzua board --prompt
```

The first writes `.urzua/cache/board.html` (derived state, ignored by git); the second the path you name. Either
prints `status`, `file`, `bytes`, `record_count`, `read` and any `notices` (see *About this board* below). The file is one complete HTML document: its style and
script are inline, it makes no network request of any kind, and its `Content-Security-Policy` allows nothing but
those two inline blocks, by hash, and the page's own icon as a `data:` address. Open it from disk. The third prints `status` and `prompt` and writes no file; see
*Build your own view*.

The board is the plain default view, laid out only from the configuration: it works with any record types, statuses and workflow, and has no opinion about
them. Anything richer is built from [`urzua export --format json`](commands.md#export), which is the stable input for a custom view.

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
swaps the content in place, keeping the search text, the state filter, the open rows and the scroll position. The top bar shows
*updated N s ago* (the exact time in its tooltip) and a *Refresh* button; if a request fails the page keeps its content and says
*not updating: last read <time>* until one succeeds. A static file does not update: it says when it was generated.

On this repository's 729 records in a release build, `/` takes 1.7 s the first time and 0.04 s from the cache, `/record/<id>` 0.18 s cold and 0.04 s warm, and `/stamp` about 0.035 s; most of the page's build is judging the checklist items. `urzua check` takes ten times as long, so the served page does not run it:
it links `GET /check`, which computes the summary for that request and says when it did and how long it took.
The file `urzua board` writes carries the summary, computed when it was written.

## What the page shows

A small web application in one file, in a system font stack: no framework, no web font, no external asset.

- **Top bar** (stays in view as the page scrolls): the urzua mark and name, a search box over id and title, where the records were read from
  in monospace (branch, short commit, and that the page shows the working tree, with a note when a record file has uncommitted changes and,
  separately, when the configuration file has: the configuration changes what the page shows even when no record did; the key is `null` in the
  export when the file cannot be placed inside the repository), when, and the actions: *Build your own view*, *About this board* (the *i* button)
  and a light/dark switch. A served page shows *updated N s ago* (the exact time in its tooltip) and a *Refresh* button; a file says when it was
  generated. Under 460 px the bar is the mark, the search and the actions on one line and the read stamp on the lines below; nothing is dropped.
- **Tabs** under the top bar: All, Needs you, Under way, Ready, and the states records are in, each with the number of records it selects, in a row that scrolls sideways
  on a phone without a scrollbar. From 1000 px a **sidebar** repeats the views and lists one entry for every record type the configuration declares, with its
  record count, none of them special; choosing one filters the page to it and the current choice is marked. Choosing a view or a type, and the search, narrow
  the page to the matching rows (a view with no match says *No record matches.*); *All* with an empty search shows the sections again.
- **Sections** are cards with a title and a count. A section with nothing in it is its title and one muted sentence, not an empty card.
- **Keys**: `/` focuses the search and `Esc` clears it (and closes an open panel).
- **Record details**: on a screen of 1000 px or more a row's *details* open in a panel on the right and the row stays in view and marked; below that the details open inside the row. The
  address `#r-<id>` (the anchor of every in-page link) scrolls to a row, opens the folds above it and, on a wide screen, its panel; `GET /record/<id>` is unchanged.
- **Light and dark** follow the viewer's setting. The switch in the top bar overrides it and remembers the choice in the browser's local storage; where that storage is unavailable
  the switch still works for the page's life and nothing is remembered.
- **Footer**: the version, and links to *About this board* and *Build your own view*.

Under the tabs, one line from `urzua check` (its status and the unwaived errors, warnings and infos) in the file; a served page links `/check`
for it instead.

The only colour on a row is the dot for its state. The page's one accent is blue: the active tab, the focus ring, links and the primary button. The mark is five squares in a U
(blue, green, yellow, orange, red); the page uses it as its logo and its icon and nowhere else, and yellow is never text. Text meets WCAG AA contrast in both themes.

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
2. **Under way**, then **Ready**: short lists of links. *Needs you*, *Under way* and *Ready* each show eight rows, then one muted *N more* row that opens in place. *Ready* lists leaves only: a parent that has not begun is `ready` in the export, but the board shows it in Work with its count, and the *Ready* count in the filter is the same leaves. A record under way is never ready, so no record is in both lists. Each row also says what the record waits on and, where it is not drawn inside its parent's row, what it is part of. A section with no row says so in one sentence. Which types declare
a `statuses.progress` is in *About this board*, not here.
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
has begun, so its records that are neither finished nor dropped are *open*. *Not started* is used only for a type that declares a progress
set, for a status outside it. Nothing is left off the page: its rows number as many as `urzua export` returns records.

## About this board

A panel (the *i* button, or *About this board* in the footer) says in plain sentences how each part of the page follows from the configuration, so
nothing on the board has to be taken on trust: which declaration makes the tree (`part_of`), what *under way* is (`statuses.progress`), what makes a record
appear under *Needs you* (`statuses.transitions` and an `authority.gates` entry), what *ready* means (`roadmap.pairs`, `roadmap.work_types`), and then, for every
type, what this repository's configuration does and does not declare: whether it is a work type, whether it declares a progress set, where its tree comes from, what it
waits through, which gate reserves a move for which group.

A type that declares statuses beyond its `initial`, `success` and `failure` ones (a type with only those has nothing to declare, and a type that declares no `initial` is never named:
without it the board cannot tell a start status from an in-between one, so it does not guess) but no `statuses.progress` is told so there, with
those statuses: until a progress set is declared they show as open and a parent's state cannot disagree with its children. The same fact is a `notice` (`board.progress`, with the `type` and its
`statuses`) in the JSON `urzua board` prints when it writes a file.

## Build your own view

The board is deliberately plain. To see the records another way, the *Build your own view* button (top bar and footer) opens a panel with a prompt to paste into a coding agent, and a *Copy* button
(the clipboard where the browser allows it, otherwise the text selected for copying). `urzua board --prompt` prints the same text without the page:

```sh
urzua board --prompt
```

It prints `status` and `prompt`, and writes nothing. The prompt is written from this repository's configuration, not fixed: it says to run `urzua export --format json` and names `--no-text`,
`--no-raw-text` and `--no-checklist-items`; lists the top-level keys and the keys of each record that the export carries; lists the record types the configuration declares, each with its
directory, its fields, the values it gives a field and its statuses (`initial`, `success`, `failure`, `progress`); says that `urzua board` is the plain default and the export is the stable input;
and ends with the line *Build me a view that ...* for the person to finish. A test holds the key lists to a real export, both ways, so they cannot drift from it.

The search, the tabs, the sidebar, the panels and the theme switch are a small inline script; with script disabled the page reads the same, without them (*About this board* and *Build your own view* are then the last two cards on the page).
It is readable at 360 px wide, with no sideways scroll.

## Where the page reads from

The page is a template over [`urzua export --format json`](commands.md#export): its states, waits, tree, counts,
references and read stamp are keys of the export, computed once. It reads the configuration's declarations and no
name: no type, status or workflow word is in it. A repository that wants a different view builds it from the
export, and gets the same answers the page shows.

## Size

Everything from a record that reaches the page is HTML-escaped; a record's text is shown as text, not rendered, and
the only address a page carries is a record's `link`, built from the configuration's `export.link_prefix` and the record's path (percent-encoded), never from a record's content. A repository whose records hold more than 2 MiB of text between them gets
a page without the text and the checklist items: a served page links each record's text at `record/<id>` (plain text), and a file says that
the text is left out. This repository's records (about 730) make a page of about 0.8 megabyte; the page's own frame (style, script, panels) is about 22 KB of that.
