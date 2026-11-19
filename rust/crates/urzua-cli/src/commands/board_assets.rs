//! The page's own style and script. Fixed text: nothing from a record is ever
//! put into either, which is what lets the page's Content-Security-Policy name
//! their hashes and allow nothing else to run or load.

pub(super) const STYLE: &str = r#"
:root {
  color-scheme: light;
  --bg: #F7F7F8; --card: #FFFFFF; --raised: #F4F4F5; --hover: #F1F1F3; --border: #E4E4E7; --fg: #18181B; --muted: #63636B;
  --green: #12B550; --amber: #FF7A1A; --blue: #0F5BF0; --red: #E5233D; --wait: #A84A07; --link: #0F5BF0;
  --radius: 8px; --stick: 92px; --s1: 4px; --s2: 8px; --s3: 12px; --s4: 16px; --s5: 24px;
  --sans: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    color-scheme: dark;
    --bg: #0B0B0C; --card: #131315; --raised: #1A1A1D; --hover: #1C1C1F; --border: #28282B; --fg: #EDEDED; --muted: #9A9AA2;
    --wait: #FF7A1A; --link: #7AA2FF;
  }
}
:root[data-theme="dark"] {
  color-scheme: dark;
  --bg: #0B0B0C; --card: #131315; --raised: #1A1A1D; --hover: #1C1C1F; --border: #28282B; --fg: #EDEDED; --muted: #9A9AA2;
  --wait: #FF7A1A; --link: #7AA2FF;
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; scroll-padding-top: calc(var(--stick) + 8px); }
body { margin: 0; background: var(--bg); color: var(--fg); font: 13px/1.45 var(--sans); overflow-wrap: anywhere; }
a { color: var(--link); text-decoration: underline; text-decoration-color: var(--border); text-underline-offset: 2px; }
a:hover { text-decoration-color: currentColor; }
code, pre { font-family: var(--mono); font-size: 12px; }
:focus-visible { outline: 2px solid var(--blue); outline-offset: 1px; }
summary:focus-visible, .tab:focus-visible, .si:focus-visible { outline-offset: -2px; }
.skip { position: absolute; left: var(--s2); top: -40px; z-index: 40; background: var(--card); padding: var(--s2) var(--s3); border-radius: var(--radius); }
.skip:focus { top: var(--s2); }
.js-only { display: none !important; }
.js .js-only { display: inline-flex !important; }
.sticky { position: sticky; top: 0; z-index: 20; background: var(--card); border-bottom: 1px solid var(--border); }
.top {
  display: grid; grid-template-columns: auto minmax(0, 24rem) minmax(0, 1fr) auto;
  grid-template-areas: "brand find stamp acts"; align-items: center; gap: var(--s3); height: 52px; padding: 0 var(--s4);
}
.brand { grid-area: brand; display: flex; align-items: center; gap: var(--s2); color: var(--fg); text-decoration: none; }
.brand svg { height: 18px; width: auto; display: block; }
.brand b { font-size: 15px; font-weight: 600; letter-spacing: -.01em; }
.brand .sub { color: var(--muted); font-size: 14px; }
.find { grid-area: find; position: relative; display: none; min-width: 0; }
.js .find { display: block; }
.find input {
  width: 100%; font: inherit; color: var(--fg); background: var(--bg); border: 1px solid var(--border);
  border-radius: var(--radius); padding: 6px 30px 6px 10px; height: 32px;
}
.find input::placeholder { color: var(--muted); }
.find kbd {
  position: absolute; right: 8px; top: 50%; transform: translateY(-50%); font: 11px var(--mono); color: var(--muted);
  border: 1px solid var(--border); border-radius: 4px; padding: 0 5px; pointer-events: none;
}
.stamp {
  grid-area: stamp; display: grid; grid-template-columns: minmax(0, 1fr) auto; grid-template-areas: "meta meta" "upd rf";
  justify-items: end; align-items: center; column-gap: var(--s2); min-width: 0; line-height: 1.35;
}
.meta { color: var(--muted); font: 11px var(--mono); }
#meta { grid-area: meta; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
#upd { grid-area: upd; justify-self: end; white-space: nowrap; }
.warn { color: var(--wait); }
.acts { grid-area: acts; display: flex; align-items: center; gap: var(--s2); }
.btn, .ib, .rf {
  font: inherit; font-size: 12px; color: var(--fg); background: var(--card); border: 1px solid var(--border);
  border-radius: var(--radius); height: 30px; padding: 0 10px; cursor: pointer; align-items: center; justify-content: center;
  gap: 6px; text-decoration: none; white-space: nowrap;
}
.btn, .ib { display: inline-flex; }
.btn:hover, .ib:hover, .rf:hover { background: var(--hover); }
.btn.go { background: var(--blue); border-color: var(--blue); color: #FFFFFF; }
.btn.go:hover { background: #0B4CCB; }
.ib { width: 30px; padding: 0; font-size: 14px; line-height: 1; }
.btn .gly { display: none; font: 600 12px var(--mono); }
.rf { display: none; grid-area: rf; color: var(--muted); height: 20px; font-size: 11px; padding: 0 8px; }
.js .rf { display: inline-flex; }
.rf:disabled { opacity: .6; cursor: default; }
.tabs { display: none; padding: 0 var(--s3); border-top: 1px solid var(--border); }
.js .tabs { display: block; }
.tabrow { display: flex; gap: 2px; overflow-x: auto; scrollbar-width: none; max-width: 100rem; margin: 0 auto; }
.tabrow::-webkit-scrollbar { display: none; }
.tab {
  flex: none; font: inherit; color: var(--muted); background: transparent; border: 0; border-bottom: 2px solid transparent;
  padding: 9px 10px 7px; cursor: pointer; white-space: nowrap; display: flex; align-items: center; gap: 6px;
}
.tab:hover { color: var(--fg); }
.tab i, .si i { font-style: normal; font-size: 11px; color: var(--muted); background: var(--raised); border-radius: 999px; padding: 0 6px; }
.tab.on { color: var(--fg); border-bottom-color: var(--blue); font-weight: 500; }
.app { display: grid; grid-template-columns: minmax(0, 1fr); gap: var(--s4); max-width: 64rem; margin: 0 auto; padding: var(--s4); }
.side { display: none; }
main { min-width: 0; }
.panel {
  background: var(--card); border: 1px solid var(--border); border-radius: var(--radius); position: sticky;
  top: calc(var(--stick) + var(--s4)); max-height: calc(100vh - var(--stick) - 32px); overflow: auto; align-self: start;
}
.ph { display: flex; align-items: baseline; gap: var(--s2); padding: var(--s3); border-bottom: 1px solid var(--border); position: sticky; top: 0; background: var(--card); }
.ph .id { min-width: 0; }
.ph .pt { flex: 1 1 0; min-width: 0; font-weight: 600; }
.ph .st { color: var(--muted); font-size: 12px; }
.panel .more { padding: var(--s3); max-width: none; }
.panel .more pre { max-height: none; }
.note { color: var(--muted); font-size: 12px; margin: 0 0 var(--s2); }
.card, .none { margin: 0 0 var(--s4); }
.card { background: var(--card); border: 1px solid var(--border); border-radius: 10px; overflow: hidden; }
.sh { display: flex; align-items: baseline; gap: var(--s2); padding: 10px var(--s3); border-bottom: 1px solid var(--border); }
.none .sh { border-bottom: 0; padding: 0 2px 4px; }
.card > .note { padding: var(--s2) var(--s3); margin: 0; }
h2 { font-size: 11px; font-weight: 500; letter-spacing: .06em; text-transform: uppercase; color: var(--muted); margin: var(--s4) var(--s2) var(--s1); }
h3 { font-size: 13px; font-weight: 600; margin: 0; }
h3.ty { font-family: var(--mono); font-size: 12px; }
h3 i { font-style: normal; font-weight: 400; margin-left: 6px; color: var(--muted); }
.empty { color: var(--muted); margin: var(--s5) 0; text-align: center; }
.none .empty { margin: 0 2px var(--s2); text-align: left; }
ul { list-style: none; margin: 0; padding: 0; }
.list li { border-top: 1px solid var(--border); min-width: 0; }
.list > li:first-child { border-top: 0; }
summary, .br, .rs {
  display: flex; align-items: center; gap: 10px; padding: 7px 10px; min-width: 0;
  list-style: none; cursor: pointer;
}
.br { cursor: default; }
summary::-webkit-details-marker { display: none; }
summary > *, .br > *, .rs > * { min-width: 0; }
.rs { cursor: default; padding-right: 76px; }
details.kd > .rs { cursor: pointer; }
li.rec { position: relative; }
.list li:hover > .rs, .list li:hover > details > .rs, .br:hover { background: var(--hover); }
.p > details > .rs { background: var(--raised); }
.p > details > .rs .t { font-weight: 600; }
.rec.sel > .rs, .rec.sel > details > .rs { background: var(--hover); }
.rec.sel::before, .si.on::before { content: ""; position: absolute; left: 0; top: 0; bottom: 0; width: 2px; background: var(--blue); }
.det > summary {
  position: absolute; top: 0; right: 0; padding: 7px 10px; color: var(--muted); font-size: 12px;
  cursor: pointer; list-style: none;
}
.det > summary:hover { color: var(--link); }
.det[open] > summary { color: var(--fg); }
.dn { display: none; }
.ch { flex: none; width: 10px; color: var(--muted); font-size: 10px; }
.tri::before { content: "\25B8"; }
details[open] > .rs > .tri::before { content: "\25BE"; }
.id { font: 12px var(--mono); color: var(--muted); flex: none; min-width: 8ch; }
.t { flex: 1 1 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.s { flex: none; max-width: 70%; display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--muted); }
.s .x { max-width: 34ch; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.st { white-space: nowrap; }
.d { width: 7px; height: 7px; border-radius: 50%; flex: none; background: var(--muted); }
.d.g { background: var(--green); }
.d.o { background: var(--amber); }
.d.b { background: var(--blue); }
.d.r { background: var(--red); }
.d.n { background: transparent; border: 1px solid var(--muted); }
.k { font: 11px var(--mono); color: var(--muted); border: 1px solid var(--border); border-radius: 4px; padding: 0 5px; flex: none; }
.w, .x.w a { color: var(--wait); }
.more { padding: 2px 10px 10px 30px; color: var(--muted); font-size: 12px; }
.more > div { margin: 2px 0; }
.more .item code { color: var(--muted); }
.more pre {
  margin: 6px 0 0; white-space: pre-wrap; color: var(--fg); background: var(--raised);
  border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 10px; max-height: 24rem; overflow: auto;
}
.fold > details > summary { color: var(--muted); font-size: 12px; }
.lv0 { --ind: 0px; } .lv1 { --ind: 18px; } .lv2 { --ind: 36px; } .lv3 { --ind: 54px; }
.lv4 { --ind: 72px; } .lv5 { --ind: 90px; } .lv6 { --ind: 108px; } .lv7 { --ind: 126px; } .lv8 { --ind: 144px; }
li[class*="lv"] > .rs, li[class*="lv"] > details > .rs { padding-left: calc(10px + var(--ind)); }
li[class*="lv"] > .det > .more { padding-left: calc(30px + var(--ind)); }
.about, .build { padding-bottom: var(--s3); }
.about > p, .build > p, .about > dl { margin: var(--s3) var(--s3) 0; color: var(--muted); }
.about dl { display: grid; grid-template-columns: minmax(8rem, max-content) 1fr; gap: var(--s2) var(--s3); }
.about dt { font: 12px var(--mono); color: var(--fg); }
.about dd { margin: 0; }
.about h4 { margin: var(--s4) var(--s3) 0; font-size: 12px; }
.x-close { display: none; margin-left: auto; }
.pop .x-close { display: inline-flex; }
#prompt {
  display: block; width: calc(100% - 24px); margin: var(--s3) var(--s3) 0; min-height: 15rem; resize: vertical; font: 12px/1.5 var(--mono);
  color: var(--fg); background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); padding: var(--s3);
}
.copyrow { display: flex; align-items: center; gap: var(--s3); margin: var(--s3) var(--s3) 0; }
.pop:focus { outline: none; }
.pop {
  position: fixed; z-index: 30; top: 8vh; left: 50%; transform: translateX(-50%); width: min(46rem, calc(100vw - 24px));
  max-height: 84vh; overflow: auto; margin: 0;
}
.scrim { position: fixed; inset: 0; z-index: 25; background: rgba(0, 0, 0, .45); }
.foot { max-width: 64rem; margin: 0 auto; padding: var(--s2) var(--s4) var(--s5); color: var(--muted); font-size: 12px; display: flex; flex-wrap: wrap; gap: var(--s2) var(--s4); }
.foot .fx { margin-left: auto; display: flex; gap: var(--s4); }
.si { position: relative; display: flex; width: 100%; align-items: center; justify-content: space-between; gap: var(--s2); font: inherit; color: var(--muted); background: transparent; border: 0; border-radius: 6px; padding: 5px var(--s2); cursor: pointer; text-align: left; }
.si:hover { background: var(--hover); color: var(--fg); }
.si.on { background: var(--raised); color: var(--fg); font-weight: 500; }
.si span { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
[hidden] { display: none !important; }
@media (min-width: 1000px) {
  .app { grid-template-columns: 14rem minmax(0, 1fr); max-width: 84rem; }
  .app.has-panel { grid-template-columns: 14rem minmax(0, 1fr) clamp(20rem, 30vw, 28rem); max-width: 108rem; }
  .js .side { display: block; position: sticky; top: calc(var(--stick) + var(--s4)); align-self: start; max-height: calc(100vh - var(--stick) - 32px); overflow: auto; }
  .foot { max-width: 84rem; }
}
@media (min-width: 1000px) and (max-width: 1279px) {
  .app.has-panel { grid-template-columns: minmax(0, 1fr) clamp(20rem, 34vw, 28rem); }
  .app.has-panel .side { display: none; }
}
@media (max-width: 999px) {
  :root { --stick: 108px; }
  .top {
    grid-template-columns: auto minmax(0, 1fr) auto; grid-template-areas: "brand find acts" "stamp stamp stamp";
    height: auto; row-gap: 2px; padding: var(--s2) var(--s3) 4px;
  }
  .stamp { justify-items: start; padding-bottom: 2px; }
  #upd { justify-self: start; }
  .panel { display: none; }
}
@media (max-width: 460px) {
  .dw { display: none; }
  .dn { display: inline; font-size: 16px; line-height: 1; }
  .rs { padding-right: 36px; }
  .id { min-width: 7ch; }
  .rs, .br { flex-wrap: wrap; align-items: flex-start; row-gap: 2px; }
  .t { display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 3; line-clamp: 3; white-space: normal; }
  .s { flex: 1 0 100%; max-width: 100%; flex-wrap: wrap; row-gap: 2px; padding-left: 20px; }
  .s .x { max-width: none; overflow: visible; text-overflow: clip; white-space: normal; }
  .st { white-space: normal; }
  .brand b, .brand .sub, .find kbd, .btn .lbl { display: none; }
  .btn .gly { display: inline; }
  .btn.go { width: 30px; padding: 0; }
  .top { gap: var(--s2); }
  .app { padding: var(--s3); }
  .sh { padding: 9px var(--s3); }
}
@media (prefers-reduced-motion: no-preference) {
  .btn, .ib, .rf, .tab, .si { transition: background-color .12s, color .12s; }
}
"#;

pub(super) const SCRIPT: &str = r##"
(function () {
  var root = document.documentElement;
  root.className += ' js';
  var state = { q: '', filter: '', type: '', modal: '', panel: '' };
  var q, n, empty, buttons, recs, haystack, shells, sections, board, app, panel, scrim, lastFocus;
  var saved = null;
  var wide = window.matchMedia ? window.matchMedia('(min-width: 1000px)') : null;
  function byId(id) { return document.getElementById(id); }
  function all(selector) { return Array.prototype.slice.call(document.querySelectorAll(selector)); }
  function isWide() { return wide !== null && wide.matches; }
  function snapshot() {
    var open = {};
    all('li[id] > details').forEach(function (d) {
      open[d.parentElement.id + (d.classList.contains('det') ? ':d' : '')] = d.open;
    });
    return open;
  }
  function restore(open) {
    all('li[id] > details').forEach(function (d) {
      var k = d.parentElement.id + (d.classList.contains('det') ? ':d' : '');
      if (Object.prototype.hasOwnProperty.call(open, k)) { d.open = open[k]; }
    });
  }
  function matches(r, text, k) {
    if (text !== '' && haystack[k].indexOf(text) === -1) { return false; }
    if (state.type !== '' && r.getAttribute('data-record') !== state.type) { return false; }
    if (state.filter === '') { return true; }
    if (state.filter === 'needs' || state.filter === 'ready') {
      return (' ' + r.getAttribute('data-flags') + ' ').indexOf(' ' + state.filter + ' ') !== -1;
    }
    return r.getAttribute('data-state') === state.filter;
  }
  function apply() {
    var text = state.q.trim().toLowerCase();
    var active = text !== '' || state.filter !== '' || state.type !== '';
    var shown = 0;
    if (active && saved === null) { saved = snapshot(); }
    if (!active && saved !== null) { restore(saved); saved = null; }
    shells.forEach(function (r) { r.hidden = active; });
    sections.forEach(function (s) { s.hidden = active; });
    recs.forEach(function (r, k) {
      if (!active || !matches(r, text, k)) { return; }
      shown += 1;
      var at = r;
      while (at) {
        if (at.classList && (at.classList.contains('rec') || at.classList.contains('fold'))) { at.hidden = false; }
        if (at.tagName === 'DETAILS') { at.open = true; }
        at = at.parentElement;
      }
    });
    n.hidden = !active;
    n.textContent = shown + ' of ' + recs.length + ' records shown';
    empty.hidden = !(active && shown === 0);
  }
  function mark() {
    buttons.forEach(function (b) {
      var on = b.hasAttribute('data-t') ? b.getAttribute('data-t') === state.type : b.getAttribute('data-f') === state.filter;
      b.classList.toggle('on', on);
      b.setAttribute('aria-pressed', on ? 'true' : 'false');
    });
  }
  function pick(b) {
    if (b.hasAttribute('data-t')) {
      var t = b.getAttribute('data-t');
      state.type = state.type === t ? '' : t;
    } else {
      state.filter = b.getAttribute('data-f');
      if (state.filter === '') { state.type = ''; }
    }
    mark();
    apply();
    if (board.getBoundingClientRect().top < 0) { board.scrollIntoView(); }
  }
  function closePanel() {
    var sel = document.querySelector('.rec.sel');
    if (sel) { sel.classList.remove('sel'); }
    if (panel) { panel.hidden = true; panel.replaceChildren(); }
    if (app) { app.classList.remove('has-panel'); }
    state.panel = '';
  }
  function openPanel(li) {
    var src = li.querySelector(':scope > .det > .more');
    if (!src || !panel) { return; }
    var sel = document.querySelector('.rec.sel');
    if (sel) { sel.classList.remove('sel'); }
    li.classList.add('sel');
    var head = document.createElement('div');
    head.className = 'ph';
    [['id', li.querySelector('.id')], ['pt', li.querySelector('.t')], ['st', li.querySelector('.st')]].forEach(function (p) {
      var span = document.createElement('span');
      span.className = p[0];
      span.textContent = p[1] ? p[1].textContent : '';
      head.appendChild(span);
    });
    var close = document.createElement('button');
    close.type = 'button';
    close.className = 'ib';
    close.setAttribute('aria-label', 'Close details');
    close.textContent = '×';
    close.addEventListener('click', closePanel);
    head.appendChild(close);
    panel.replaceChildren(head, src.cloneNode(true));
    panel.hidden = false;
    panel.scrollTop = 0;
    app.classList.add('has-panel');
    state.panel = li.id;
  }
  function reveal(li) {
    var at = li;
    while (at) {
      if (at.tagName === 'DETAILS' && at.classList.contains('kd')) { at.open = true; }
      at = at.parentElement;
    }
    li.scrollIntoView({ block: 'center' });
  }
  function hashId() { return document.URL.split('#')[1] || ''; }
  function goto(id) {
    var li = id === '' ? null : byId(id);
    if (!li || !li.classList.contains('rec')) { return; }
    reveal(li);
    if (isWide()) { openPanel(li); }
  }
  function openModal(id) {
    var s = byId(id);
    if (!s) { return; }
    closeModal();
    lastFocus = document.activeElement;
    state.modal = id;
    s.classList.add('pop');
    s.setAttribute('role', 'dialog');
    s.setAttribute('aria-modal', 'true');
    scrim.hidden = false;
    s.focus();
  }
  function closeModal() {
    var s = state.modal === '' ? null : byId(state.modal);
    if (s) {
      s.classList.remove('pop');
      s.removeAttribute('role');
      s.removeAttribute('aria-modal');
    }
    state.modal = '';
    if (scrim) { scrim.hidden = true; }
    if (lastFocus && lastFocus.focus) { lastFocus.focus(); }
    lastFocus = null;
  }
  function copyPrompt() {
    var ta = byId('prompt');
    var out = byId('copied');
    function done(ok) { out.textContent = ok ? 'Copied' : 'Select the text and copy it'; }
    function fallback() {
      ta.focus();
      ta.select();
      var ok = false;
      try { ok = document.execCommand('copy'); } catch (e) { ok = false; }
      done(ok);
    }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(ta.value).then(function () { done(true); }, fallback);
    } else {
      fallback();
    }
  }
  function themeIsDark() {
    var t = root.getAttribute('data-theme');
    if (t === 'dark') { return true; }
    if (t === 'light') { return false; }
    return window.matchMedia ? window.matchMedia('(prefers-color-scheme: dark)').matches : false;
  }
  function themeButton() {
    var b = byId('theme');
    if (b) { b.setAttribute('aria-pressed', themeIsDark() ? 'true' : 'false'); }
  }
  function toggleTheme() {
    var next = themeIsDark() ? 'light' : 'dark';
    root.setAttribute('data-theme', next);
    try { localStorage.setItem('urzua-board-theme', next); } catch (e) { next = ''; }
    themeButton();
  }
  function initTheme() {
    try {
      var t = localStorage.getItem('urzua-board-theme');
      if (t === 'light' || t === 'dark') { root.setAttribute('data-theme', t); }
    } catch (e) { t = ''; }
    themeButton();
  }
  function wire() {
    q = byId('q');
    board = byId('board');
    n = byId('shown');
    empty = byId('empty');
    app = byId('app');
    panel = byId('panel');
    scrim = byId('scrim');
    if (!q || !n || !empty || !board) { return; }
    buttons = all('[data-f], [data-t]');
    recs = all('.rec');
    haystack = recs.map(function (r) {
      return (r.querySelector('.id').textContent + ' ' + r.querySelector('.t').textContent).toLowerCase();
    });
    shells = all('.rec, .fold');
    sections = all('section[data-sec]');
    saved = null;
    q.value = state.q;
    buttons.forEach(function (b) { b.addEventListener('click', function () { pick(b); }); });
    mark();
    apply();
    var held = state.panel;
    var modal = state.modal;
    state.panel = '';
    state.modal = '';
    if (held !== '' && byId(held) && isWide()) { openPanel(byId(held)); } else { closePanel(); }
    if (modal !== '') { openModal(modal); }
  }
  function editing(t) {
    return t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable);
  }
  function trap(e) {
    var s = byId(state.modal);
    var items = s ? Array.prototype.slice.call(s.querySelectorAll('a[href], button, textarea, input')) : [];
    if (items.length === 0) { return; }
    var first = items[0];
    var last = items[items.length - 1];
    if (e.shiftKey && (document.activeElement === first || document.activeElement === s)) { e.preventDefault(); last.focus(); }
    if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
  }
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Tab' && state.modal !== '') { trap(e); return; }
    if (e.key === '/' && !editing(e.target) && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      q.focus();
      q.select();
    } else if (e.key === 'Escape') {
      if (state.modal !== '') { closeModal(); }
      else if (state.q !== '' || document.activeElement === q) { state.q = ''; q.value = ''; apply(); q.blur(); }
      else if (state.panel !== '') { closePanel(); }
    }
  });
  document.addEventListener('click', function (e) {
    var t = e.target;
    if (!t.closest) { return; }
    var summary = t.closest('details.det > summary');
    var opener = t.closest('[data-open]');
    var link = panel && !panel.hidden ? t.closest('#panel a[href^="#r-"]') : null;
    if (summary && isWide()) {
      e.preventDefault();
      var li = summary.closest('li.rec');
      if (state.panel === li.id) { closePanel(); } else { openPanel(li); }
    } else if (opener) {
      e.preventDefault();
      openModal(opener.getAttribute('data-open'));
    } else if (t.closest('[data-close]') || t === scrim) {
      closeModal();
    } else if (t.closest('#copy')) {
      copyPrompt();
    } else if (t.closest('#theme')) {
      toggleTheme();
    } else if (link) {
      var target = byId(link.getAttribute('href').slice(1));
      if (target) { e.preventDefault(); reveal(target); openPanel(target); }
    }
  });
  window.addEventListener('hashchange', function () { goto(hashId()); });
  initTheme();
  wire();
  q.addEventListener('input', function () { state.q = q.value; apply(); });
  goto(hashId());
