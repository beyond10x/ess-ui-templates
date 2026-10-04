// The run picker in a real browser: ARIA, filtering, keyboard, selection, URL state and the palette
// path. Composes the page from data.json with a probe appended, runs it in headless Chrome and
// reads the probe's verdict from the DOM.
// Usage: node web/test/picker.test.mjs <data.json> <out-dir>
import { readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join, resolve } from 'node:path';
import * as assets from '../dist/assets.js';
import { compose } from '../compose.js';

const [dataPath, outDir] = process.argv.slice(2);
const data = JSON.parse(readFileSync(dataPath, 'utf8'));
const probe = `<script>
window.addEventListener('load', () => setTimeout(async () => {
  const out = []; const ok = (c, m) => out.push((c ? 'ok   ' : 'FAIL ') + m);
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const key = (el, k) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));
  const btn = document.getElementById('dm-trace'), q = document.getElementById('dm-run-q'), list = document.getElementById('dm-run-list');
  const traces = JSON.parse(document.getElementById('ess-sim').textContent).traces;
  ok(btn.getAttribute('aria-haspopup') === 'listbox' && btn.getAttribute('aria-expanded') === 'false', 'closed button is a listbox popup');
  ok(btn.querySelector('.rp-kind').textContent && /\\d+ steps/.test(btn.querySelector('.rp-steps').textContent), 'button shows kind and step count');
  btn.click(); await wait(50);
  ok(btn.getAttribute('aria-expanded') === 'true' && document.activeElement === q, 'click opens and focuses the search');
  ok(q.getAttribute('role') === 'combobox' && list.getAttribute('role') === 'listbox', 'combobox and listbox roles');
  const opts = list.querySelectorAll('[role=option]');
  ok(opts.length === traces.length, 'every run is an option (' + opts.length + ')');
  ok(q.getAttribute('aria-activedescendant') && document.getElementById(q.getAttribute('aria-activedescendant')), 'active option is announced');
  const scen = [...opts].filter((o) => traces[+o.dataset.i].kind === 'scenario');
  ok(scen.every((o) => /(met|failed|undetermined)/.test(o.querySelector('.rp-v').textContent) && o.querySelector('.rp-v i')), 'scenarios carry a verdict icon and word');
  q.value = 'seeded'; q.dispatchEvent(new Event('input')); await wait(20);
  ok(list.querySelectorAll('[role=option]').length === traces.filter((t) => t.origin === 'seeded').length, 'search filters to the seeded runs');
  key(q, 'ArrowDown'); key(q, 'Enter'); await wait(400);
  ok(btn.getAttribute('aria-expanded') === 'false' && document.getElementById('dm-run-pop').hidden, 'Enter chooses and closes');
  ok(window.essDemo.get().trace === 'run:2' && btn.querySelector('.rp-name').textContent === 'seeded run 2', 'the chosen run plays and is named');
  ok(/trace=run%3A2|trace=run:2/.test(location.hash), 'URL state carries trace=');
  key(btn, 'b'); await wait(50);
  ok(btn.getAttribute('aria-expanded') === 'true' && q.value === 'b', 'typing on the button opens with that text');
  key(q, 'Escape'); key(q, 'Escape'); await wait(50);
  ok(btn.getAttribute('aria-expanded') === 'false' && document.activeElement === btn, 'Esc clears, then closes and returns focus');
  window.essDemo.load(traces[0].id); await wait(50);
  ok(btn.querySelector('.rp-name').textContent === traces[0].title, 'a load from the palette updates the picker');
  const pre = document.createElement('pre'); pre.id = 'picker-result'; pre.textContent = out.join('\\n'); document.body.appendChild(pre);
}, 300));
</script>`;
const page = resolve(join(outDir, 'picker-test.html'));
writeFileSync(page, compose(data, assets, { theme: 'light' }).replace('</body></html>', probe + '</body></html>'));
const dom = execFileSync('google-chrome-stable', ['--headless=new', '--disable-gpu', '--window-size=1440,1000', '--virtual-time-budget=8000', '--dump-dom', 'file://' + page], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], maxBuffer: 64 << 20 });
const m = dom.match(/<pre id="picker-result">([\s\S]*?)<\/pre>/);
if (!m) { console.error('FAIL picker: the probe did not run'); process.exit(1); }
const text = m[1].replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>');
console.log(text);
if (/^FAIL/m.test(text)) process.exit(1);
