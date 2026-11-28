// Types mirror the JSON produced by the Rust server (crates/server/src/api.rs).

export type Category =
  | "image" | "audio" | "video" | "document" | "spreadsheet" | "presentation" | "ebook";

export interface Format {
  ext: string;
  name: string;
  category: Category;
}

export interface PlannedStep {
  converter: string;
  from: string;
  to: string;
}

export interface Target {
  format: Format;
  plan: { steps: PlannedStep[]; cost: number };
}

export interface Info {
  version: string;
  converters: string[];
  formats: Format[];
  pdf_tools: boolean;
  /** Formats the server can convert from. */
  inputs: string[];
  /** Formats the server can produce. */
  outputs: string[];
  /** GPU video encoder in use, e.g. "nvenc". */
  hardware_video: string | null;
}

export type Compression = "light" | "balanced" | "strong";

/** Mirrors `Options` in crates/core/src/options.rs. Omitted fields = defaults. */
export interface Options {
  quality?: number;
  max_width?: number;
  max_height?: number;
  audio_bitrate_kbps?: number;
  video_height?: number;
  strip_audio?: boolean;
  dpi?: number;
  compression?: Compression;
  target_size_kb?: number;
}

export interface Detected {
  format: Format;
  targets: Target[];
}

export interface Result {
  blob: Blob;
  fileName: string;
}

async function json<T>(res: Response): Promise<T> {
  if (!res.ok) throw new Error(await errorMessage(res));
  return res.json() as Promise<T>;
}

async function errorMessage(res: Response): Promise<string> {
  try {
    const body = await res.json();
    if (typeof body?.error === "string") return body.error;
  } catch {
    /* not JSON */
  }
  return `${res.status} ${res.statusText}`;
}

export const getInfo = () => fetch("/api/info").then((r) => json<Info>(r));

/**
 * Ask the server what a file is. Only the first 4 KB are sent: the server
 * recognises formats from their first bytes ("magic numbers") and the name.
 */
export const detect = (file: File) =>
  fetch(`/api/detect?name=${encodeURIComponent(file.name)}`, {
    method: "POST",
    body: file.slice(0, 4096),
  }).then((r) => json<Detected>(r));

export function convert(
  file: File,
  target: string,
  options: Options,
  onUploadProgress: (fraction: number) => void,
  signal?: AbortSignal,
): Promise<Result> {
  const form = new FormData();
  form.append("target", target); // small fields first, so the server has them early
  form.append("options", JSON.stringify(options));
  form.append("file", file);
  return upload("/api/convert", form, onUploadProgress, signal);
}

export function mergePdfs(files: File[], onUploadProgress: (f: number) => void, signal?: AbortSignal) {
  const form = new FormData();
  for (const f of files) form.append("file", f);
  return upload("/api/pdf/merge", form, onUploadProgress, signal);
}

export function splitPdf(file: File, pages: string, onUploadProgress: (f: number) => void, signal?: AbortSignal) {
  const form = new FormData();
  form.append("pages", pages);
  form.append("file", file);
  return upload("/api/pdf/split", form, onUploadProgress, signal);
}

/**
 * POST a form and get a file back.
 * Uses XMLHttpRequest (not fetch) because only XHR reports upload progress.
 */
function upload(
  url: string,
  form: FormData,
  onUploadProgress: (fraction: number) => void,
  signal?: AbortSignal,
): Promise<Result> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", url);
    xhr.responseType = "blob";
    xhr.upload.onprogress = (e) => e.lengthComputable && onUploadProgress(e.loaded / e.total);
    xhr.upload.onload = () => onUploadProgress(1);
    xhr.onerror = () => reject(new Error("Network error"));
    xhr.onabort = () => reject(new DOMException("Cancelled", "AbortError"));
    xhr.onload = async () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        const disposition = xhr.getResponseHeader("Content-Disposition") ?? "";
        const match = /filename="([^"]+)"/.exec(disposition);
        resolve({ blob: xhr.response as Blob, fileName: match?.[1] ?? "download" });
      } else {
        const res = new Response(xhr.response as Blob, { status: xhr.status, statusText: xhr.statusText });
        reject(new Error(await errorMessage(res)));
      }
    };
    signal?.addEventListener("abort", () => xhr.abort());
    xhr.send(form);
  });
}
