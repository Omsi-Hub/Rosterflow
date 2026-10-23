// Spreadsheets and company files in and out: in the app the system's own dialogs choose the
// file (the Rust side opens them); in a browser the page reads the file chosen and hands its
// text over.

import { inApp, readLocalFile } from '../api.js';
import { act } from '../company/kit.js';

/** A spreadsheet taken into the list `what` (stops, fleet, staff). */
export async function importList(what) {
  if (inApp) return act('md_import', { what });
  const text = await readLocalFile('.csv,.txt');
  if (text == null) return null;
  return act('md_import', { what, text });
}

/** The list `what` saved as a spreadsheet. */
export function exportList(what) {
  return act('md_export', { what });
}

/** A company file taken in and opened. */
export async function importCompany() {
  if (inApp) return act('app_import', {});
  const text = await readLocalFile('.json');
  if (text == null) return null;
  return act('app_import', { text });
}
