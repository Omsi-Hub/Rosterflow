// The stops of the network: their code and name, where they are, whether drivers may be
// relieved there, and the lines that call at each. Entered one by one or taken from a
// spreadsheet (code, name, latitude, longitude - a GTFS `stops.txt` reads too).

import { html, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { act, useView } from '../company/kit.js';
import { Page, Btn, Table, Side, Field, Text, Num, Check, Row, Tag, Note } from './parts.js';
import { importList, exportList } from './io.js';

export function StopsPage() {
  const v = useView('md_stops');
  const [edit, setEdit] = useState(null);
  const rows = (v?.stops || []).map((x) => ({ ...x.stop, lines: x.lines }));
  const tools = html`<${Btn} label=${tr('Import')} icon="upload" kind="ghost" onClick=${() => importList('stops')} />
    <${Btn} label=${tr('Export')} icon="download" kind="ghost" onClick=${() => exportList('stops')} />
    <${Btn} label=${tr('New stop')} icon="add" kind="primary" onClick=${() => setEdit({ id: 0, code: '', name: '', lat: 0, lon: 0, relief: false, notes: '' })} />`;
  return html`<${Page} title=${tr('Stops')} sub=${tr('Every stop your lines call at. A relief point is where a driver may hand over the bus to another.')} tools=${tools}>
    ${v?.error && html`<${Note} tone="danger" text=${v.error} />`}
    <${Table} rows=${rows} text=${(r) => `${r.code} ${r.name}`} onOpen=${(r) => setEdit({ ...r })} empty=${tr('No stops yet. Add them one by one, or import a spreadsheet with the columns Code, Name, Latitude and Longitude.')}
      cols=${[
        { key: 'code', label: tr('Code'), w: '110px' },
        { key: 'name', label: tr('Name') },
        { label: tr('Lines'), render: (r) => r.lines.map((n) => html`<${Tag} text=${n} />`) },
        { label: tr('Relief point'), w: '120px', render: (r) => (r.relief ? html`<${Tag} text=${tr('Yes')} tone="ok" />` : '') },
        { label: tr('Position'), w: '170px', render: (r) => (r.lat || r.lon ? `${r.lat.toFixed(5)}, ${r.lon.toFixed(5)}` : html`<span class="dim">—</span>`) },
      ]} />
    ${edit && html`<${StopForm} s=${edit} onClose=${() => setEdit(null)} />`}
  </${Page}>`;
}

function StopForm({ s: start, onClose }) {
  const [s, setS] = useState(start);
  const [saving, setSaving] = useState(false);
  const set = (patch) => setS((x) => ({ ...x, ...patch }));
  const save = async () => {
    setSaving(true);
    const r = await act('md_stop_save', { stop: { ...s, lines: undefined } });
    setSaving(false);
    if (r?.ok) onClose();
  };
  const remove = async () => {
    if (!confirm(tr('Take the stop “%{name}” out?', { name: s.name }))) return;
    if ((await act('md_stop_delete', { id: s.id }))?.ok) onClose();
  };
  return html`<${Side} title=${s.id ? s.name : tr('New stop')} onClose=${onClose} onSave=${save} saving=${saving}
    extra=${s.id ? html`<${Btn} label=${tr('Take out')} icon="delete" kind="danger" onClick=${remove} />` : null}>
    <${Row}><${Field} label=${tr('Code')} w="130px"><${Text} value=${s.code} onChange=${(x) => set({ code: x })} /></${Field}>
      <${Field} label=${tr('Name')} grow=${true}><${Text} value=${s.name} autofocus=${true} onChange=${(x) => set({ name: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Latitude')} grow=${true}><${Num} value=${s.lat} min=${-90} max=${90} step=${0.00001} onChange=${(x) => set({ lat: x })} /></${Field}>
      <${Field} label=${tr('Longitude')} grow=${true}><${Num} value=${s.lon} min=${-180} max=${180} step=${0.00001} onChange=${(x) => set({ lon: x })} /></${Field}></${Row}>
    <${Check} checked=${s.relief} label=${tr('Relief point: drivers may change here')} onChange=${(x) => set({ relief: x })} />
    <${Field} label=${tr('Notes')}><${Text} value=${s.notes} onChange=${(x) => set({ notes: x })} /></${Field}>
    ${s.lines?.length > 0 && html`<${Note} text=${tr('Lines calling here: %{lines}', { lines: s.lines.join(', ') })} />`}
  </${Side}>`;
}
