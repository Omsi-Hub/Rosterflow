// The lines of the network: the list on the left, the line chosen in the line editor beside it
// (`lineeditor.js`). "Open in the Timeplan" takes a line on to its trips in RosterFlow.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { tr } from '../hub/bits.js';
import { cmd, handOver, ui, setUi, refresh } from '../company/kit.js';
import { S } from '../store.js';
import { toast } from '../store.js';
import { LineEditor, LineList, useHold } from './lineeditor.js';

export function LinesPage() {
  const [lines, setLines] = useState(null);
  const sel = ui('lines').sel ?? null;
  const hold = useHold();
  const gen = S.co.shell?.gen ?? 0;
  useEffect(() => {
    let gone = false;
    cmd('md_lines')
      .then((v) => {
        if (gone) return;
        setLines(v.lines);
        if ((sel == null || !v.lines.some((l) => l.id === sel)) && v.lines.length) setUi('lines', { sel: v.lines[0].id });
      })
      .catch((e) => toast(e.message, 'error'));
    return () => {
      gone = true;
    };
  }, [gen]);
  const leave = () => !hold.current() || confirm(tr('The line has changes not saved. Leave them?'));
  const pick = (id) => id !== sel && leave() && setUi('lines', { sel: id });
  const fresh = async () => {
    if (!leave()) return;
    const r = await cmd('md_line_new', {}).catch((e) => (toast(e.message, 'error'), null));
    if (!r?.id) return;
    setUi('lines', { sel: r.id });
    refresh();
  };
  return html`<section class="pg wide lines-page">
    <header class="pg-head"><div class="grow"><h1>${tr('Lines')}</h1><p>${tr('Each line: its stops in both directions with the minutes between them, and when its buses leave. RosterFlow makes the tours from them.')}</p></div></header>
    <div class="ed-host app-ed">
      <${LineList} lines=${lines || []} sel=${sel} onPick=${pick} onNew=${fresh} />
      <div class="ed-main">
        ${sel != null
          ? html`<${LineEditor} key=${sel} id=${sel} hold=${hold} onChanged=${(id) => { setUi('lines', { sel: id }); refresh(); }} onGone=${() => { setUi('lines', { sel: null }); refresh(); }} onTimeplan=${(name) => handOver('timetable=' + name)} />`
          : html`<div class="ed-wait">${lines && !lines.length ? tr('No lines yet: make the first one with +.') : tr('Reading…')}</div>`}
      </div>
    </div>
  </section>`;
}
