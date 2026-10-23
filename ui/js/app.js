// RosterFlow's page: the start (the companies), and a company opened - its overview, RosterFlow's
// planning over the whole window, and the master data's pages (lines, stops, fleet, staff,
// holiday requests, agency drivers, settings) beside a side bar. Toasts in the corner, the popup
// that says why something cannot be done over it all.

import { html, render, useEffect } from '../vendor/preact-htm.js';
import { call } from './api.js';
import { S, set, useStore, toast } from './store.js';
import { setLanguage, useLang } from './i18n.js';
import { applyTheme } from './hub/theme.js';
import { tr, Icon } from './hub/bits.js';
import { HOME, PLANNING, setCo, freshCo, goTab, refresh, handOver, Popup, Mark } from './company/kit.js';
import { RosterFlowPage, openRosterFlow, rfOpen } from './rosterflow/index.js';
import { fresh as freshRf } from './rosterflow/state.js';
import { StartPage } from './pages/start.js';
import { Dashboard } from './pages/dashboard.js';
import { LinesPage } from './pages/lines.js';
import { StopsPage } from './pages/stops.js';
import { FleetPage } from './pages/fleet.js';
import { StaffPage } from './pages/staff.js';
import { LeavePage } from './pages/leave.js';
import { AgencyPage } from './pages/agency.js';
import { SettingsPage } from './pages/settings.js';

/** The side bar's pages: key, icon, label (translated where drawn). */
const NAV = [
  [HOME, 'grid_view', 'Overview'],
  [PLANNING, 'calendar_month', 'Planning'],
  ['lines', 'route', 'Lines'],
  ['stops', 'place', 'Stops'],
  ['fleet', 'directions_bus', 'Fleet'],
  ['staff', 'badge', 'Staff'],
  ['leave', 'event_busy', 'Holiday requests'],
  ['agency', 'groups', 'Agency drivers'],
  ['settings', 'settings', 'Settings'],
];

const PAGES = { lines: LinesPage, stops: StopsPage, fleet: FleetPage, staff: StaffPage, leave: LeavePage, agency: AgencyPage, settings: SettingsPage };

/** The company opened (by the start, a new one made, a file taken in): its screen shown. */
async function enter() {
  set({ rf: freshRf(), co: freshCo() });
  await refresh();
  set({ page: 'company' });
}
S.enter = enter;

/** Back to the start: the company closed. */
async function leave() {
  await call('app_close').catch(() => {});
  const companies = await call('app_companies').catch(() => []);
  set({ page: 'start', boot: { ...S.boot, companies }, rf: freshRf(), co: freshCo() });
}

async function boot() {
  let b;
  try {
    b = await call('app_boot');
  } catch (e) {
    set({ page: 'failed', failed: String(e.message || e) });
    return;
  }
  set({ boot: b, settings: b.settings });
  applyTheme();
  await setLanguage(b.settings.language || '');
  if (b.open) await enter();
  else set({ page: 'start' });
}

function Toasts() {
  const toasts = useStore((s) => s.toasts);
  return html`<div class="toasts">${toasts.map((t) => html`<div class=${'toast ' + t.kind} key=${t.id}><${Icon} name=${t.kind === 'error' ? 'error' : t.kind === 'ok' ? 'check_circle' : 'info'} size=${18} /><span>${t.text}</span></div>`)}</div>`;
}

/** What RosterFlow says in its corner (`S.co.notice`), as a toast. */
let lastNotice = 0;
function Notices() {
  const n = useStore((s) => s.co?.notice);
  useEffect(() => {
    if (n && n.at !== lastNotice) {
      lastNotice = n.at;
      toast(n.text, n.error ? 'error' : 'info', 6000);
    }
  }, [n]);
  return null;
}

function SideBar() {
  const co = S.co;
  const sh = co.shell || {};
  const go = (k) => {
    if (k === PLANNING) openRosterFlow();
    else goTab(k);
  };
  return html`<nav class="side">
    <div class="side-co" title=${sh.name}>
      <${Mark} mark=${{ short: sh.short, colours: sh.colours }} size=${36} />
      <span class="grow cut"><b class="cut">${sh.name || ''}</b><small>RosterFlow</small></span>
    </div>
    ${NAV.map(([k, icon, label]) => html`<button type="button" class=${'side-item' + (co.tab === k ? ' on' : '')} onClick=${() => go(k)}><${Icon} name=${icon} size=${20} /><span>${tr(label)}</span></button>`)}
    <span class="grow"></span>
    <button type="button" class="side-item quiet" onClick=${leave}><${Icon} name="swap_horiz" size=${20} /><span>${tr('Other company')}</span></button>
  </nav>`;
}

function CompanyScreen() {
  const co = S.co;
  // (the shell's notes - what went wrong saving - said once)
  useEffect(() => {
    for (const n of co.shell?.notes || []) toast(n.text, n.warn ? 'error' : 'info');
  }, [co.shell?.gen]);
  if (co.tab === PLANNING && S.rf.open) return html`<div class="rf-stage"><${RosterFlowPage} /></div>`;
  const Page = PAGES[co.tab] || Dashboard;
  return html`<div class="shell"><${SideBar} /><main class="main" key=${String(co.tab)}><${Page} /></main></div>`;
}

function App() {
  useStore();
  useLang();
  const page = S.page;
  let body;
  if (page === 'company' && S.co) body = html`<${CompanyScreen} />`;
  else if (page === 'start') body = html`<${StartPage} />`;
  else if (page === 'failed') body = html`<div class="start"><div class="start-card"><h1>RosterFlow</h1><p class="bad">${S.failed}</p></div></div>`;
  else body = html`<div class="splash"><div class="spinner"></div></div>`;
  return html`${body}${S.co?.popup && html`<${Popup} p=${S.co.popup} />`}<${Notices} /><${Toasts} />`;
}

// (keys the planner presses anywhere: Ctrl+, the settings)
addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === ',' && S.page === 'company') {
    e.preventDefault();
    goTab('settings');
  }
});

export { handOver, rfOpen };

render(html`<${App} />`, document.getElementById('app'));
boot();
