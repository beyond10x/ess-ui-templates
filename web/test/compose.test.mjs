// The React wrapper's document is the page `ess-ui page` writes: compose(data.json) must equal the
// HTML byte for byte outside the two embedded JSON documents, and those must be equal as data.
// Usage: node web/test/compose.test.mjs <data.json> <page.html>
import { readFileSync } from 'node:fs';
import { deepStrictEqual, strictEqual, throws } from 'node:assert';
import * as assets from '../dist/assets.js';
import { compose } from '../compose.js';

const [dataPath, htmlPath] = process.argv.slice(2);
const data = JSON.parse(readFileSync(dataPath, 'utf8'));
const html = readFileSync(htmlPath, 'utf8');
const composed = compose(data, assets);

const split = (doc) => {
  const parts = {};
  const rest = doc.replace(/<script type="application\/json" id="(ess-model|ess-sim)">([\s\S]*?)<\/script>/g, (_, id, body) => {
    parts[id] = JSON.parse(body);
    return `<script type="application/json" id="${id}"></script>`;
  });
  return { rest, parts };
};
const a = split(composed);
const b = split(html);
strictEqual(a.rest, b.rest, 'the composed document differs from the page outside its data');
deepStrictEqual(a.parts['ess-model'], b.parts['ess-model'], 'ess-model differs');
deepStrictEqual(a.parts['ess-sim'], b.parts['ess-sim'], 'ess-sim differs');
throws(() => compose({ format: 'other' }, assets));
const dark = compose(data, assets, { theme: 'dark' });
strictEqual(dark.includes('window.ESS_UI_THEME="dark"'), true, 'a theme is passed to the document');
console.log(`ok   compose: the React wrapper's document equals ${htmlPath} (${composed.length} bytes)`);
