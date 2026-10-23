// The staff: the drivers RosterFlow plans on duties and the colleagues in other jobs - their
// personnel number, contact, licence and what they may drive (endorsements, type trainings),
// their contract (hours and days a week, wage), their days off and shift preference - and who is
// ill, on holiday or on a course today. Entered one by one or taken from a spreadsheet.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { act, cmd, useView, ui, setUi } from '../company/kit.js';
import { Page, Btn, Table, Side, Field, Text, Num, Day, Choice, Check, Chips, Row, Sub, Tag, Note } from './parts.js';
import { importList, exportList } from './io.js';

const TONE = { in: 'ok', sick: 'danger', holiday: 'accent', training: 'warn', gone: '' };

export function StaffPage() {
  const v = useView('md_staff');
  const [opts, setOpts] = useState(null);
  const [edit, setEdit] = useState(null);
  const [who, setWho] = useState('drivers');
  useEffect(() => {
    cmd('md_options').then(setOpts).catch(() => {});
  }, []);
  const want = ui('staff').open;
  useEffect(() => {
    if (want == null || !v?.staff) return;
    const x = v.staff.find((f) => f.employee.id === want);
    if (x) setEdit({ ...x.employee, wage_text: x.wage_text });
    setUi('staff', { open: null });
  }, [want, v]);
  const all = (v?.staff || []).map((x) => ({ ...x.employee, _: x }));
  const rows = all.filter((r) => who === 'all' || (who === 'drivers') === (r.role === 'driver'));
  const blank = () => ({ id: 0, name: '', number: '', phone: '', email: '', licence: 'D', licence_until: '', wage: 0, wage_text: '', contract_hours: 0, week_days_max: 0, hired: '', notice_until: null, holiday_days: 25, holiday_left: 25, endorsements: [], types: [], role: 'driver', days_off: [], shift: 'any', standby: false, notes: '' });
  const tools = html`<${Btn} label=${tr('Import')} icon="upload" kind="ghost" onClick=${() => importList('staff')} />
    <${Btn} label=${tr('Export')} icon="download" kind="ghost" onClick=${() => exportList('staff')} />
    <${Btn} label=${tr('New employee')} icon="person_add" kind="primary" onClick=${() => setEdit(blank())} />`;
  const filters = html`<div class="pg-seg">${[['drivers', tr('Drivers')], ['others', tr('Other staff')], ['all', tr('Everyone')]].map(([k, l]) => html`<button type="button" class=${who === k ? 'on' : ''} onClick=${() => setWho(k)}>${l}</button>`)}</div>`;
  return html`<${Page} title=${tr('Staff')} sub=${tr('The drivers are planned on duties by the rules of their contract and their licence. Report illness here: the roster finds a replacement.')} tools=${tools}>
    ${v?.error && html`<${Note} tone="danger" text=${v.error} />`}
    <${Table} rows=${rows} filters=${filters} text=${(r) => `${r.number} ${r.name} ${r.phone}`} onOpen=${(r) => setEdit({ ...r, wage_text: r._.wage_text, _: undefined })}
      empty=${tr('No staff yet. Add them one by one, or import a spreadsheet with the columns Number, Name, Licence, Hours a week and Hourly wage.')}
      cols=${[
        { key: 'number', label: tr('No.'), w: '80px' },
        { key: 'name', label: tr('Name'), render: (r) => html`<b>${r.name}</b>${r.standby ? html` <${Tag} text=${tr('Standby')} />` : ''}` },
        { label: tr('Job'), w: '130px', render: (r) => r._.role },
        { label: tr('Licence'), w: '120px', render: (r) => html`${r.licence}${r._.marks.map((m) => html` <${Tag} text=${m} />`)}` },
        { label: tr('Contract'), w: '130px', render: (r) => `${r._.hours} ${tr('h')} · ${r._.week_days} ${tr('days')}` },
        { label: tr('Today'), w: '190px', render: (r) => html`<${Tag} text=${r._.state_text} tone=${TONE[r._.state]} />` },
      ]} />
    ${edit && opts && html`<${PersonForm} e=${edit} opts=${opts} types=${v?.types || []} typeTraining=${v?.type_training} onClose=${() => setEdit(null)} />`}
  </${Page}>`;
}

