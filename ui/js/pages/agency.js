// Agency drivers: booked ahead for a day (from tomorrow, up to six weeks), for set hours or the
// whole day; RosterFlow's roster gives them the open duties of their day. A booking is cancelled
// free before its day.

import { html, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { act, useView } from '../company/kit.js';
import { Page, Btn, Table, Side, Field, Text, Num, Day, Row, Tag, Note } from './parts.js';

export function AgencyPage() {
  const v = useView('md_agency');
  const [booking, setBooking] = useState(false);
  const rows = v?.bookings || [];
  const tools = html`<${Btn} label=${tr('Book agency drivers')} icon="add" kind="primary" onClick=${() => setBooking(true)} />`;
  return html`<${Page} title=${tr('Agency drivers')} sub=${v ? tr('Booked at %{rate} an hour. The roster gives them the open duties of their day.', { rate: v.rate }) : ''} tools=${tools}>
    ${v?.error && html`<${Note} tone="danger" text=${v.error} />`}
    <${Table} rows=${rows} text=${(r) => `${r.label} ${r.agency}`} empty=${tr('No agency drivers booked.')}
      cols=${[
        { key: 'label', label: tr('Day'), render: (r) => html`<b>${r.label}</b>` },
        { key: 'agency', label: tr('Agency') },
        { label: tr('Drivers'), w: '90px', align: 'r', render: (r) => r.count },
        { label: tr('Hours'), w: '130px', render: (r) => r.hours || tr('Whole day') },
        { label: tr('Duties given'), w: '120px', align: 'r', render: (r) => r.duties },
        { label: tr('Cost'), w: '110px', align: 'r', render: (r) => r.cost },
        { label: '', w: '150px', align: 'r', render: (r) => (r.past || r.settled ? html`<${Tag} text=${tr('Done')} />` : html`<${Btn} small=${true} kind="ghost" label=${tr('Cancel')} onClick=${() => confirm(tr('Cancel this booking?')) && act('md_agency_cancel', { no: r.no })} />`) },
      ]} />
    ${booking && v && html`<${BookForm} v=${v} onClose=${() => setBooking(false)} />`}
  </${Page}>`;
}

function BookForm({ v, onClose }) {
  const [b, setB] = useState({ date: v.first, count: 1, from: '', to: '', agency: v.agencies[0] || '' });
  const [saving, setSaving] = useState(false);
  const set = (patch) => setB((x) => ({ ...x, ...patch }));
  const save = async () => {
    setSaving(true);
    const r = await act('md_agency_book', b);
    setSaving(false);
    if (r?.ok) onClose();
  };
  return html`<${Side} title=${tr('Book agency drivers')} onClose=${onClose} onSave=${save} saving=${saving} saveLabel=${tr('Book')}>
    <${Row}><${Field} label=${tr('Day')} grow=${true} hint=${tr('From tomorrow up to six weeks ahead.')}><${Day} value=${b.date} onChange=${(x) => set({ date: x })} /></${Field}>
      <${Field} label=${tr('Drivers')} w="120px"><${Num} value=${b.count} min=${1} max=${99} onChange=${(x) => set({ count: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('From (HH:MM)')} grow=${true} hint=${tr('Empty: the whole day.')}><${Text} value=${b.from} placeholder="06:00" onChange=${(x) => set({ from: x })} /></${Field}>
      <${Field} label=${tr('To (HH:MM)')} grow=${true}><${Text} value=${b.to} placeholder="14:00" onChange=${(x) => set({ to: x })} /></${Field}></${Row}>
    <${Field} label=${tr('Agency')}><${Text} value=${b.agency} onChange=${(x) => set({ agency: x })} /></${Field}>
  </${Side}>`;
}
