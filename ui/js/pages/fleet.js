// The fleet: every bus with its fleet number, plate, make and model, its size and drive (who may
// drive it), its seats, the depot it stands at, when it is the company's, and when it is in the
// workshop (then it is not planned). Entered one by one or taken from a spreadsheet.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { act, cmd, useView, ui, setUi } from '../company/kit.js';
import { S } from '../store.js';
import { Page, Btn, Table, Side, Field, Text, Num, Day, Choice, Row, Sub, Tag, Note } from './parts.js';
import { importList, exportList } from './io.js';

export function FleetPage() {
  const v = useView('md_fleet');
  const [opts, setOpts] = useState(null);
  const [edit, setEdit] = useState(null);
  useEffect(() => {
    cmd('md_options').then(setOpts).catch(() => {});
  }, []);
  // (a bus named elsewhere - RosterFlow's service panel - opened at once)
  const want = ui('fleet').open;
  useEffect(() => {
    if (want == null || !v?.fleet) return;
    const x = v.fleet.find((f) => f.vehicle.id === want);
    if (x) setEdit({ ...x.vehicle });
    setUi('fleet', { open: null });
  }, [want, v]);
  const rows = (v?.fleet || []).map((x) => ({ ...x.vehicle, _: x }));
  const blank = () => ({ id: 0, number: v?.next_number || '', plate: '', name: '', kind: { size: 'solo', drive: 'diesel' }, seats: 0, standing: 0, built: '', from: '', until: '', workshop_until: null, depot: '', notes: '' });
  const tools = html`<${Btn} label=${tr('Import')} icon="upload" kind="ghost" onClick=${() => importList('fleet')} />
    <${Btn} label=${tr('Export')} icon="download" kind="ghost" onClick=${() => exportList('fleet')} />
    <${Btn} label=${tr('New bus')} icon="add" kind="primary" onClick=${() => setEdit(blank())} />`;
  return html`<${Page} title=${tr('Fleet')} sub=${tr('Your buses. A bus in the workshop is not planned until the day after it is ready.')} tools=${tools}>
    ${v?.error && html`<${Note} tone="danger" text=${v.error} />`}
    <${Table} rows=${rows} text=${(r) => `${r.number} ${r.plate} ${r.name}`} onOpen=${(r) => setEdit({ ...r, _: undefined })}
      empty=${tr('No buses yet. Add them one by one, or import a spreadsheet with the columns Number, Plate, Model, Size, Drive and Seats.')}
      cols=${[
        { key: 'number', label: tr('Number'), w: '90px', render: (r) => html`<b>${r.number}</b>` },
        { key: 'plate', label: tr('Plate'), w: '120px' },
        { key: 'name', label: tr('Model') },
        { label: tr('Kind'), render: (r) => `${r._.kind}${r.kind.drive !== 'diesel' ? ' · ' + r._.drive : ''}` },
        { label: tr('Seats'), w: '80px', align: 'r', render: (r) => (r.seats ? `${r.seats}${r.standing ? ' + ' + r.standing : ''}` : '') },
        { label: tr('Drivers'), w: '90px', align: 'r', render: (r) => r._.drivers },
        { label: tr('Status'), w: '140px', render: (r) => (!r._.held ? html`<${Tag} text=${tr('Not in the fleet')} />` : r._.workshop ? html`<${Tag} text=${tr('In the workshop')} tone="warn" />` : html`<${Tag} text=${tr('In service')} tone="ok" />`) },
      ]} />
    ${edit && opts && html`<${BusForm} b=${edit} opts=${opts} depots=${v?.depots || []} onClose=${() => setEdit(null)} />`}
  </${Page}>`;
}

function BusForm({ b: start, opts, depots, onClose }) {
  const [b, setB] = useState(start);
  const [saving, setSaving] = useState(false);
  const set = (patch) => setB((x) => ({ ...x, ...patch }));
  const save = async () => {
    setSaving(true);
    const r = await act('md_vehicle_save', { vehicle: b });
    setSaving(false);
    if (r?.ok) onClose();
  };
  const remove = async () => {
    if (!confirm(tr('Take bus %{n} out of the fleet? To keep its history, give it a last day instead.', { n: b.number }))) return;
    if ((await act('md_vehicle_delete', { id: b.id }))?.ok) onClose();
  };
  const today = S.boot?.today || '';
  return html`<${Side} title=${b.id ? `${tr('Bus')} ${b.number}` : tr('New bus')} onClose=${onClose} onSave=${save} saving=${saving}
    extra=${b.id ? html`<${Btn} label=${tr('Take out')} icon="delete" kind="danger" onClick=${remove} />` : null}>
    <${Row}><${Field} label=${tr('Fleet number')} w="140px"><${Text} value=${b.number} autofocus=${!b.id} onChange=${(x) => set({ number: x })} /></${Field}>
      <${Field} label=${tr('Plate')} grow=${true}><${Text} value=${b.plate} onChange=${(x) => set({ plate: x })} /></${Field}></${Row}>
    <${Field} label=${tr('Make and model')} hint=${tr('Drivers need a type training per model when the company keeps them (Settings).')}><${Text} value=${b.name} placeholder="VDL Citea SLF-120" onChange=${(x) => set({ name: x })} /></${Field}>
    <${Row}><${Field} label=${tr('Size')} grow=${true}><${Choice} value=${b.kind.size} options=${opts.sizes} onChange=${(x) => set({ kind: { ...b.kind, size: x } })} /></${Field}>
      <${Field} label=${tr('Drive')} grow=${true}><${Choice} value=${b.kind.drive} options=${opts.drives} onChange=${(x) => set({ kind: { ...b.kind, drive: x } })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Seats')} grow=${true}><${Num} value=${b.seats} max=${200} onChange=${(x) => set({ seats: x })} /></${Field}>
      <${Field} label=${tr('Standing places')} grow=${true}><${Num} value=${b.standing} max=${250} onChange=${(x) => set({ standing: x })} /></${Field}>
      <${Field} label=${tr('Built')} grow=${true}><${Text} value=${b.built} placeholder="2021" onChange=${(x) => set({ built: x })} /></${Field}></${Row}>
    <${Field} label=${tr('Depot')}><${Choice} value=${b.depot} options=${[['', tr('The main depot')], ...depots.filter(Boolean).map((d) => [d, d])]} onChange=${(x) => set({ depot: x })} /></${Field}>
    <${Sub} text=${tr('Availability')} />
    <${Row}><${Field} label=${tr('In the fleet from')} grow=${true}><${Day} value=${b.from} onChange=${(x) => set({ from: x })} /></${Field}>
      <${Field} label=${tr('Last day in the fleet')} grow=${true}><${Day} value=${b.until} onChange=${(x) => set({ until: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('In the workshop up to and including')} grow=${true} hint=${tr('Empty: available.')}><${Day} value=${b.workshop_until || ''} onChange=${(x) => set({ workshop_until: x || null })} /></${Field}>
      <span class="pg-inline">${b.workshop_until && b.workshop_until >= today ? html`<${Btn} small=${true} label=${tr('Ready again')} kind="ghost" onClick=${() => set({ workshop_until: null })} />` : html`<${Btn} small=${true} label=${tr('To the workshop today')} kind="ghost" onClick=${() => set({ workshop_until: today })} />`}</span></${Row}>
    <${Field} label=${tr('Notes')}><${Text} value=${b.notes} onChange=${(x) => set({ notes: x })} /></${Field}>
  </${Side}>`;
}
