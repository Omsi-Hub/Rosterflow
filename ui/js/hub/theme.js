// The look: light or dark, as the planner chose it in the settings (`theme`: "light", "dark",
// or "" for the system's), put on <html> as `data-theme` for the stylesheets.

import { S } from '../store.js';

const system = () => !!window.matchMedia?.('(prefers-color-scheme: dark)').matches;

/** The dark look is shown. */
export function dark() {
  const t = S.settings?.theme || '';
  return t ? t === 'dark' : system();
}

/** The look put on the page (after the settings changed, or the system's look). */
export function applyTheme() {
  document.documentElement.dataset.theme = dark() ? 'dark' : 'light';
}

window.matchMedia?.('(prefers-color-scheme: dark)').addEventListener?.('change', applyTheme);
