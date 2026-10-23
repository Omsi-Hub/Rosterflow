// The pieces the master data's pages are built of: a page's head with its tools, a table with a
// search over it whose rows open a form, a form's fields (text, number, date, choice, switch,
// chips of several), a side panel to edit one record in, and a confirmation.

import { html, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { Icon } from '../company/kit.js';

/** A page: its title and the line under it, tools on the right, then its content. */
export function Page({ title, sub, tools, children, wide }) {
  return html`<section class=${'pg' + (wide ? ' wide' : '')}>
    <header class="pg-head"><div class="grow"><h1>${title}</h1>${sub && html`<p>${sub}</p>`}</div>${tools && html`<div class="pg-tools">${tools}</div>`}</header>
    ${children}
  </section>`;
}

/** A button: `kind` primary, ghost or danger. */
export function Btn({ label, icon, kind = '', onClick, disabled, title, small }) {
  return html`<button type="button" class=${['pg-btn', kind, small && 'small'].filter(Boolean).join(' ')} onClick=${onClick} disabled=${disabled} title=${title}>
    ${icon && html`<${Icon} name=${icon} size=${small ? 15 : 17} />`}${label && html`<span>${label}</span>`}</button>`;
}

/** A table: `cols` [{key, label, w, render(row), align}], its rows, a search over `text(row)`,
 * a row clicked `onOpen(row)`. */
export function Table({ cols, rows, text, onOpen, empty, sel, filters }) {
  const [q, setQ] = useState('');
  const shown = q ? rows.filter((r) => (text ? text(r) : JSON.stringify(r)).toLowerCase().includes(q.toLowerCase())) : rows;
  return html`<div class="pg-tablebox">
    <div class="pg-tablebar">
      <label class="pg-search"><${Icon} name="search" size=${16} /><input placeholder=${tr('Search…')} value=${q} onInput=${(e) => setQ(e.currentTarget.value)} /></label>
      ${filters}
      <span class="grow"></span>
      <span class="pg-count">${shown.length === rows.length ? rows.length : `${shown.length} / ${rows.length}`}</span>
    </div>
    <div class="pg-tablewrap">
      <table class="pg-table">
        <thead><tr>${cols.map((c) => html`<th style=${c.w ? `width:${c.w}` : ''} class=${c.align || ''}>${c.label}</th>`)}</tr></thead>
        <tbody>
          ${shown.map((r, i) => html`<tr key=${r.id ?? i} class=${(onOpen ? 'click' : '') + (sel != null && r.id === sel ? ' on' : '')} onClick=${() => onOpen?.(r)}>
            ${cols.map((c) => html`<td class=${c.align || ''}>${c.render ? c.render(r) : r[c.key]}</td>`)}
          </tr>`)}
        </tbody>
      </table>
      ${!rows.length && html`<div class="pg-empty">${empty || tr('Nothing here yet.')}</div>`}
    </div>
  </div>`;
}

/** A field: its label over what is typed or chosen, a hint under it. */
export function Field({ label, hint, children, w, grow }) {
  return html`<label class=${'pg-field' + (grow ? ' grow' : '')} style=${w ? `width:${w}` : ''}><span class="pg-label">${label}</span>${children}${hint && html`<small class="pg-hint">${hint}</small>`}</label>`;
}

export function Text({ value, onChange, placeholder, type = 'text', autofocus }) {
  return html`<input class="pg-in" type=${type} value=${value ?? ''} placeholder=${placeholder || ''} autofocus=${autofocus} onInput=${(e) => onChange(e.currentTarget.value)} />`;
}

export function Num({ value, onChange, min = 0, max = 100000, step = 1 }) {
  return html`<input class="pg-in" type="number" min=${min} max=${max} step=${step} value=${value ?? ''} onInput=${(e) => {
    const v = e.currentTarget.value;
    onChange(v === '' ? 0 : Math.max(min, Math.min(max, Number(v))));
  }} />`;
}

/** A day (`YYYY-MM-DD`; "" none). */
export function Day({ value, onChange }) {
  return html`<input class="pg-in" type="date" value=${value || ''} onInput=${(e) => onChange(e.currentTarget.value)} />`;
}

/** A choice of `options` ([value, label]). */
export function Choice({ value, options, onChange }) {
  return html`<select class="pg-in" value=${value ?? ''} onChange=${(e) => onChange(e.currentTarget.value)}>${options.map(([v, l]) => html`<option value=${v}>${l}</option>`)}</select>`;
}

/** A switch with its words. */
export function Check({ checked, onChange, label }) {
  return html`<label class="pg-check"><input type="checkbox" checked=${!!checked} onChange=${(e) => onChange(e.currentTarget.checked)} /><span>${label}</span></label>`;
}

/** Several of `options` chosen: a chip each. */
export function Chips({ value = [], options, onChange }) {
  const has = (v) => value.includes(v);
  return html`<div class="pg-chips">${options.map(([v, l]) => html`<button type="button" class=${'pg-chip' + (has(v) ? ' on' : '')} onClick=${() => onChange(has(v) ? value.filter((x) => x !== v) : [...value, v])}>${l}</button>`)}</div>`;
}

/** A panel on the right to edit one record: its title, the form, Save and the other buttons. */
export function Side({ title, onClose, onSave, saving, extra, children, saveLabel }) {
  return html`<div class="pg-scrim" onMouseDown=${(e) => e.target === e.currentTarget && onClose()}>
    <aside class="pg-side" onKeyDown=${(e) => e.key === 'Escape' && onClose()}>
      <header><h2 class="grow">${title}</h2><button type="button" class="pg-x" title=${tr('Close')} onClick=${onClose}><${Icon} name="close" size=${20} /></button></header>
      <div class="pg-sidebody">${children}</div>
      <footer>${extra}<span class="grow"></span><${Btn} label=${tr('Cancel')} kind="ghost" onClick=${onClose} /><${Btn} label=${saving ? tr('Saving…') : saveLabel || tr('Save')} kind="primary" icon="check" disabled=${saving} onClick=${onSave} /></footer>
    </aside>
  </div>`;
}

/** A row of fields side by side. */
export const Row = ({ children }) => html`<div class="pg-row">${children}</div>`;

/** A heading in a form. */
export const Sub = ({ text }) => html`<h3 class="pg-sub">${text}</h3>`;

/** A coloured tag: `tone` ok, warn, danger, accent or none. */
export const Tag = ({ text, tone = '' }) => html`<span class=${'pg-tag ' + tone}>${text}</span>`;

/** A note in a frame, with its icon. */
export const Note = ({ text, tone = '', icon = 'info', children }) => html`<div class=${'pg-note ' + tone}><${Icon} name=${icon} size=${17} /><div class="grow">${text}${children}</div></div>`;

/** A line's plate: its number on its colour. */
export const Plate = ({ number, colour }) => html`<span class="pg-plate" style=${`background:${colour || '#2a75f7'}`}>${number}</span>`;
