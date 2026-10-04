// <EssPresentation data={…}>: the ESS presentation page inside a React (Docusaurus) site.
//
// The page is rendered into a sandboxed iframe from the `ess-ui data` document, so its keyboard
// control, presentation mode, URL state and styles stay its own and never collide with the host
// site's. The host's light or dark theme is followed: pass `theme`, or leave it out and the
// wrapper reads `data-theme` from the host's <html>, as Docusaurus sets it.

import { createElement, useEffect, useMemo, useRef, useState } from 'react';
import * as assets from './dist/assets.js';
import { compose } from './compose.js';

export { compose } from './compose.js';

function hostTheme() {
  if (typeof document === 'undefined') return 'light';
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
}

/**
 * @param {object} props
 * @param {object} props.data the JSON `ess-ui data` writes (`ess-ui-presentation/1`)
 * @param {'light'|'dark'} [props.theme] fixed theme; default: follow the host page
 * @param {string|number} [props.height] the frame's height; default `min(88vh, 960px)`
 * @param {string} [props.title] the frame's accessible title; default the presentation's title
 * @param {string} [props.className]
 * @param {object} [props.style]
 */
export function EssPresentation({ data, theme, height = 'min(88vh, 960px)', title, className, style }) {
  const frame = useRef(null);
  const [current, setCurrent] = useState(() => theme || hostTheme());

  useEffect(() => {
    if (theme) {
      setCurrent(theme);
      return undefined;
    }
    setCurrent(hostTheme());
    if (typeof MutationObserver === 'undefined') return undefined;
    const watch = new MutationObserver(() => setCurrent(hostTheme()));
    watch.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => watch.disconnect();
  }, [theme]);

  // The document is composed once per data; a theme change is posted to it, not re-rendered.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const srcDoc = useMemo(() => compose(data, assets, { theme: current }), [data]);

  const post = () => {
    const w = frame.current && frame.current.contentWindow;
    if (w) w.postMessage({ type: 'ess-ui:theme', theme: current }, '*');
  };
  useEffect(post, [current]);

  return createElement('iframe', {
    ref: frame,
    srcDoc,
    title: title || data.title,
    onLoad: post,
    className,
    allow: 'fullscreen; clipboard-write',
    allowFullScreen: true,
    sandbox: 'allow-scripts allow-popups allow-popups-to-escape-sandbox',
    style: { display: 'block', width: '100%', height, border: '1px solid var(--b10x-color-hairline, #d6dfda)', borderRadius: '0.85rem', ...style },
  });
}

export default EssPresentation;
