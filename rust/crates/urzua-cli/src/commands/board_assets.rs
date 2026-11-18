//! The page's own style and script. Fixed text: nothing from a record is ever
//! put into either, which is what lets the page's Content-Security-Policy name
//! their hashes and allow nothing else to run or load.

pub(super) const STYLE: &str = r#"
:root {
  --bg: #FFFFFF; --raised: #F6F6F7; --border: #E4E4E7; --fg: #18181B; --muted: #71717A;
  --green: #12B550; --amber: #FF7A1A; --blue: #0F5BF0; --red: #E5233D; --wait: #B45309;
  --radius: 6px;
  --sans: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
@media (prefers-color-scheme: dark) {
  :root { --bg: #0F0F10; --raised: #161617; --border: #262628; --fg: #EDEDED; --muted: #8B8B8F; --wait: #FF7A1A; }
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body { margin: 0; background: var(--bg); color: var(--fg); font: 13px/1.45 var(--sans); overflow-wrap: anywhere; }
.wrap { max-width: 62rem; margin: 0 auto; padding: 14px 16px 40px; }
a { color: inherit; text-decoration: underline; text-decoration-color: var(--border); text-underline-offset: 2px; }
a:hover { text-decoration-color: currentColor; }
code, pre { font-family: var(--mono); font-size: 12px; }
.top { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; margin-bottom: 6px; }
.top b { font-size: 14px; font-weight: 600; }
.meta { color: var(--muted); font: 12px var(--mono); }
.note { color: var(--muted); font-size: 12px; margin: 0 0 6px; }
.bar { display: none; gap: 8px; flex-wrap: wrap; margin: 10px 0 4px; }
.js .bar { display: flex; }
.bar input {
  flex: 1 1 180px; min-width: 0; font: inherit; color: var(--fg); background: transparent;
  border: 1px solid var(--border); border-radius: var(--radius); padding: 6px 10px;
}
.bar input:focus, .seg button:focus-visible { outline: 1px solid var(--muted); outline-offset: -1px; }
.seg { display: flex; max-width: 100%; overflow-x: auto; border: 1px solid var(--border); border-radius: var(--radius); }
.seg button {
  flex: none; font: inherit; font-size: 12px; color: var(--muted); background: transparent; border: 0;
  border-left: 1px solid var(--border); padding: 6px 10px; cursor: pointer; white-space: nowrap;
}
.seg button:first-child { border-left: 0; }
.seg button.on { background: var(--raised); color: var(--fg); }
h3 {
  font-size: 11px; font-weight: 500; letter-spacing: .06em; text-transform: uppercase;
  color: var(--muted); margin: 18px 0 6px;
}
h3.ty { text-transform: none; letter-spacing: 0; font-size: 12px; font-family: var(--mono); }
h3 i { font-style: normal; margin-left: 6px; opacity: .7; }
ul { list-style: none; margin: 0; padding: 0; }
.list { border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
.list li { border-top: 1px solid var(--border); min-width: 0; }
.list > li:first-child { border-top: 0; }
summary, .br {
  display: flex; align-items: center; gap: 10px; padding: 7px 10px; min-width: 0;
  list-style: none; cursor: pointer;
}
.br { cursor: default; }
summary::-webkit-details-marker { display: none; }
summary > *, .br > * { min-width: 0; }
.p > details > summary { background: var(--raised); }
.p > details > summary .t { font-weight: 600; }
.ch { flex: none; width: 10px; color: var(--muted); font-size: 10px; }
.tri::before { content: "\25B8"; }
details[open] > summary > .tri::before { content: "\25BE"; }
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
li[class*="lv"] > details > summary, li[class*="lv"] > details > .more { padding-left: calc(10px + var(--ind)); }
li[class*="lv"] > details > .more { padding-left: calc(30px + var(--ind)); }
[hidden] { display: none !important; }
@media (max-width: 460px) {
  .s .x { display: none; }
  .id { min-width: 7ch; }
  .st { max-width: 12ch; overflow: hidden; text-overflow: ellipsis; }
  .s { max-width: 45%; }
}
"#;

pub(super) const SCRIPT: &str = r#"
(function () {
  document.documentElement.className += ' js';
  var q = document.getElementById('q');
  var n = document.getElementById('shown');
  var buttons = Array.prototype.slice.call(document.querySelectorAll('.seg button'));
  if (!q || !n) { return; }
  var recs = Array.prototype.slice.call(document.querySelectorAll('.rec'));
  var haystack = recs.map(function (r) {
    return (r.querySelector('.id').textContent + ' ' + r.querySelector('.t').textContent).toLowerCase();
  });
  var shells = Array.prototype.slice.call(document.querySelectorAll('.rec, .fold'));
  var sections = Array.prototype.slice.call(document.querySelectorAll('section[data-sec]'));
  var filter = '';
  function matches(r, text, k) {
    if (text !== '' && haystack[k].indexOf(text) === -1) { return false; }
    if (filter === '') { return true; }
    if (filter === 'needs' || filter === 'ready') {
      return (' ' + r.getAttribute('data-flags') + ' ').indexOf(' ' + filter + ' ') !== -1;
    }
    return r.getAttribute('data-state') === filter;
  }
  function apply() {
    var text = q.value.trim().toLowerCase();
    var active = text !== '' || filter !== '';
    var shown = 0;
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
  }
  buttons.forEach(function (b) {
    b.addEventListener('click', function () {
      filter = b.getAttribute('data-f');
      buttons.forEach(function (o) { o.className = o === b ? 'on' : ''; });
      apply();
    });
  });
  q.addEventListener('input', apply);
  apply();
})();
"#;
