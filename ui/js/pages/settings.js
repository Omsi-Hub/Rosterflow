// The settings: the company (its name, mark, depots, country and currency), its calendar (the
// school holidays its school lines keep to, public holidays besides the country's), the rules of
// its staff, the program's own (language, look), and its data (a company file saved, a day's
// copy put back).

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { setLanguage } from '../i18n.js';
import { applyTheme } from '../hub/theme.js';
import { act, cmd, useView } from '../company/kit.js';
import { S, set as setStore, toast } from '../store.js';
import { Page, Btn, Field, Text, Num, Choice, Check, Row, Sub, Note } from './parts.js';

export function SettingsPage() {
  const v = useView('md_company');
  const [c, setC] = useState(null);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    if (v && !v.error && !dirty) setC(structuredClone(v));
  }, [v]);
  if (!c) return html`<${Page} title=${tr('Settings')}><p class="dim">${tr('Reading…')}</p></${Page}>`;
  const set = (patch) => {
    setC((x) => ({ ...x, ...patch }));
    setDirty(true);
  };
  const save = async () => {
    setSaving(true);
    const patch = { ...c, colour: c.colours[0], colour2: c.colours[1] };
    const r = await act('md_company_set', { patch });
    setSaving(false);
    if (r?.ok) setDirty(false);
  };
  const tools = html`${dirty && html`<${Btn} label=${tr('Discard')} kind="ghost" onClick=${() => { setDirty(false); setC(structuredClone(v)); }} />`}
    <${Btn} label=${saving ? tr('Saving…') : tr('Save')} icon="check" kind="primary" disabled=${!dirty || saving} onClick=${save} />`;
  const boot = S.boot || {};
  return html`<${Page} title=${tr('Settings')} tools=${tools}>
    <div class="pg-cards">
      <div class="pg-card">
        <${Sub} text=${tr('Company')} />
        <${Row}><${Field} label=${tr('Name')} grow=${true}><${Text} value=${c.name} onChange=${(x) => set({ name: x })} /></${Field}>
          <${Field} label=${tr('Short name')} w="110px"><${Text} value=${c.short} onChange=${(x) => set({ short: x })} /></${Field}></${Row}>
        <${Row}><${Field} label=${tr('Colour')}><input class="pg-colour" type="color" value=${c.colours[0]} onInput=${(e) => set({ colours: [e.currentTarget.value, c.colours[1]] })} /></${Field}>
          <${Field} label=${tr('Second colour')}><input class="pg-colour" type="color" value=${c.colours[1]} onInput=${(e) => set({ colours: [c.colours[0], e.currentTarget.value] })} /></${Field}></${Row}>
        <${Row}><${Field} label=${tr('Country (public holidays)')} grow=${true}><${Choice} value=${c.country} options=${boot.countries || []} onChange=${(x) => set({ country: x })} /></${Field}>
          <${Field} label=${tr('Currency')} grow=${true}><${Choice} value=${c.currency} options=${boot.currencies || []} onChange=${(x) => set({ currency: x })} /></${Field}></${Row}>
        <${Field} label=${tr('Main depot')} hint=${tr('Where the buses start and end their day: the empty runs of the lines go from and to it.')}><${Text} value=${c.depot} onChange=${(x) => set({ depot: x })} /></${Field}>
        <${ListEdit} label=${tr('Other depots')} items=${c.depots.filter((d) => d.name !== c.depot)} blank=${{ name: '', address: '' }}
          fields=${[['name', tr('Name')], ['address', tr('Address')]]} onChange=${(list) => set({ depots: list })} />
      </div>
      <div class="pg-card">
        <${Sub} text=${tr('Staff rules')} />
        <${Row}><${Field} label=${tr('Days a week a driver works at most')} grow=${true}><${Num} value=${c.week_days} min=${1} max=${7} onChange=${(x) => set({ week_days: x })} /></${Field}>
          <${Field} label=${tr('Hours of a full-time week')} grow=${true}><${Num} value=${c.full_week} min=${1} max=${80} step=${0.5} onChange=${(x) => set({ full_week: x })} /></${Field}></${Row}>
        <${Field} label=${tr('Drivers off on holiday on one day at most')} hint=${tr('Empty: a quarter of the fleet (now %{n}).', { n: c.max_off_default })}>
          <input class="pg-in" type="number" min="1" max="99" value=${c.max_off ?? ''} onInput=${(e) => set({ max_off: e.currentTarget.value === '' ? null : Number(e.currentTarget.value) })} /></${Field}>
        <${Check} checked=${c.type_training} label=${tr('Drivers need a type training for each bus model')} onChange=${(x) => set({ type_training: x })} />
        <${Note} text=${tr('The working-time rules (duty length, breaks, rest between duties) are RosterFlow\'s preferences: open the Planning and choose Preferences.')} />
      </div>
      <div class="pg-card">
        <${Sub} text=${tr('Calendar')} />
        <p class="dim small">${tr('Enter a period as 2026-07-18 to 2026-08-30, or for every year as 07-18 to 08-30. School lines do not run in the school holidays; on public holidays the Sunday timetable runs.')}</p>
        <${ListEdit} label=${tr('School holidays')} items=${c.school_holidays} blank=${{ name: '', from: '', to: '' }} fields=${[['name', tr('Name')], ['from', tr('From')], ['to', tr('To')]]} onChange=${(list) => set({ school_holidays: list })} />
        <${ListEdit} label=${tr('Public holidays besides the country\'s')} items=${c.public_holidays} blank=${{ name: '', from: '', to: '' }} fields=${[['name', tr('Name')], ['from', tr('From')], ['to', tr('To')]]} onChange=${(list) => set({ public_holidays: list })} />
      </div>
      <${ProgramCard} />
      <${DataCard} />
    </div>
  </${Page}>`;
}

