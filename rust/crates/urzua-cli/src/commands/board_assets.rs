//! The page's own style and script. Fixed text: nothing from a record is ever
//! put into either, which is what lets the page's Content-Security-Policy name
//! their hashes and allow nothing else to run or load.

pub(super) const STYLE: &str = r#"
:root {
  --bg: #FFFFFF; --surface: #F3F4F6; --text: #14181F; --muted: #4A5361; --line: #D5D9E0;
  --finished: #12B550; --waiting: #FFD21F; --waiting-text: #14181F;
  --under-way: #FF7A1A; --dropped: #E5233D; --link: #0F5BF0; --ready: #0F5BF0;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0E1116; --surface: #1A1F27; --text: #E8EBF0; --muted: #A5AEBC; --line: #343B47;
    --link: #6C9CFF; --ready: #6C9CFF;
  }
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body {
  margin: 0; background: var(--bg); color: var(--text);
  font: 16px/1.45 system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  overflow-wrap: anywhere;
}
main, header.top { max-width: 62rem; margin: 0 auto; padding: 0 16px; }
header.top { padding-top: 16px; }
h1 { font-size: 1.5rem; margin: 0 0 .5rem; }
h2 { font-size: 1.15rem; margin: 2rem 0 .5rem; padding-top: .75rem; border-top: 1px solid var(--line); }
h3 { font-size: 1rem; margin: 1.25rem 0 .4rem; color: var(--muted); }
a { color: var(--link); }
code, pre { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .9em; }
pre {
  white-space: pre-wrap; background: var(--surface); padding: .6rem .75rem;
  border-radius: 4px; margin: .5rem 0 0; max-height: 28rem; overflow: auto;
}
ul { list-style: none; margin: 0; padding: 0; }
ul.kids { margin-left: 1rem; border-left: 2px solid var(--line); padding-left: .75rem; }
li.rec { margin: .35rem 0; }
.stamp, .check, .note { color: var(--muted); margin: .25rem 0; }
.stamp strong, .check strong { color: var(--text); }
.rec > details, .rec > .row { background: var(--surface); border-radius: 4px; padding: .4rem .6rem; }
summary { cursor: pointer; display: flex; flex-wrap: wrap; gap: .25rem .6rem; align-items: baseline; }
.id { font-weight: 600; }
.status { color: var(--muted); }
.mark {
  display: inline-block; padding: 0 .45rem; border-radius: 3px; font-size: .8rem;
  border: 1px solid currentColor; border-left-width: .5rem; white-space: nowrap;
}
.m-finished { border-color: var(--finished); }
.m-dropped { border-color: var(--dropped); }
.m-under-way { border-color: var(--under-way); }
.m-not-started, .m-none { border-color: var(--line); color: var(--muted); }
.m-waiting { background: var(--waiting); border-color: var(--waiting); color: var(--waiting-text); }
.m-ready { border-color: var(--ready); color: var(--link); }
.m-disagree { border-color: var(--dropped); border-left-width: 1px; font-weight: 600; }
.counts { color: var(--muted); font-size: .85rem; }
.meta { margin: .3rem 0 0; color: var(--muted); font-size: .9rem; }
.meta div { margin: .1rem 0; }
.more { margin-top: .4rem; }
.more > summary { color: var(--muted); font-size: .85rem; }
details.group > summary { font-weight: 600; font-size: 1rem; padding: .3rem 0; }
.controls { display: none; gap: .5rem; flex-wrap: wrap; margin: .75rem 0; }
.js .controls { display: flex; }
.controls input, .controls select {
  font: inherit; padding: .35rem .5rem; color: var(--text); background: var(--surface);
  border: 1px solid var(--line); border-radius: 4px; max-width: 100%;
}
.controls input { flex: 1 1 12rem; }
[hidden] { display: none !important; }
.brief li { margin: .2rem 0; }
"#;

pub(super) const SCRIPT: &str = r#"
(function () {
  document.documentElement.className += ' js';
  var q = document.getElementById('q');
  var s = document.getElementById('s');
  var n = document.getElementById('shown');
  if (!q || !s) { return; }
  var recs = Array.prototype.slice.call(document.querySelectorAll('.rec'));
  function apply() {
    var text = q.value.trim().toLowerCase();
    var state = s.value;
    var active = text !== '' || state !== '';
    var shown = 0;
    recs.forEach(function (r) { r.hidden = active; });
    recs.forEach(function (r) {
      var hit = (text === '' || r.getAttribute('data-search').indexOf(text) !== -1) &&
        (state === '' || r.getAttribute('data-state') === state);
      if (!active || !hit) { return; }
      shown += 1;
      var at = r;
      while (at) {
        if (at.classList && at.classList.contains('rec')) { at.hidden = false; }
        if (at.tagName === 'DETAILS') { at.open = true; }
        at = at.parentElement;
      }
    });
    if (!active) { shown = recs.length; }
    n.textContent = shown + ' of ' + recs.length + ' records shown';
  }
  q.addEventListener('input', apply);
  s.addEventListener('change', apply);
  apply();
})();
"#;
