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

export const getTargets = (ext: string) =>
  fetch(`/api/targets/${encodeURIComponent(ext)}`).then((r) => json<Target[]>(r));

export interface ConvertResult {
  blob: Blob;
  fileName: string;
}

/**
 * Upload a file and get the converted file back.
 * Uses XMLHttpRequest (not fetch) because only XHR reports upload progress.
 */
export function convert(
  file: File,
  target: string,
  onUploadProgress: (fraction: number) => void,
  signal?: AbortSignal,
): Promise<ConvertResult> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", "/api/convert");
    xhr.responseType = "blob";
    xhr.upload.onprogress = (e) => e.lengthComputable && onUploadProgress(e.loaded / e.total);
    xhr.onerror = () => reject(new Error("Network error"));
    xhr.onabort = () => reject(new DOMException("Cancelled", "AbortError"));
    xhr.onload = async () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        const disposition = xhr.getResponseHeader("Content-Disposition") ?? "";
        const match = /filename="([^"]+)"/.exec(disposition);
        const fallback = file.name.replace(/\.[^.]+$/, "") + "." + target;
        resolve({ blob: xhr.response as Blob, fileName: match?.[1] ?? fallback });
      } else {
        const res = new Response(xhr.response as Blob, { status: xhr.status, statusText: xhr.statusText });
        reject(new Error(await errorMessage(res)));
      }
    };
    signal?.addEventListener("abort", () => xhr.abort());

    const form = new FormData();
    form.append("target", target); // send target first so the server knows it early
    form.append("file", file);
    xhr.send(form);
  });
}

/** Same rules as the Rust `normalize_ext` + `detect_format`. */
export function extensionOf(name: string): string | null {
  const m = /\.([^./\\]+)$/.exec(name);
  if (!m) return null;
  const ext = m[1].toLowerCase();
  const aliases: Record<string, string> = { jpeg: "jpg", jpe: "jpg", tif: "tiff", htm: "html", markdown: "md", text: "txt" };
  return aliases[ext] ?? ext;
}