/*LIVE*/
})();
"##;

/// What only a served page has: the relative "updated" time, polling `stamp`
/// while the tab is visible, the Refresh button, and the swap that keeps the
/// reader's place. Every string it puts in the page came from the server,
/// escaped, as nodes of the fetched document; it builds none itself.
pub(super) const LIVE: &str = r#"
  var upd = document.getElementById('upd');
  var meta = document.getElementById('meta');
  var button = document.getElementById('refresh');
  var shell = ['board', 'tabs', 'side'];
  if (upd && meta && button && board) {
    var digest = upd.getAttribute('data-d');
    var exact = upd.getAttribute('title');
    var receivedAt = Date.now();
    var failed = false;
    var polling = null;
    var ticking = null;
    function ago(ms) {
      var s = Math.max(0, Math.round(ms / 1000));
      if (s < 60) { return s + ' s ago'; }
      if (s < 3600) { return Math.floor(s / 60) + ' min ago'; }
      if (s < 86400) { return Math.floor(s / 3600) + ' h ago'; }
      return Math.floor(s / 86400) + ' d ago';
    }
    function show() {
      if (failed) {
        upd.className = 'meta warn';
        upd.textContent = 'not updating: last read ' + exact;
      } else {
        upd.className = 'meta';
        upd.textContent = 'updated ' + ago(Date.now() - receivedAt);
      }
      upd.title = exact;
    }
    function get(path) {
      return fetch(path, { cache: 'no-store' }).then(function (r) {
        if (!r.ok) { throw new Error('status ' + r.status); }
        return r;
      });
    }
    function swap(text) {
      var doc = new DOMParser().parseFromString(text, 'text/html');
      var fresh = doc.getElementById('board');
      var freshUpd = doc.getElementById('upd');
      var freshMeta = doc.getElementById('meta');
      if (!fresh || !freshUpd || !freshMeta) { throw new Error('not a board page'); }
      var open = saved !== null ? saved : snapshot();
      var y = window.scrollY;
      shell.forEach(function (id) {
        var host = document.getElementById(id);
        var from = doc.getElementById(id);
        if (!host || !from) { return; }
        var nodes = Array.prototype.map.call(from.childNodes, function (c) { return document.importNode(c, true); });
        host.replaceChildren.apply(host, nodes);
      });
      restore(open);
      wire();
      meta.textContent = freshMeta.textContent;
      digest = freshUpd.getAttribute('data-d');
      exact = freshUpd.getAttribute('title');
      receivedAt = Date.now();
      window.scrollTo(0, y);
    }
    function fail() { failed = true; show(); }
    function reload() {
      return get('./').then(function (r) { return r.text(); }).then(function (text) {
        swap(text);
        failed = false;
        show();
      }).catch(fail);
    }
    function poll() {
      return get('stamp').then(function (r) { return r.json(); }).then(function (s) {
        failed = false;
        if (s.digest !== digest) { return reload(); }
        show();
      }).catch(fail);
    }
    function start() {
      if (polling === null) { polling = setInterval(poll, 15000); }
      if (ticking === null) { ticking = setInterval(show, 1000); }
    }
    function stop() {
      clearInterval(polling); polling = null;
      clearInterval(ticking); ticking = null;
    }
    button.addEventListener('click', function () {
      button.disabled = true;
      reload().then(function () { button.disabled = false; });
    });
    document.addEventListener('visibilitychange', function () {
      if (document.hidden) { stop(); } else { poll(); show(); start(); }
    });
    show();
    if (!document.hidden) { start(); }
  }
"#;