/** A list of small records edited in place: a row each, one added at its foot. */
function ListEdit({ label, items, blank, fields, onChange }) {
  const put = (k, patch) => onChange(items.map((x, i) => (i === k ? { ...x, ...patch } : x)));
  return html`<div class="pg-listedit">
    <span class="pg-label">${label}</span>
    ${items.map((it, k) => html`<div class="pg-row tight" key=${k}>
      ${fields.map(([f, l]) => html`<input class="pg-in" placeholder=${l} value=${it[f] || ''} onInput=${(e) => put(k, { [f]: e.currentTarget.value })} />`)}
      <${Btn} small=${true} kind="ghost" icon="close" title=${tr('Take out')} onClick=${() => onChange(items.filter((_, i) => i !== k))} />
    </div>`)}
    <${Btn} small=${true} kind="ghost" icon="add" label=${tr('Add')} onClick=${() => onChange([...items, { ...blank }])} />
  </div>`;
}

/** The program's own settings: its language and its look. */
function ProgramCard() {
  const s = S.settings || {};
  const langs = S.boot?.languages || [];
  const change = async (patch) => {
    const set = await cmd('app_settings', { patch }).catch((e) => (toast(e.message, 'error'), null));
    if (!set) return;
    setStore({ settings: set });
    if ('language' in patch) await setLanguage(set.language);
    if ('theme' in patch) applyTheme();
  };
  return html`<div class="pg-card">
    <${Sub} text=${tr('Program')} />
    <${Field} label=${tr('Language')}><${Choice} value=${s.language || ''} options=${[['', tr('As the computer')], ...langs]} onChange=${(x) => change({ language: x })} /></${Field}>
    <${Field} label=${tr('Look')}><${Choice} value=${s.theme || ''} options=${[['', tr('As the computer')], ['light', tr('Light')], ['dark', tr('Dark')]]} onChange=${(x) => change({ theme: x })} /></${Field}>
  </div>`;
}

/** The company's data: saved as a file, a day's copy put back, where it is kept. */
function DataCard() {
  const backups = useView('app_backups');
  const [day, setDay] = useState('');
  return html`<div class="pg-card">
    <${Sub} text=${tr('Data')} />
    <p class="dim small">${tr('The company is saved after every change. A copy of each day it changed is kept for 30 days.')}</p>
    <div class="pg-row"><${Btn} icon="save" label=${tr('Save the company as a file')} onClick=${() => act('app_export', {})} /></div>
    ${Array.isArray(backups) && backups.length > 0 && html`<div class="pg-row">
      <${Field} label=${tr('Put the company back as it was on')} grow=${true}><${Choice} value=${day || backups[0]} options=${backups.map((d) => [d, d])} onChange=${setDay} /></${Field}>
      <span class="pg-inline"><${Btn} kind="danger" label=${tr('Put back')} onClick=${() => confirm(tr('Put the company back as it was on %{day}? What changed since is kept as a copy of its own.', { day: day || backups[0] })) && act('app_restore', { day: day || backups[0] })} /></span>
    </div>`}
    <p class="dim small">${tr('Data folder')}: <code class="selectable">${S.boot?.data || ''}</code></p>
  </div>`;
}
