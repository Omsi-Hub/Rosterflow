// The page's line to Rust: `call(cmd, args)` answers a command of `rosterflow-api` as JSON,
// `callBytes` as bytes. In the desktop app the calls go through Tauri (`window.__TAURI__`, the
// app's `withGlobalTauri`); in a browser they go to the development server
// (`cargo run -p rosterflow-dev`), which serves this folder and answers `/api/call`.

const tauri = window.__TAURI__;

/** Running in the desktop app (else: the development server in a browser). */
export const inApp = !!(tauri && tauri.core);

/** A command's error: its text, also as `String(e)` and as `e.message`. */
export class CallError extends Error {
  toString() {
    return this.message;
  }
}
const failed = (e) => {
  throw e instanceof CallError ? e : new CallError(typeof e === 'string' ? e : (e && e.message) || String(e));
};

// (the window's own commands, called through `call` as well)
const OWN = {
  pick_folder: (a) => pickFolder(a.title),
  pick_file: (a) => pickFile(a.title, a.filters),
  open_url: (a) => openUrl(a.url),
  open_path: (a) => openPath(a.path),
};

async function http(path, cmd, args) {
  const r = await fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd, args }) });
  if (!r.ok) throw new CallError((await r.text()) || r.statusText);
  return path.endsWith('bytes') ? r.arrayBuffer() : r.json();
}

/** A command of `rosterflow-api` (`app_boot`, `rf_board` …); `args` is one object. Rejects with
 * the command's error text. */
export function call(cmd, args = {}) {
  if (OWN[cmd]) return Promise.resolve().then(() => OWN[cmd](args || {}));
  if (inApp) return tauri.core.invoke('call', { cmd, args }).catch(failed);
  return http('/api/call', cmd, args).catch(failed);
}

/** A command that answers bytes: an ArrayBuffer. */
export function callBytes(cmd, args = {}) {
  if (inApp) return tauri.core.invoke('call_bytes', { cmd, args }).catch(failed);
  return http('/api/bytes', cmd, args).catch(failed);
}

/** The system's folder picker: the path, or null. */
export async function pickFolder(title = '') {
  if (inApp) return tauri.core.invoke('pick_folder', { title });
  return window.prompt(title || 'Folder', '') || null;
}

/** The system's file picker: the path, or null. `filters`: [{name, extensions: ['csv']}]. */
export async function pickFile(title = '', filters = []) {
  if (inApp) return tauri.core.invoke('pick_file', { title, filters });
  return window.prompt(title || 'File', '') || null;
}

/** A web page in the system's browser. */
export async function openUrl(url) {
  if (inApp) return tauri.core.invoke('open_url', { url });
  window.open(url, '_blank', 'noopener');
}

/** A file or folder in the system's file manager. */
export async function openPath(path) {
  if (inApp) return tauri.core.invoke('open_path', { path });
  console.info('open', path);
}

/** A file the planner picks, read in the page (a spreadsheet to import): its text, or null. */
export function readLocalFile(accept = '.csv,.txt') {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return resolve(null);
      const buf = await f.arrayBuffer();
      // (a spreadsheet of an older Windows saves Latin-1)
      let text;
      try {
        text = new TextDecoder('utf-8', { fatal: true }).decode(buf);
      } catch {
        text = new TextDecoder('windows-1252').decode(buf);
      }
      resolve(text);
    };
    input.click();
  });
}
