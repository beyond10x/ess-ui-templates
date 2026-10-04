// The self-contained document of a presentation: the same document `ess-ui page` writes, composed
// from the data `ess-ui data` writes and the bundle in dist/assets.js.

const ENTITIES = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#x27;' };
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ENTITIES[c]);
const scriptJson = (v) => JSON.stringify(v).replace(/<\//g, '<\\/');

/**
 * @param {object} data an `ess-ui-presentation/1` document
 * @param {{css: string, js: string, headJs: string}} assets the bundle
 * @param {{theme?: 'light'|'dark'}} [opts] a theme to open in; otherwise the reader's preference
 * @returns {string} the HTML document
 */
export function compose(data, assets, opts = {}) {
  if (!data || data.format !== 'ess-ui-presentation/1') {
    throw new Error('ess-ui: expected an ess-ui-presentation/1 document');
  }
  const theme = opts.theme === 'dark' || opts.theme === 'light' ? `<script>window.ESS_UI_THEME=${JSON.stringify(opts.theme)}</script>` : '';
  return `<!doctype html>
<html lang="en" data-theme="light"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="ess-ir-sha256" content="${data.ir_sha256}"><meta name="generator" content="${esc(data.generator)}">
<title>${esc(data.document_title)}</title>${theme}<script>${assets.headJs}</script><style>${assets.css}</style></head>
<body class="${esc(data.body_class)}">${data.body}
<script type="application/json" id="ess-model">${scriptJson(data.model)}</script>
<script type="application/json" id="ess-sim">${scriptJson(data.sim)}</script>
<script>${assets.js}</script></body></html>
`;
}
