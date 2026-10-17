/**
 * The desktop shell. When the editor runs inside the Tauri app (`desktop/`), Open, Save and
 * Export go through native file dialogs; in the browser build every function here is a no-op
 * and the toolbar keeps its `<input type=file>` and download links.
 *
 * The Tauri packages are imported lazily so the web bundle never loads them.
 */

export type FileFilter = { name: string; extensions: string[] };

export const VECTRO_FILES: FileFilter = { name: "Vectro project", extensions: ["json"] };
export const RIV_FILES: FileFilter = { name: "Rive runtime file", extensions: ["riv"] };
export const SVG_FILES: FileFilter = { name: "SVG", extensions: ["svg"] };

/** True inside the Tauri window, false in a browser. */
export function isDesktop(): boolean {
  return typeof window !== "undefined" && ("__TAURI__" in window || "__TAURI_INTERNALS__" in window);
}

/** Shows the native open dialog and reads the chosen file as text; null if the user cancelled. */
export async function openTextFile(filter: FileFilter): Promise<{ name: string; text: string } | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const { readTextFile } = await import("@tauri-apps/plugin-fs");
  const path = await open({ multiple: false, directory: false, filters: [filter] });
  if (!path) return null;
  return { name: basename(path), text: await readTextFile(path) };
}

/** Shows the native save dialog and writes the data; returns the path, or null if cancelled. */
export async function saveFile(suggestedName: string, filter: FileFilter, data: string | Uint8Array): Promise<string | null> {
  const { save } = await import("@tauri-apps/plugin-dialog");
  const { writeFile, writeTextFile } = await import("@tauri-apps/plugin-fs");
  const path = await save({ defaultPath: suggestedName, filters: [filter] });
  if (!path) return null;
  if (typeof data === "string") await writeTextFile(path, data);
  else await writeFile(path, data);
  return path;
}

function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