function PersonForm({ e: start, opts, types, typeTraining, onClose }) {
  const [e, setE] = useState(start);
  const [saving, setSaving] = useState(false);
  const set = (patch) => setE((x) => ({ ...x, ...patch }));
  const save = async () => {
    setSaving(true);
    const r = await act('md_employee_save', { employee: e });
    setSaving(false);
    if (r?.ok) onClose();
  };
  const remove = async () => {
    if (!confirm(tr('Take %{name} out of the staff? To keep their history, give them a last working day instead.', { name: e.name }))) return;
    if ((await act('md_employee_delete', { id: e.id }))?.ok) onClose();
  };
  const driver = e.role === 'driver';
  const weekdays = opts.weekdays.map((d, k) => [k, d]);
  return html`<${Side} title=${e.id ? e.name : tr('New employee')} onClose=${onClose} onSave=${save} saving=${saving}
    extra=${e.id ? html`<${Btn} label=${tr('Take out')} icon="delete" kind="danger" onClick=${remove} />` : null}>
    ${e.id > 0 && html`<${Absence} e=${e} onDone=${onClose} />`}
    <${Row}><${Field} label=${tr('Name')} grow=${true}><${Text} value=${e.name} autofocus=${!e.id} onChange=${(x) => set({ name: x })} /></${Field}>
      <${Field} label=${tr('Personnel number')} w="150px"><${Text} value=${e.number} onChange=${(x) => set({ number: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Job')} grow=${true}><${Choice} value=${e.role} options=${opts.roles} onChange=${(x) => set({ role: x })} /></${Field}>
      <${Field} label=${tr('Phone')} grow=${true}><${Text} value=${e.phone} type="tel" onChange=${(x) => set({ phone: x })} /></${Field}></${Row}>
    <${Field} label=${tr('Email')}><${Text} value=${e.email} type="email" onChange=${(x) => set({ email: x })} /></${Field}>
    ${driver && html`<${Sub} text=${tr('Licence')} />
      <${Row}><${Field} label=${tr('Driving licence')} grow=${true}><${Choice} value=${e.licence} options=${opts.licences} onChange=${(x) => set({ licence: x })} /></${Field}>
        <${Field} label=${tr('Valid until (code 95)')} grow=${true} hint=${tr('Not planned after this day.')}><${Day} value=${e.licence_until} onChange=${(x) => set({ licence_until: x })} /></${Field}></${Row}>
      <${Field} label=${tr('May also drive')}><${Chips} value=${e.endorsements} options=${opts.endorsements} onChange=${(x) => set({ endorsements: x })} /></${Field}>
      ${typeTraining && html`<${Field} label=${tr('Type trainings')} hint=${types.length ? '' : tr('Enter the models of your buses on the Fleet page first.')}><${Chips} value=${e.types} options=${types} onChange=${(x) => set({ types: x })} /></${Field}>`}`}
    <${Sub} text=${tr('Contract')} />
    <${Row}><${Field} label=${tr('Hours a week')} grow=${true} hint=${tr('0: full time')}><${Num} value=${e.contract_hours} max=${60} step=${0.5} onChange=${(x) => set({ contract_hours: x })} /></${Field}>
      <${Field} label=${tr('Days a week at most')} grow=${true} hint=${tr('0: the company\'s rule')}><${Num} value=${e.week_days_max} max=${7} onChange=${(x) => set({ week_days_max: x })} /></${Field}>
      <${Field} label=${tr('Hourly wage')} grow=${true} hint=${tr('Empty: the rate of the preferences')}><${Text} value=${e.wage_text} onChange=${(x) => set({ wage_text: x })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Employed from')} grow=${true}><${Day} value=${e.hired} onChange=${(x) => set({ hired: x })} /></${Field}>
      <${Field} label=${tr('Last working day')} grow=${true}><${Day} value=${e.notice_until || ''} onChange=${(x) => set({ notice_until: x || null })} /></${Field}></${Row}>
    <${Row}><${Field} label=${tr('Holiday days a year')} grow=${true}><${Num} value=${e.holiday_days} max=${60} onChange=${(x) => set({ holiday_days: x })} /></${Field}>
      <${Field} label=${tr('Holiday days left')} grow=${true}><${Num} value=${e.holiday_left} max=${99} onChange=${(x) => set({ holiday_left: x })} /></${Field}></${Row}>
    ${driver && html`<${Sub} text=${tr('Planning')} />
      <${Field} label=${tr('Never works on')}><${Chips} value=${e.days_off} options=${weekdays} onChange=${(x) => set({ days_off: x })} /></${Field}>
      <${Field} label=${tr('Shifts')}><${Choice} value=${e.shift} options=${opts.shifts} onChange=${(x) => set({ shift: x })} /></${Field}>
      <${Check} checked=${e.standby} label=${tr('Standby driver: put on the open duties of the day first')} onChange=${(x) => set({ standby: x })} />`}
    <${Field} label=${tr('Notes')}><${Text} value=${e.notes} onChange=${(x) => set({ notes: x })} /></${Field}>
  </${Side}>`;
}

/** Ill, fit again, on holiday or on a course: reported at once (not with Save). */
function Absence({ e, onDone }) {
  const [kind, setKind] = useState(null);
  const [until, setUntil] = useState('');
  const send = async (k, u = '') => {
    const r = await act('md_absence', { id: e.id, kind: k, until: u });
    if (r?.ok) onDone();
  };
  const ill = !!e.sick_open || !!e.sick_until;
  return html`<div class="pg-absence">
    <div class="pg-row">
      ${ill ? html`<${Btn} small=${true} icon="check_circle" label=${tr('Report fit')} onClick=${() => send('fit')} />` : html`<${Btn} small=${true} icon="sick" label=${tr('Report ill')} onClick=${() => setKind('sick')} />`}
      <${Btn} small=${true} kind="ghost" icon="event_busy" label=${tr('Holiday until…')} onClick=${() => setKind('holiday')} />
      <${Btn} small=${true} kind="ghost" icon="school" label=${tr('Course until…')} onClick=${() => setKind('training')} />
    </div>
    ${kind && html`<div class="pg-row">
      <${Field} label=${kind === 'sick' ? tr('Ill up to and including (empty: until reported fit)') : tr('Up to and including')} grow=${true}><${Day} value=${until} onChange=${setUntil} /></${Field}>
      <span class="pg-inline"><${Btn} small=${true} kind="primary" label=${tr('Report')} onClick=${() => send(kind, until)} /></span>
    </div>`}
  </div>`;
}
