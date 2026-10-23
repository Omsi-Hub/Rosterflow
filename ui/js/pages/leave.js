// Holiday requests: entered for a driver, answered by the planner with RosterFlow's advice beside
// each (how many would be off those days against the company's limit, the days the driver has
// left); an approved holiday keeps the driver off the roster on its days.

import { html, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { act, useView } from '../company/kit.js';
import { Page, Btn, Table, Side, Field, Text, Day, Choice, Row, Tag, Note } from './parts.js';
import { useEffect } from '../../vendor/preact-htm.js';
import { cmd } from '../company/kit.js';

const STATE = { asked: ['Waiting', 'warn'], approved: ['Approved', 'ok'], refused: ['Refused', 'danger'] };

export function LeavePage() {
  const v = useView('md_leave');
  const [adding, setAdding] = useState(false);
  const [show, setShow] = useState('open');
  const rows = (v?.requests || []).filter((r) => show === 'all' || r.state === 'asked');
  const tools = html`<${Btn} label=${tr('New request')} icon="add" kind="primary" onClick=${() => setAdding(true)} />`;
  const filters = html`<div class="pg-seg">${[['open', tr('To answer')], ['all', tr('All')]].map(([k, l]) => html`<button type="button" class=${show === k ? 'on' : ''} onClick=${() => setShow(k)}>${l}</button>`)}</div>`;
  return html`<${Page} title=${tr('Holiday requests')} sub=${v ? tr('At most %{n} drivers off on holiday on one day (Settings).', { n: v.limit }) : ''} tools=${tools}>
    ${v?.error && html`<${Note} tone="danger" text=${v.error} />`}
    <${Table} rows=${rows} filters=${filters} text=${(r) => `${r.name} ${r.reason_label} ${r.from}`}
      empty=${show === 'open' ? tr('No requests wait for an answer.') : tr('No holiday requests yet.')}
      cols=${[
        { key: 'name', label: tr('Name'), render: (r) => html`<b>${r.name}</b>` },
        { label: tr('Days'), render: (r) => html`${r.from_label}${r.until !== r.from ? html` – ${r.until_label}` : ''} <span class="dim">(${tr('%{n} working days', { n: r.days })})</span>` },
        { label: tr('Why'), render: (r) => html`${r.reason_label}${r.note ? html`<div class="dim small">${r.note}</div>` : ''}` },
        { label: tr('Advice'), w: '230px', render: (r) => (r.state === 'asked' ? html`<span class=${r.advice ? 'ok' : 'warn'}>${r.advice_text}</span>` : '') },
        { label: tr('State'), w: '110px', render: (r) => html`<${Tag} text=${tr(STATE[r.state][0])} tone=${STATE[r.state][1]} />` },
        {
          label: '',
          w: '230px',
          align: 'r',
          render: (r) => html`<span class="pg-acts" onClick=${(e) => e.stopPropagation()}>
            ${r.state === 'asked' && html`<${Btn} small=${true} kind="primary" label=${tr('Approve')} onClick=${() => act('md_leave_answer', { no: r.no, approve: true })} />
              <${Btn} small=${true} kind="ghost" label=${tr('Refuse')} onClick=${() => act('md_leave_answer', { no: r.no, approve: false })} />`}
            <${Btn} small=${true} kind="ghost" icon="delete" title=${tr('Take out')} onClick=${() => confirm(tr('Take this request out?')) && act('md_leave_remove', { no: r.no })} />
          </span>`,
        },
      ]} />
    ${adding && v && html`<${RequestForm} staff=${v.staff} onClose=${() => setAdding(false)} />`}
  </${Page}>`;
}

function RequestForm({ staff, onClose }) {
  const [opts, setOpts] = useState(null);
  const [r, setR] = useState({ id: staff[0]?.[0] ?? 0, from: '', until: '', reason: 'break', note: '' });
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    cmd('md_options').then(setOpts).catch(() => {});
  }, []);
  const set = (patch) => setR((x) => ({ ...x, ...patch }));
  const save = async () => {
    setSaving(true);
    const x = await act('md_leave_add', r);
    setSaving(false);
    if (x?.ok) onClose();
  };
  return html`<${Side} title=${tr('New holiday request')} onClose=${onClose} onSave=${save} saving=${saving}>
    <${Field} label=${tr('Employee')}><${Choice} value=${r.id} options=${staff.map(([id, n]) => [id, n])} onChange=${(x) => set({ id: Number(x) })} /></${Field}>
    <${Row}><${Field} label=${tr('From')} grow=${true}><${Day} value=${r.from} onChange=${(x) => set({ from: x })} /></${Field}>
      <${Field} label=${tr('Up to and including')} grow=${true}><${Day} value=${r.until} onChange=${(x) => set({ until: x })} /></${Field}></${Row}>
    ${opts && html`<${Field} label=${tr('Why')}><${Choice} value=${r.reason} options=${opts.reasons} onChange=${(x) => set({ reason: x })} /></${Field}>`}
    <${Field} label=${tr('Note')}><${Text} value=${r.note} onChange=${(x) => set({ note: x })} /></${Field}>
  </${Side}>`;
}
