import { useEffect, useRef, useState } from "react";
import { convert, detect, type Format, type Options, type Target } from "./api";
import type { Done } from "./components";

export type Status = "detecting" | "ready" | "working" | "done" | "error";

export interface ConversionState {
  format?: Format;
  targets: Target[];
  target: string | null;
  status: Status;
  upload: number;
  result?: Done & { blob: Blob };
  error?: string;
}

/**
 * Everything needed to convert one file: detect its format, pick a target,
 * run the conversion and keep the result.
 */
export function useConversion(file: File, initialTarget?: string | "same") {
  const [state, setState] = useState<ConversionState>({ targets: [], target: null, status: "detecting", upload: 0 });
  const abort = useRef<AbortController | null>(null);
  const patch = (p: Partial<ConversionState>) => setState((s) => ({ ...s, ...p }));

  useEffect(() => {
    let cancelled = false;
    setState({ targets: [], target: null, status: "detecting", upload: 0 });
    detect(file)
      .then(({ format, targets }) => {
        if (cancelled) return;
        const wanted = initialTarget === "same" ? format.ext : initialTarget;
        const target = targets.some((t) => t.format.ext === wanted) ? wanted! : null;
        patch({
          format,
          targets,
          target,
          status: targets.length ? "ready" : "error",
          error: targets.length ? undefined : `AkarBox can't convert ${format.name} files yet.`,
        });
      })
      .catch((e: Error) => !cancelled && patch({ status: "error", error: e.message }));
    return () => {
      cancelled = true;
      abort.current?.abort();
    };
    // `initialTarget` only matters for the first detection of this file.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file]);

  // Free the previous result's memory when it's replaced or the view goes away.
  const resultUrl = state.result?.url;
  useEffect(() => () => {
    if (resultUrl) URL.revokeObjectURL(resultUrl);
  }, [resultUrl]);

  function setTarget(target: string) {
    patch({ target, status: "ready", result: undefined, error: undefined });
  }

  async function run(options: Options) {
    if (!state.target) return;
    const controller = new AbortController();
    abort.current = controller;
    patch({ status: "working", upload: 0, error: undefined, result: undefined });
    try {
      const { blob, fileName } = await convert(file, state.target, options, (upload) => patch({ upload }), controller.signal);
      patch({ status: "done", result: { blob, fileName, size: blob.size, url: URL.createObjectURL(blob) } });
    } catch (e) {
      if ((e as Error).name === "AbortError") patch({ status: "ready" });
      else patch({ status: "error", error: (e as Error).message });
    }
  }

  /** Clear a finished result or error so options can be changed and re-run. */
  const reset = () => setState((s) => (s.status === "done" || s.status === "error" ? { ...s, status: s.targets.length ? "ready" : "error", result: undefined, error: s.targets.length ? undefined : s.error } : s));

  return { ...state, setTarget, run, reset, cancel: () => abort.current?.abort() };
}
