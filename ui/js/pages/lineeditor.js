// The line editor: a line's number, name and colour, the kind of service it is, its stops in
// each direction with the minutes between them, when its buses leave on each group of days (the
// first and the last departure and how often, or a headway of its own for each part of the
// day), its depot runs and the buses that may run it. It edits a copy of the line
// (`md_line`) and saves it whole (`md_line_save`): the tours whose trips changed are planned
// anew. Used by the Lines page and by RosterFlow's Routes tab, in the look of either (`ed-*`
// classes on the colours the page around them gives).

import { html, useEffect, useMemo, useRef, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { cmd, Icon } from '../company/kit.js';
import { toast } from '../store.js';

/** Minutes after midnight as "06:15" (past midnight "24:30"). */
export function hhmm(m) {
  const v = Math.max(0, Math.round(Number(m) || 0));
  return `${String(Math.floor(v / 60)).padStart(2, '0')}:${String(v % 60).padStart(2, '0')}`;
}

/** "6:15", "0615", "24:30" as minutes after midnight (null: not a time). */
export function parseTime(text) {
  const t = String(text || '').trim().replace('.', ':');
  let m = /^(\d{1,2}):(\d{2})$/.exec(t) || /^(\d{1,2})(\d{2})$/.exec(t);
  if (!m) {
    if (/^\d{1,2}$/.test(t)) m = [t, t, '0'];
    else return null;
  }
  const h = Number(m[1]);
  const mi = Number(m[2]);
  if (h > 30 || mi > 59) return null;
  return h * 60 + mi;
}

const num = (v, d = 0) => (Number.isFinite(Number(v)) ? Number(v) : d);

/** A time field: typed as text, taken when it reads as a time. */
function TimeField({ value, onChange, disabled }) {
  const [text, setText] = useState(hhmm(value));
  useEffect(() => setText(hhmm(value)), [value]);
  const take = () => {
    const m = parseTime(text);
    if (m == null) setText(hhmm(value));
    else if (m !== value) onChange(m);
  };
  return html`<input class="ed-in time" value=${text} disabled=${disabled} onInput=${(e) => setText(e.currentTarget.value)} onBlur=${take} onKeyDown=${(e) => e.key === 'Enter' && e.currentTarget.blur()} />`;
}

/** A number field: taken on leaving it. */
function NumField({ value, onChange, min = 0, max = 9999, step = 1, w = 64, disabled }) {
  return html`<input class="ed-in num" type="number" style=${`width:${w}px`} min=${min} max=${max} step=${step} value=${value} disabled=${disabled}
    onChange=${(e) => onChange(Math.max(min, Math.min(max, num(e.currentTarget.value, min))))} />`;
}

/** The editor of line `id`. `onChanged(id)`: the network changed (saved, copied, a stop made);
 * `onGone()`: the line was taken out; `onTimeplan(name)`: its timetable opened in RosterFlow;
 * `hold`: a ref the editor sets to a function that says whether there are changes not saved. */
export function LineEditor({ id, onChanged, onGone, onTimeplan, hold, compact }) {
  const [v, setV] = useState(null);
  const [line, setLine] = useState(null);
  const [opts, setOpts] = useState(null);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [dir, setDir] = useState(0);
  const load = () =>
    cmd('md_line', { id })
      .then((x) => {
        setV(x);
        setLine(structuredClone(x.line));
        setDirty(false);
      })
      .catch((e) => {
        toast(e.message, 'error');
        onGone?.();
      });
  useEffect(() => {
    load();
  }, [id]);
  useEffect(() => {
    cmd('md_options').then(setOpts).catch(() => {});
  }, []);
  if (hold) hold.current = () => dirty;
  if (!v || !line || !opts) return html`<div class="ed-wait">${tr('Reading…')}</div>`;
  const change = (patch) => {
    setLine((l) => ({ ...l, ...(typeof patch === 'function' ? patch(l) : patch) }));
    setDirty(true);
  };
  const save = async () => {
    setBusy(true);
    try {
      const r = await cmd('md_line_save', { line });
      if (r?.say?.text) toast(r.say.text, 'ok');
      await load();
      onChanged?.(id);
    } catch (e) {
      toast(e.message, 'error');
    } finally {
      setBusy(false);
    }
  };
  const copy = async () => {
    if (dirty && !confirm(tr('The changes not saved are left out of the copy. Copy the line all the same?'))) return;
    const r = await cmd('md_line_copy', { id }).catch((e) => (toast(e.message, 'error'), null));
    if (r?.id) onChanged?.(r.id);
  };
  const remove = async () => {
    if (!confirm(tr('Take line %{n} out of the network? Its tours leave the roster.', { n: line.number }))) return;
    const r = await cmd('md_line_delete', { id }).catch((e) => (toast(e.message, 'error'), null));
    if (r?.ok) {
      toast(r.say?.text || tr('The line is taken out.'), 'ok');
      onGone?.();
    }
  };
  const row = v.row;
  return html`<div class=${'ed' + (compact ? ' compact' : '')}>
    <div class="ed-bar">
      <span class="ed-plate" style=${`background:${line.colour}`}>${line.number || '?'}</span>
      <label class="ed-f"><span>${tr('Number')}</span><input class="ed-in" style="width:70px" value=${line.number} onInput=${(e) => change({ number: e.currentTarget.value })} /></label>
      <label class="ed-f grow"><span>${tr('Name')}</span><input class="ed-in" value=${line.name} placeholder=${row.caption} onInput=${(e) => change({ name: e.currentTarget.value })} /></label>
      <label class="ed-f"><span>${tr('Colour')}</span><input class="ed-colour" type="color" value=${line.colour} onInput=${(e) => change({ colour: e.currentTarget.value })} /></label>
      <label class="ed-f"><span>${tr('Kind of service')}</span>
        <select class="ed-in" value=${line.service} onChange=${(e) => change({ service: e.currentTarget.value })}>${opts.services.map(([k, l]) => html`<option value=${k}>${l}</option>`)}</select>
      </label>
      <span class="ed-tools">
        <button type="button" class="ed-btn" title=${tr('Copy the line')} onClick=${copy}><${Icon} name="content_copy" size=${16} /></button>
        <button type="button" class="ed-btn" title=${tr('Take the line out')} onClick=${remove}><${Icon} name="delete" size=${16} /></button>
        <button type="button" class="ed-btn text" disabled=${!dirty || busy} onClick=${load}>${tr('Discard')}</button>
        <button type="button" class="ed-btn solid" disabled=${!dirty || busy} onClick=${save}>${busy ? tr('Saving…') : tr('Save')}</button>
      </span>
    </div>
    <div class="ed-status">
      ${row.problems.length ? row.problems.map((p) => html`<span class="ed-tag warn"><${Icon} name="warning" size=${14} />${p}</span>`) : html`<span class="ed-tag ok"><${Icon} name="check_circle" size=${14} />${tr('The line can run')}</span>`}
      ${opts.day_groups.map((g, k) => html`<span class="ed-tag">${g}: ${tr('%{n} buses', { n: row.tours[k] || 0 })}</span>`)}
      ${dirty && html`<span class="ed-tag accent">${tr('Changes not saved')}</span>`}
      <span class="grow"></span>
      ${onTimeplan && row.stem && !row.problems.length && html`<button type="button" class="ed-btn text" disabled=${dirty} title=${dirty ? tr('Save the line first') : ''} onClick=${() => onTimeplan(row.stem)}><${Icon} name="departure_board" size=${16} />${tr('Open in the Timeplan')}</button>`}
    </div>
    <div class="ed-body">
      <section class="ed-sec">
        <h3>${tr('Route')}</h3>
        <div class="ed-dirs">
          ${[0, 1].map((d) => html`<button type="button" class=${'ed-dirtab' + (dir === d ? ' on' : '')} onClick=${() => setDir(d)}>
            ${d === 0 ? tr('Outbound') : tr('Way back')} <small>${destination(line.directions[d]) ? `→ ${destination(line.directions[d])}` : ''}</small></button>`)}
          <span class="grow"></span>
          ${dir === 1 && html`<button type="button" class="ed-btn text" onClick=${() => change((l) => ({ directions: [l.directions[0], reversed(l.directions[0])] }))}><${Icon} name="swap_horiz" size=${16} />${tr('Outbound reversed')}</button>`}
        </div>
        <${DirectionEditor} key=${dir} d=${line.directions[dir] || { stops: [], times: [] }} stops=${v.stops}
          onChange=${(d) => change((l) => { const dirs = [...l.directions]; dirs[dir] = d; return { directions: dirs }; })}
          onStopMade=${() => cmd('md_line', { id }).then((x) => setV((old) => ({ ...old, stops: x.stops })))} />
      </section>
      <section class="ed-sec">
        <h3>${tr('Departures')}</h3>
        ${line.table_on && html`<p class="ed-note"><${Icon} name="info" size=${15} />${tr('The trips of this line are edited one by one in the Timeplan: the departures below are used again only when you switch back.')}
          <button type="button" class="ed-btn text" onClick=${() => confirm(tr('Make the timetable from the departures below again? The trips edited in the Timeplan are let go.')) && change({ table_on: false, table: [] })}>${tr('Use these departures')}</button></p>`}
        <${DaysEditor} line=${line} opts=${opts} change=${change} />
      </section>
      <section class="ed-sec">
        <h3>${tr('Depot and buses')}</h3>
        <div class="ed-grid">
          <label class="ed-f"><span>${tr('Depot')}</span>
            <select class="ed-in" value=${line.depot} onChange=${(e) => change({ depot: e.currentTarget.value })}>
              <option value="">${tr('The main depot')}</option>
              ${v.depots.map((d) => html`<option value=${d}>${d}</option>`)}
            </select></label>
          <label class="ed-f"><span>${tr('Empty run from the depot (min)')}</span><${NumField} value=${line.depot_minutes} max=${180} onChange=${(x) => change({ depot_minutes: x })} /></label>
          <label class="ed-f"><span>${tr('Empty run (km)')}</span><${NumField} value=${line.depot_km} max=${300} step=${0.1} onChange=${(x) => change({ depot_km: x })} /></label>
        </div>
        <div class="ed-f"><span>${tr('Buses that may run it (none ticked: any)')}</span>
          <div class="ed-checks">${opts.sizes.map(([k, l]) => html`<label class="ed-check"><input type="checkbox" checked=${line.sizes.includes(k)}
            onChange=${(e) => change((x) => ({ sizes: e.currentTarget.checked ? [...x.sizes, k] : x.sizes.filter((s) => s !== k) }))} />${l}</label>`)}</div>
        </div>
        <label class="ed-f"><span>${tr('Name in public (optional)')}</span><input class="ed-in" value=${line.title} placeholder=${row.caption} onInput=${(e) => change({ title: e.currentTarget.value })} /></label>
      </section>
    </div>
  </div>`;
}

function destination(d) {
  if (!d || !d.stops?.length) return '';
  if (d.terminus?.trim()) return d.terminus.trim();
  return d.stops[d.stops.length - 1].name;
}

/** The way back as the outbound reversed: its stops the other way, the minutes between them the
 * same. */
function reversed(d) {
  const stops = [...(d.stops || [])].reverse();
  const t = d.times || [];
  const total = t[t.length - 1] || 0;
  const times = [...t].reverse().map((x) => total - x);
  return { ...d, terminus: '', stops, times, legs: [...(d.legs || [])].reverse(), manual_times: true };
}

/** A direction's stops in order, the minutes from each to the next, a stop added, moved or taken
 * out. */
function DirectionEditor({ d, stops, onChange, onStopMade }) {
  const times = d.times?.length === d.stops.length ? d.times : d.stops.map((_, k) => k * 2);
  const gaps = times.map((t, k) => (k === 0 ? 0 : Math.round((t - times[k - 1]) * 10) / 10));
  const put = (list, gapList) => {
    const t = [];
    gapList.forEach((g, k) => t.push(k === 0 ? 0 : t[k - 1] + Math.max(0, g)));
    onChange({ ...d, stops: list, times: t, manual_times: true, legs: list.slice(1).map((_, k) => d.legs?.[k] || { length: 0 }) });
  };
  const move = (k, by) => {
    const j = k + by;
    if (j < 0 || j >= d.stops.length) return;
    const s = [...d.stops];
    const g = [...gaps];
    [s[k], s[j]] = [s[j], s[k]];
    [g[k], g[j]] = [g[j], g[k]];
    g[0] = 0;
    put(s, g);
  };
  const removeAt = (k) => put(d.stops.filter((_, i) => i !== k), gaps.filter((_, i) => i !== k).map((g, i) => (i === 0 ? 0 : g)));
  const add = (stop, at) => {
    const s = [...d.stops];
    const g = [...gaps];
    const k = at == null ? s.length : at;
    s.splice(k, 0, { id: stop.id, name: stop.name });
    g.splice(k, 0, s.length > 1 ? 2 : 0);
    g[0] = 0;
    put(s, g);
  };
  const total = times[times.length - 1] || 0;
  return html`<div class="ed-dir">
    <div class="ed-stops">
      <div class="ed-srow head"><span>#</span><span class="grow">${tr('Stop')}</span><span class="r">${tr('Minutes from the stop before')}</span><span class="r">${tr('From the start')}</span><span></span></div>
      ${d.stops.map((s, k) => html`<div class="ed-srow" key=${k}>
        <span class="n">${k + 1}</span>
        <span class="grow cut">${s.name}${stopCode(stops, s.id) && html` <small>${stopCode(stops, s.id)}</small>`}</span>
        <span class="r">${k === 0 ? '' : html`<input class="ed-in num" type="number" min="0" step="0.5" value=${gaps[k]} onChange=${(e) => { const g = [...gaps]; g[k] = Math.max(0, num(e.currentTarget.value)); put(d.stops, g); }} />`}</span>
        <span class="r dim">${times[k]} ${tr('min')}</span>
        <span class="acts">
          <button type="button" class="ed-ib" title=${tr('Up')} disabled=${k === 0} onClick=${() => move(k, -1)}><${Icon} name="arrow_upward" size=${15} /></button>
          <button type="button" class="ed-ib" title=${tr('Down')} disabled=${k === d.stops.length - 1} onClick=${() => move(k, 1)}><${Icon} name="arrow_downward" size=${15} /></button>
          <button type="button" class="ed-ib" title=${tr('Take out')} onClick=${() => removeAt(k)}><${Icon} name="close" size=${15} /></button>
        </span>
      </div>`)}
      ${!d.stops.length && html`<p class="ed-empty">${tr('No stops yet: add the first stop of this direction below.')}</p>`}
    </div>
    <${StopAdder} stops=${stops} onAdd=${add} onStopMade=${onStopMade} />
    <div class="ed-dirfoot">
      <span>${tr('Trip time')}: <b>${total} ${tr('min')}</b></span>
      <label class="ed-f inline"><span>${tr('Destination shown')}</span><input class="ed-in" value=${d.terminus || ''} placeholder=${d.stops.length ? d.stops[d.stops.length - 1].name : ''} onInput=${(e) => onChange({ ...d, terminus: e.currentTarget.value })} /></label>
    </div>
  </div>`;
}

const stopCode = (stops, id) => stops.find((s) => s.id === id)?.code || '';

/** A stop picked by its name or code (one not there yet made on the spot), added at the end. */
function StopAdder({ stops, onAdd, onStopMade }) {
  const [text, setText] = useState('');
  const listId = useMemo(() => 'stops-' + Math.random().toString(36).slice(2), []);
  const label = (s) => (s.code ? `${s.name} (${s.code})` : s.name);
  const take = async () => {
    const t = text.trim();
    if (!t) return;
    let s = stops.find((x) => label(x) === t) || stops.find((x) => x.name.toLowerCase() === t.toLowerCase()) || stops.find((x) => x.code && x.code === t);
    if (!s) {
      if (!confirm(tr('There is no stop “%{name}” yet. Make it?', { name: t }))) return;
      const r = await cmd('md_stop_save', { stop: { name: t } }).catch((e) => (toast(e.message, 'error'), null));
      if (!r?.id) return;
      s = { id: r.id, name: t, code: '' };
      onStopMade?.();
    }
    onAdd(s);
    setText('');
  };
  return html`<div class="ed-adder">
    <input class="ed-in grow" list=${listId} value=${text} placeholder=${tr('Add a stop: its name or code')} onInput=${(e) => setText(e.currentTarget.value)} onKeyDown=${(e) => e.key === 'Enter' && take()} />
    <datalist id=${listId}>${stops.map((s) => html`<option value=${label(s)} />`)}</datalist>
    <button type="button" class="ed-btn" onClick=${take}><${Icon} name="add" size=${16} />${tr('Add')}</button>
  </div>`;
}

/** When the buses leave on each group of days: on or off, the first and last departure, how
 * often, how long at least at the end - or a headway for each part of the day. */
function DaysEditor({ line, opts, change }) {
  const own = line.back_days?.length > 0;
  const [side, setSide] = useState(0);
  const pats = side === 1 && own ? line.back_days : line.days;
  const setPat = (k, patch) =>
    change((l) => {
      const key = side === 1 && own ? 'back_days' : 'days';
      const list = [...l[key]];
      list[k] = { ...list[k], ...patch };
      return { [key]: list };
    });
  const allowed = (k) => (line.service === 'school' ? k === 0 : line.service === 'leisure' ? k > 0 : true);
  return html`<div>
    <div class="ed-dirs">
      <label class="ed-check"><input type="checkbox" checked=${own} onChange=${(e) => change((l) => ({ back_days: e.currentTarget.checked ? structuredClone(l.days) : [] }))} />${tr('The way back has departures of its own')}</label>
      ${own && html`<span class="grow"></span>${[0, 1].map((d) => html`<button type="button" class=${'ed-dirtab' + (side === d ? ' on' : '')} onClick=${() => setSide(d)}>${d === 0 ? tr('Outbound') : tr('Way back')}</button>`)}`}
    </div>
    <div class="ed-days">
      <div class="ed-drow head"><span>${tr('Days')}</span><span>${tr('Runs')}</span><span>${tr('First')}</span><span>${tr('Last')}</span><span>${tr('Every (min)')}</span><span>${tr('Layover (min)')}</span><span>${tr('Per part of the day')}</span></div>
      ${opts.day_groups.map((g, k) => {
        const p = pats[k] || {};
        const banded = (p.bands || []).length > 0;
        const ok = allowed(k);
        return html`<div class=${'ed-dgroup' + (ok ? '' : ' off')} key=${k}>
          <div class="ed-drow">
            <b>${g}</b>
            <span><input type="checkbox" checked=${p.on && ok} disabled=${!ok} onChange=${(e) => setPat(k, { on: e.currentTarget.checked })} /></span>
            <span><${TimeField} value=${p.first} disabled=${!p.on || banded} onChange=${(m) => setPat(k, { first: m })} /></span>
            <span><${TimeField} value=${p.last} disabled=${!p.on || banded} onChange=${(m) => setPat(k, { last: m })} /></span>
            <span><${NumField} value=${p.headway} min=${1} max=${600} disabled=${!p.on || banded} onChange=${(x) => setPat(k, { headway: x })} /></span>
            <span><${NumField} value=${p.layover} max=${120} disabled=${!p.on} onChange=${(x) => setPat(k, { layover: x })} /></span>
            <span><input type="checkbox" checked=${banded} disabled=${!p.on} onChange=${(e) => setPat(k, { bands: e.currentTarget.checked ? defaultBands(p) : [] })} /></span>
          </div>
          ${banded && p.on && html`<div class="ed-bands">
            ${p.bands.map((b, j) => html`<div class="ed-band" key=${j}>
              <span>${tr('From')}</span><${TimeField} value=${b.from} onChange=${(m) => setPat(k, { bands: p.bands.map((x, i) => (i === j ? { ...x, from: m } : x)) })} />
              <span>${tr('to')}</span><${TimeField} value=${b.to} onChange=${(m) => setPat(k, { bands: p.bands.map((x, i) => (i === j ? { ...x, to: m } : x)) })} />
              <span>${tr('every')}</span><${NumField} value=${b.headway} min=${1} max=${600} onChange=${(x) => setPat(k, { bands: p.bands.map((y, i) => (i === j ? { ...y, headway: x } : y)) })} /><span>${tr('min')}</span>
              <button type="button" class="ed-ib" title=${tr('Take out')} onClick=${() => setPat(k, { bands: p.bands.filter((_, i) => i !== j) })}><${Icon} name="close" size=${15} /></button>
            </div>`)}
            <button type="button" class="ed-btn text" onClick=${() => { const last = p.bands[p.bands.length - 1]; setPat(k, { bands: [...p.bands, { from: last ? last.to : 360, to: last ? Math.min(1560, last.to + 180) : 540, headway: last ? last.headway : 30, size: null }] }); }}><${Icon} name="add" size=${15} />${tr('Add a part of the day')}</button>
          </div>`}
        </div>`;
      })}
    </div>
  </div>`;
}

/** The parts of the day a pattern starts with: its own hours in three parts. */
function defaultBands(p) {
  const first = num(p.first, 360);
  const last = num(p.last, 1320) + 1;
  const h = num(p.headway, 20);
  if (last - first < 240) return [{ from: first, to: last, headway: h, size: null }];
  return [
    { from: first, to: 540, headway: Math.max(5, Math.round(h / 2)), size: null },
    { from: 540, to: 960, headway: h, size: null },
    { from: 960, to: Math.max(1080, last), headway: Math.max(5, Math.round(h / 2)), size: null },
  ];
}

/** The lines of the network as a list to pick one from, a new one made at its foot. */
export function LineList({ lines, sel, onPick, onNew }) {
  const [q, setQ] = useState('');
  const shown = lines.filter((l) => !q || `${l.number} ${l.caption}`.toLowerCase().includes(q.toLowerCase()));
  return html`<div class="ed-list">
    <div class="ed-listhead"><input class="ed-in grow" placeholder=${tr('Search…')} value=${q} onInput=${(e) => setQ(e.currentTarget.value)} />
      <button type="button" class="ed-btn solid" title=${tr('New line')} onClick=${onNew}><${Icon} name="add" size=${16} /></button></div>
    <div class="ed-items">
      ${shown.map((l) => html`<button type="button" class=${'ed-item' + (l.id === sel ? ' on' : '')} onClick=${() => onPick(l.id)}>
        <span class="ed-plate small" style=${`background:${l.colour}`}>${l.number}</span>
        <span class="grow cut"><b class="cut">${l.name || l.caption || tr('New line')}</b><small class="cut">${l.dirs.map((d) => `${d.from} → ${d.to}`).join(' · ') || tr('No stops yet')}</small></span>
        ${l.problems.length > 0 && html`<span class="ed-warn" title=${l.problems.join('\n')}><${Icon} name="warning" size=${15} /></span>`}
      </button>`)}
      ${!lines.length && html`<p class="ed-empty">${tr('No lines yet.')}</p>`}
    </div>
  </div>`;
}

/** A ref an editor fills with whether it holds changes not saved. */
export const useHold = () => useRef(() => false);
