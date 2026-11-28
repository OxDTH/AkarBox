import { zipSync } from "fflate";
import type { Category, Format } from "./api";

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

/** "4.2 MB → 680 KB (−84%)" */
export function sizeChange(before: number, after: number): string {
  const pct = before > 0 ? Math.round(((after - before) / before) * 100) : 0;
  const sign = pct > 0 ? "+" : pct < 0 ? "−" : "±";
  return `${formatBytes(before)} → ${formatBytes(after)} (${sign}${Math.abs(pct)}%)`;
}

export function formatDuration(seconds: number): string {
  if (!isFinite(seconds)) return "";
  const s = Math.round(seconds);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const pad = (n: number) => String(n).padStart(2, "0");
  return h ? `${h}:${pad(m % 60)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}

export const isPdf = (f: File) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf");

/** Formats a browser can show in an <img>, <audio> or <video> tag. */
export function previewKind(format: Format | undefined): "image" | "audio" | "video" | null {
  if (!format) return null;
  if (format.category === "image") return ["png", "jpg", "webp", "gif", "bmp", "ico", "avif", "svg"].includes(format.ext) ? "image" : null;
  if (format.category === "audio") return ["mp3", "wav", "ogg", "opus", "m4a", "aac", "flac"].includes(format.ext) ? "audio" : null;
  if (format.category === "video") return ["mp4", "webm", "mov", "mkv"].includes(format.ext) ? "video" : null;
  return null;
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
