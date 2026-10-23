// The company's overview: what it has (stops, lines, buses, drivers), today's and the coming
// days' planning at a glance - the tours each day, how many have their bus and drivers, what is
// still open - and what needs doing, each with the page that does it.

import { html } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { useView, goTab, handOver, Icon } from '../company/kit.js';
import { Page, Btn, Note } from './parts.js';

const GO = { stops: 'stops', lines: 'lines', fleet: 'fleet', staff: 'staff', leave: 'leave' };

export function Dashboard() {
  const v = useView('app_summary');
  if (!v) return html`<${Page} title=${tr('Overview')}><p class="dim">${tr('Reading…')}</p></${Page}>`;
  if (v.error) return html`<${Page} title=${tr('Overview')}><${Note} tone="danger" text=${v.error} /></${Page}>`;
  const n = v.counts;
  const go = (to) => (to === 'roster' ? handOver('planning', { view: 0 }) : GO[to] && goTab(GO[to]));
  const tiles = [
    ['place', tr('Stops'), n.stops, 'stops'],
    ['route', tr('Lines'), n.lines, 'lines'],
    ['directions_bus', tr('Buses'), n.buses, 'fleet', n.workshop ? tr('%{n} in the workshop', { n: n.workshop }) : ''],
    ['badge', tr('Drivers'), n.drivers, 'staff', n.absent ? tr('%{n} absent today', { n: n.absent }) : ''],
  ];
  return html`<${Page} title=${v.name} sub=${v.today} tools=${html`<${Btn} kind="primary" icon="calendar_month" label=${tr('Open the planning')} onClick=${() => handOver('planning')} />`}>
    ${v.save_error && html`<${Note} tone="danger" icon="error" text=${tr('The company could not be saved') + ': ' + v.save_error} />`}
    <div class="db-tiles">
      ${tiles.map(([icon, label, value, to, under]) => html`<button type="button" class="db-tile" onClick=${() => goTab(to)}>
        <${Icon} name=${icon} size=${22} /><b>${value}</b><span>${label}</span>${under && html`<small>${under}</small>`}</button>`)}
    </div>
    ${v.todo.length > 0 && html`<div class="pg-card">
      <h3 class="pg-sub">${tr('To do')}</h3>
      ${v.todo.map((t) => html`<button type="button" class=${'db-todo ' + t.tone} onClick=${() => go(t.go)}><${Icon} name=${t.tone === 'err' ? 'error' : 'arrow_forward'} size=${18} /><span class="grow">${t.text}</span><${Icon} name="chevron_right" size=${18} /></button>`)}
    </div>`}
    <div class="pg-card">
      <h3 class="pg-sub">${tr('The coming days')}</h3>
      <div class="db-days">
        ${v.days.map((d, k) => {
          const open = d.open + d.no_bus;
          const tone = !d.tours ? 'none' : open ? 'warn' : 'ok';
          return html`<button type="button" class=${'db-day ' + tone} onClick=${() => handOver('planning', { view: 0, day: k })}>
            <span class="dim small">${k === 0 ? tr('Today') : ''}</span>
            <b>${d.label.replace(/ \d{4}$/, '')}</b>
            <span>${d.tours ? tr('%{n} tours', { n: d.tours }) : tr('No service')}</span>
            ${d.tours > 0 && html`<span class=${'db-state ' + tone}>${open ? tr('%{n} open', { n: open }) : tr('Fully planned')}</span>`}
          </button>`;
        })}
      </div>
    </div>
    ${v.draft && html`<${Note} tone="warn" text=${tr('RosterFlow holds changes not saved yet: open the planning to save or discard them.')} />`}
  </${Page}>`;
}
