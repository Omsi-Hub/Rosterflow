// The duty card: a duty of the roster as the driver gets it on paper - the company, the day, the
// driver, the line, the tour and the duty's number, signing on and off, the bus, and every trip
// with its times and its stops. The Rust side says it (`rf_roster_card`); this shows it at the
// paper's width (80 mm for a receipt printer, or A4) and prints it with the window's own print.

import { render } from '../../vendor/preact-htm.js';
import { toast } from '../store.js';
import { html, useEffect, useState, tr, cmd, Btn, Seg, Dialog } from './kit.js';

const KEPT = 'rosterflow.dutycard';

function kept() {
  try {
    return { paper: '80', ...JSON.parse(localStorage.getItem(KEPT) || '{}') };
  } catch {
    return { paper: '80' };
  }
}

let host = null;

/** Open the card of `duty` (its `date`, `line`, `tour` and `duty`, counted from 0). */
export function openDutyCard(duty) {
  if (!host) {
    host = document.createElement('div');
    host.id = 'dutycard-host';
    document.body.append(host);
  }
  const close = () => render(null, host);
  render(html`<${DutyCardDialog} duty=${duty} onClose=${close} />`, host);
}

function DutyCardDialog({ duty, onClose }) {
  const [v, setV] = useState(null);
  const [paper, setPaper] = useState(kept().paper);
  useEffect(() => {
    cmd('rf_roster_card', duty)
      .then(setV)
      .catch((e) => {
        toast(e.message, 'error');
        onClose();
      });
  }, []);
  const choose = (p) => {
    setPaper(p);
    try {
      localStorage.setItem(KEPT, JSON.stringify({ paper: p }));
    } catch {
      // (no storage: the choice holds while the window is open)
    }
  };
  const print = () => {
    document.documentElement.classList.add('dc-printing');
    const done = () => document.documentElement.classList.remove('dc-printing');
    addEventListener('afterprint', done, { once: true });
    window.print();
    setTimeout(done, 1000);
  };
  const foot = html`<${Seg} value=${paper} options=${[['80', '80 mm'], ['a4', 'A4']]} onChange=${choose} /><span class="rest"></span>
    <${Btn} label=${tr('Close')} onClick=${onClose} /><${Btn} kind="primary" icon="print" label=${tr('Print')} disabled=${!v} onClick=${print} />`;
  return html`<${Dialog} icon="receipt" title=${tr('Print duty card')} w=${520} onClose=${onClose} foot=${foot}>
    ${v ? html`<${Card} v=${v} paper=${paper} />` : html`<p class="co-para">${tr('Reading…')}</p>`}
  </${Dialog}>`;
}

function Card({ v, paper }) {
  return html`<div class=${'dc-card ' + (paper === 'a4' ? 'a4' : 'slip')}>
    <div class="dc-head"><b>${v.company}</b><span>${v.date}</span></div>
    <div class="dc-big">${tr('Line')} ${v.line} · ${tr('Tour')} ${v.tour} · ${tr('Duty')} ${v.duty}</div>
    <div class="dc-row"><span>${tr('Driver')}</span><b>${v.driver}</b></div>
    <div class="dc-row"><span>${tr('Bus')}</span><b>${v.bus || '—'}</b></div>
    <div class="dc-row"><span>${tr('Sign on')}</span><b>${v.from}</b></div>
    <div class="dc-row"><span>${tr('Sign off')}</span><b>${v.to}</b></div>
    <table class="dc-trips">
      <thead><tr><th>${tr('Dep.')}</th><th>${tr('From')}</th><th>${tr('To')}</th><th>${tr('Arr.')}</th></tr></thead>
      <tbody>${v.trips.map((t) => html`<tr class=${t.empty ? 'empty' : ''}><td>${t.dep}</td><td>${t.empty ? tr('Empty run') : `${t.line} `}${t.from}</td><td>${t.to}</td><td>${t.arr}</td></tr>`)}</tbody>
    </table>
  </div>`;
}
