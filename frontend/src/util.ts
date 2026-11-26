import { zipSync } from "fflate";
import type { Category } from "./api";

export const CATEGORY_LABELS: Record<Category, string> = {
  image: "Images",
  audio: "Audio",
  video: "Video",
  document: "Documents",
  spreadsheet: "Spreadsheets",
  presentation: "Presentations",
  ebook: "Ebooks",
};

export function formatBytes(n: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

let counter = 0;
export const newId = () => `${Date.now()}-${counter++}`;

/** Trigger a browser download for a blob. */
export function saveBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

/** Bundle several files into one .zip (in the browser, nothing is uploaded). */
export async function zipBlobs(files: { name: string; blob: Blob }[]): Promise<Blob> {
  const entries: Record<string, Uint8Array> = {};
  for (const { name, blob } of files) {
    entries[uniqueName(name, entries)] = new Uint8Array(await blob.arrayBuffer());
  }
  // level 0 = store only: converted media is already compressed.
  const zipped = zipSync(entries, { level: 0 });
  return new Blob([zipped.buffer as ArrayBuffer], { type: "application/zip" });
}

/** "a.png", "a.png" -> "a.png", "a (2).png" */
function uniqueName(name: string, taken: Record<string, unknown>): string {
  if (!(name in taken)) return name;
  const dot = name.lastIndexOf(".");
  const [stem, ext] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  for (let i = 2; ; i++) {
    const candidate = `${stem} (${i})${ext}`;
    if (!(candidate in taken)) return candidate;
  }
}

/** Run `task` over `items`, at most `limit` at a time. */
export async function runLimited<T>(items: T[], limit: number, task: (item: T) => Promise<void>) {
  const queue = [...items];
  const worker = async () => {
    for (let item = queue.shift(); item !== undefined; item = queue.shift()) await task(item);
  };
  await Promise.all(Array.from({ length: Math.min(limit, queue.length) }, worker));
}
