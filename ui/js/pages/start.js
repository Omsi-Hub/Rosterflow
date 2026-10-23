// The start: the companies on this computer, the one changed last first - one opened, a new one
// made (empty, or with an example network, fleet and staff to try everything on), a company file
// opened, one deleted.

import { html, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { cmd } from '../company/kit.js';
import { S, set, toast } from '../store.js';
import { Btn, Field, Text, Choice, Check, Row, Side, Note } from './parts.js';
import { Mark } from '../company/kit.js';
import { importCompany } from './io.js';
import { Mark as Logo } from '../rosterflow/brand.js';

/** Open company `id` (the app's screen follows: `app.js`). */
export async function openCompany(id) {
  try {
    await cmd('app_open', { id });
    await S.enter?.();
  } catch (e) {
    toast(e.message, 'error');
  }
}

export function StartPage() {
  const list = S.boot?.companies || [];
  const [making, setMaking] = useState(null);
  const reload = async () => {
    const companies = await cmd('app_companies').catch(() => list);
    set({ boot: { ...S.boot, companies } });
  };
  const remove = async (c) => {
    if (!confirm(tr('Delete the company “%{name}”? Its file goes to the trash folder of the data folder.', { name: c.name }))) return;
    await cmd('app_delete', { id: c.id }).catch((e) => toast(e.message, 'error'));
    reload();
  };
  const opened = async () => {
    const r = await importCompany();
    if (r?.id) S.enter?.();
  };
  return html`<div class="start">
    <div class="start-card">
      <div class="start-brand"><${Logo} size=${52} /><div><h1>RosterFlow</h1><p>${tr('Bus scheduling and crew rostering')}</p></div></div>
      ${list.length > 0 && html`<h2>${tr('Your companies')}</h2>
        <div class="start-list">${list.map((c) => html`<div class="start-co" key=${c.id}>
          <button type="button" class="start-open" onClick=${() => openCompany(c.id)}>
            <${Mark} mark=${{ short: c.short, colours: c.colours }} size=${42} />
            <span class="grow"><b>${c.name}</b><small>${tr('%{lines} lines · %{buses} buses · %{drivers} drivers', { lines: c.lines, buses: c.buses, drivers: c.drivers })}</small></span>
          </button>
          <button type="button" class="pg-x" title=${tr('Delete')} onClick=${() => remove(c)}>×</button>
        </div>`)}</div>`}
      ${!list.length && html`<${Note} text=${tr('Welcome! Make your company to begin: enter its stops, lines, buses and drivers, and RosterFlow plans the blocks, the duties and the roster. Or try everything first on an example company.')} />`}
      <div class="pg-row start-acts">
        <${Btn} kind="primary" icon="add" label=${tr('New company')} onClick=${() => setMaking({ demo: false })} />
        <${Btn} icon="stars" label=${tr('Example company')} onClick=${() => setMaking({ demo: true, name: tr('Example Bus Company') })} />
        <${Btn} kind="ghost" icon="folder_open" label=${tr('Open a company file')} onClick=${opened} />
      </div>
      <p class="start-foot dim small">RosterFlow ${S.boot?.version || ''}</p>
    </div>
    ${making && html`<${NewCompany} start=${making} onClose=${() => setMaking(null)} />`}
  </div>`;
}

function NewCompany({ start, onClose }) {
  const boot = S.boot || {};
  const guessCountry = { nl: 'NL', de: 'DE', fr: 'BE', en: 'GB' }[S.lang] || 'NL';
  const [f, setF] = useState({ name: '', short: '', colour: '#1f6feb', colour2: '#0f1b2d', depot: '', country: guessCountry, currency: guessCountry === 'GB' ? 'GBP' : 'EUR', ...start });
  const [saving, setSaving] = useState(false);
  const set = (patch) => setF((x) => ({ ...x, ...patch }));
  const save = async () => {
    setSaving(true);
    try {
      await cmd('app_create', f);
      onClose();
      await S.enter?.();
    } catch (e) {
      toast(e.message, 'error');
    } finally {
      setSaving(false);
    }
  };
  return html`<${Side} title=${f.demo ? tr('Example company') : tr('New company')} onClose=${onClose} onSave=${save} saving=${saving} saveLabel=${tr('Make')}>
    ${f.demo && html`<${Note} text=${tr('An example company with three lines, twelve buses and thirty-one employees, to try everything on. Every name is made up.')} />`}
    <${Row}><${Field} label=${tr('Name')} grow=${true}><${Text} value=${f.name} autofocus=${true} onChange=${(x) => set({ name: x })} /></${Field}>
      <${Field} label=${tr('Short name')} w="110px" hint=${tr('2 to 4 letters')}><${Text} value=${f.short} onChange=${(x) => set({ short: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Colour')}><input class="pg-colour" type="color" value=${f.colour} onInput=${(e) => set({ colour: e.currentTarget.value })} /></${Field}>
      <${Field} label=${tr('Second colour')}><input class="pg-colour" type="color" value=${f.colour2} onInput=${(e) => set({ colour2: e.currentTarget.value })} /></${Field}>
      <${Field} label=${tr('Main depot')} grow=${true}><${Text} value=${f.depot} placeholder=${tr('Depot')} onChange=${(x) => set({ depot: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Country (public holidays)')} grow=${true}><${Choice} value=${f.country} options=${boot.countries || []} onChange=${(x) => set({ country: x })} /></${Field}>
      <${Field} label=${tr('Currency')} grow=${true}><${Choice} value=${f.currency} options=${boot.currencies || []} onChange=${(x) => set({ currency: x })} /></${Field}></${Row}>
    <${Check} checked=${f.demo} label=${tr('With example lines, buses and staff')} onChange=${(x) => set({ demo: x })} />
  </${Side}>`;
}
