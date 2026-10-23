// RosterFlow's Routes tab: the network's lines on the left, the line chosen in the line editor
// (`pages/lineeditor.js`) beside them - its stops in each direction, the minutes between them,
// its departures on each group of days - so that a route is made or changed without leaving
// RosterFlow. A line saved is planned anew where its trips changed; "Open in the Timeplan" takes
// it on to its trips.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { S } from '../store.js';
import { tr } from '../hub/bits.js';
import { cmd as coCmd } from '../company/kit.js';
import { LineEditor, LineList, useHold } from '../pages/lineeditor.js';
import { setRf, setPart, bump, rfGen, say, TIMEPLAN } from './state.js';

/** The title bar's tab (after `state.js`'s four). */
export const ROUTES = 4;

/** What the editor shown holds (whether it has changes not saved). */
let held = () => false;

/** The tab's changes not saved said before the tab or RosterFlow is left: true keeps them (the
 * planner chose to stay). */
export function holdChanges() {
  if (S.rf.view !== ROUTES || !held()) return false;
  return !confirm(tr('The line has changes not saved. Leave them?'));
}

/** Escape on the tab: nothing of its own to close. */
export function rtEscape() {
  return holdChanges();
}

export function RoutesTab() {
  const sel = S.rf.rt?.sel ?? null;
  const [lines, setLines] = useState(null);
  const hold = useHold();
  held = () => hold.current();
  const gen = rfGen();
  useEffect(() => {
    let gone = false;
    coCmd('md_lines')
      .then((v) => {
        if (gone) return;
        setLines(v.lines);
        if ((sel == null || !v.lines.some((l) => l.id === sel)) && v.lines.length) setPart('rt', { sel: v.lines[0].id });
      })
      .catch((e) => say(e.message, true));
    return () => {
      gone = true;
    };
  }, [gen]);
  const pick = (id) => {
    if (id === sel || (hold.current() && !confirm(tr('The line has changes not saved. Leave them?')))) return;
    setPart('rt', { sel: id });
  };
  const fresh = async () => {
    if (hold.current() && !confirm(tr('The line has changes not saved. Leave them?'))) return;
    const r = await coCmd('md_line_new', {}).catch((e) => (say(e.message, true), null));
    if (!r?.id) return;
    setPart('rt', { sel: r.id });
    bump();
  };
  const toTimeplan = (name) => setRf({ view: TIMEPLAN, tpLine: name, tpHint: null, tpReload: true, hand: null, zones: null, sel: [], card: null, menu: null, panel: null });
  return html`<div class="rf-routes ed-host rf-ed">
    <${LineList} lines=${lines || []} sel=${sel} onPick=${pick} onNew=${fresh} />
    <div class="ed-main">
      ${sel != null
        ? html`<${LineEditor} key=${sel} id=${sel} hold=${hold} compact=${true} onChanged=${(id) => { setPart('rt', { sel: id }); bump(); }} onGone=${() => { setPart('rt', { sel: null }); bump(); }} onTimeplan=${toTimeplan} />`
        : html`<div class="ed-wait">${lines && !lines.length ? tr('No lines yet: make the first one with +.') : tr('Reading…')}</div>`}
    </div>
  </div>`;
}
