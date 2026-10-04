// The self-contained document of a presentation: the same document `ess-ui page` writes, composed
// from the data `ess-ui data` writes, the bundle in dist/assets.js and the fonts in dist/fonts.js.

const ENTITIES = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#x27;' };
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ENTITIES[c]);
const scriptJson = (v) => JSON.stringify(v).replace(/<\//g, '<\\/');

/**
 * @param {object} data an `ess-ui-presentation/1` document
 * @param {{css: string, js: string, headJs: string}} assets the bundle
 * @param {{theme?: 'light'|'dark', fonts?: string}} [opts] a theme to open in, otherwise the reader's
 *   preference; and the `@font-face` rules to embed (`css` of dist/fonts.js for the page `ess-ui page`
 *   writes), otherwise none and the page falls back to system fonts
 * @returns {string} the HTML document
 */
export function compose(data, assets, opts = {}) {
  if (!data || data.format !== 'ess-ui-presentation/1') {
    throw new Error('ess-ui: expected an ess-ui-presentation/1 document');
  }
  const fonts = typeof opts.fonts === 'string' ? opts.fonts : '';
  const theme = opts.theme === 'dark' || opts.theme === 'light' ? `<script>window.ESS_UI_THEME=${JSON.stringify(opts.theme)}</script>` : '';
  return `<!doctype html>
<html lang="en" data-theme="light"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="ess-ir-sha256" content="${data.ir_sha256}"><meta name="generator" content="${esc(data.generator)}">
<title>${esc(data.document_title)}</title>${theme}<script>${assets.headJs}</script><style>${fonts}${assets.css}</style></head>
<body class="${esc(data.body_class)}">${data.body}
<script type="application/json" id="ess-model">${scriptJson(data.model)}</script>
<script type="application/json" id="ess-sim">${scriptJson(data.sim)}</script>
<script>${assets.js}</script></body></html>
`;
}

const FAMILIES = ['Inter', 'Fira Code'];
const FONT_FACE_RULE = 5;

/** The host's `@font-face` rules for FAMILIES with absolute URLs, or null unless every family has one. */
export function hostFonts(doc = typeof document === 'undefined' ? undefined : document) {
  if (!doc) return null;
  const rules = [];
  for (const sheet of Array.from(doc.styleSheets || [])) {
    let list;
    try {
      list = sheet.cssRules;
    } catch (e) {
      continue;
    }
    const base = sheet.href || doc.baseURI;
    for (const rule of Array.from(list || [])) {
      if (rule.type !== FONT_FACE_RULE) continue;
      const family = rule.style.getPropertyValue('font-family').replace(/["']/g, '').trim();
      if (!FAMILIES.includes(family)) continue;
      const css = rule.cssText.replace(/url\(\s*(["']?)([^"')]+)\1\s*\)/g, (_, q, u) => `url("${new URL(u, base).href}")`);
      rules.push({ family, css });
    }
  }
  if (!FAMILIES.every((f) => rules.some((r) => r.family === f))) return null;
  return rules.map((r) => r.css).join('\n') + '\n';
}
