// The React wrapper's document is the page `ess-ui page` writes: compose(data.json) must equal the
// HTML byte for byte outside the two embedded JSON documents, and those must be equal as data.
// Usage: node web/test/compose.test.mjs <data.json> <page.html>
import { readFileSync } from 'node:fs';
import { deepStrictEqual, strictEqual, throws } from 'node:assert';
import * as assets from '../dist/assets.js';
import * as fonts from '../dist/fonts.js';
import { compose, hostFonts } from '../compose.js';

const [dataPath, htmlPath] = process.argv.slice(2);
const data = JSON.parse(readFileSync(dataPath, 'utf8'));
const html = readFileSync(htmlPath, 'utf8');
const composed = compose(data, assets, { fonts: fonts.css });

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

// Fonts: the page embeds every face as a data: URL; without fonts the document declares none.
strictEqual((composed.match(/@font-face\{/g) || []).length, 3, 'the page embeds three faces');
strictEqual(/src:url\("(?!data:font\/woff2;base64,)/.test(composed), false, 'no face is loaded from a URL');
strictEqual(composed.includes('SIL OPEN FONT LICENSE Version 1.1'), true, 'the OFL licences are embedded');
strictEqual(dark.includes('@font-face'), false, 'without fonts the document declares no face');

// hostFonts: the host's faces are taken with absolute URLs, and only when both families are there.
const face = (family, url) => ({
  type: 5,
  style: { getPropertyValue: (p) => (p === 'font-family' ? `"${family}"` : '') },
  cssText: `@font-face { font-family: "${family}"; src: url("${url}") format("woff2"); }`,
});
const sheet = (rules, href = null) => ({ href, cssRules: rules });
const hostDoc = (sheets) => ({ baseURI: 'https://example.org/ess/docs/page', styleSheets: sheets });
const both = hostFonts(hostDoc([
  { href: 'https://cdn.example.org/x.css', get cssRules() { throw new Error('cross-origin'); } },
  sheet([{ type: 1 }, face('Inter', '/ess/b10x-fonts/inter-variable-latin.woff2')]),
  sheet([face('Fira Code', 'fonts/fira.woff2')], 'https://example.org/ess/assets/site.css'),
]));
strictEqual(both.includes('url("https://example.org/ess/b10x-fonts/inter-variable-latin.woff2")'), true, 'a root-relative URL is made absolute against the page');
strictEqual(both.includes('url("https://example.org/ess/assets/fonts/fira.woff2")'), true, 'a relative URL is resolved against its stylesheet');
strictEqual(hostFonts(hostDoc([sheet([face('Inter', '/i.woff2')])])), null, 'one family alone is not enough');
strictEqual(hostFonts(undefined), null, 'no document, no host fonts');
const viaHost = compose(data, assets, { fonts: both });
strictEqual(viaHost.includes('data:font/woff2'), false, 'host fonts replace the embedded ones');
console.log(`ok   compose: the React wrapper's document equals ${htmlPath} (${composed.length} bytes)`);
