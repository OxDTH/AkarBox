import { useEffect, useMemo, useRef, useState } from "react";
import { convert, detect, type Category, type Format, type Options, type Target } from "./api";
import { Progress, type Done } from "./components";
import { groupsFor, mergeGroups, OptionsPanel } from "./OptionsPanel";
import { CATEGORY_LABELS, formatBytes, isPdf, runLimited, saveBlob, sizeChange, zipBlobs } from "./util";

/** How many files convert at the same time. */
const PARALLEL = 2;

export interface Entry {
  id: string;
  file: File;
}

interface Item {
  format?: Format;
  targets?: Target[];
  target: string | null;
  status: "detecting" | "ready" | "working" | "done" | "error";
  upload: number;
  result?: Done & { blob: Blob };
  error?: string;
}

/** Several files at once: per-file targets, shared options, "Download all". */
export function BatchView({
  entries,
  onRemove,
  onClear,
  onMergePdfs,
  initialTarget,
  initialOptions,
}: {
  entries: Entry[];
  onRemove: (id: string) => void;
  onClear: () => void;
  onMergePdfs?: () => void;
  initialTarget?: string | "same";
  initialOptions?: Options;
}) {
  const [items, setItems] = useState<Record<string, Item>>({});
  const [options, setOptions] = useState<Options>(initialOptions ?? {});
  const [zipping, setZipping] = useState(false);
  const abort = useRef<AbortController | null>(null);

  const update = (id: string, patch: Partial<Item>) =>
    setItems((all) => ({ ...all, [id]: { ...(all[id] ?? { target: null, status: "detecting", upload: 0 }), ...patch } }));

  // Detect every new file once.
  const seen = useRef(new Set<string>());
  useEffect(() => {
    for (const { id, file } of entries) {
      if (seen.current.has(id)) continue;
      seen.current.add(id);
      update(id, { status: "detecting" });
      detect(file)
        .then(({ format, targets }) => {
          const wanted = initialTarget === "same" ? format.ext : initialTarget;
          update(id, {
            format,
            targets,
            target: targets.some((t) => t.format.ext === wanted) ? wanted! : null,
            status: targets.length ? "ready" : "error",
            error: targets.length ? undefined : `Can't convert ${format.name} yet.`,
          });
        })
        .catch((e: Error) => update(id, { status: "error", error: e.message }));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries]);

  // Free download URLs when results are dropped.
  const itemsRef = useRef(items);
  itemsRef.current = items;
  useEffect(() => () => Object.values(itemsRef.current).forEach((it) => it.result && URL.revokeObjectURL(it.result.url)), []);

  const rows = entries.map((e) => ({ ...e, item: items[e.id] ?? { target: null, status: "detecting" as const, upload: 0 } }));

  function setTarget(id: string, target: string) {
    const old = items[id]?.result;
    if (old) URL.revokeObjectURL(old.url);
    update(id, { target, status: "ready", result: undefined, error: undefined });
  }

  async function run() {
    const controller = new AbortController();
    abort.current = controller;
    const todo = rows.filter((r) => r.item.target && r.item.targets?.length && r.item.status !== "working" && r.item.status !== "done");
    await runLimited(todo, PARALLEL, async ({ id, file, item }) => {
      if (controller.signal.aborted || !item.target) return;
      update(id, { status: "working", upload: 0, error: undefined });
      try {
        const { blob, fileName } = await convert(file, item.target, options, (upload) => update(id, { upload }), controller.signal);
        update(id, { status: "done", result: { blob, fileName, size: blob.size, url: URL.createObjectURL(blob) } });
      } catch (e) {
        const cancelled = (e as Error).name === "AbortError";
        update(id, cancelled ? { status: "ready" } : { status: "error", error: (e as Error).message });
      }
    });
  }

  async function downloadAll() {
    setZipping(true);
    try {
      const done = rows.filter((r) => r.item.result).map((r) => ({ name: r.item.result!.fileName, blob: r.item.result!.blob }));
      saveBlob(await zipBlobs(done), "akarbox-converted.zip");
    } finally {
      setZipping(false);
    }
  }

  // Formats that *every* detected file can be converted to (plus "compress all").
  const common = useMemo(() => {
    const detected = rows.filter((r) => r.item.targets?.length);
    if (detected.length < 2) return [];
    const [first, ...rest] = detected;
    return first.item.targets!.filter((t) => rest.every((r) => r.item.targets!.some((o) => o.format.ext === t.format.ext)));
  }, [rows]);
  const canCompressAll = rows.length > 1 && rows.every((r) => r.item.format && r.item.targets?.some((t) => t.format.ext === r.item.format!.ext));

  const groups = mergeGroups(rows.map((r) => groupsFor(r.item.format, r.item.targets?.find((t) => t.format.ext === r.item.target))));
  const working = rows.some((r) => r.item.status === "working");
  const pending = rows.filter((r) => r.item.target && r.item.targets?.length && (r.item.status === "ready" || r.item.status === "error")).length;
  const doneCount = rows.filter((r) => r.item.status === "done").length;
  const allPdfs = entries.every((e) => isPdf(e.file));

  return (
    <div className="workspace">
      <section className="card">
        <div className="row between">
          <h2>{rows.length} files</h2>
          <div className="row tight">
            {common.length > 0 && (
              <label className="row tight">
                <span className="muted small">Convert all to</span>
                <TargetSelect targets={common} value={null} disabled={working} onChange={(ext) => rows.forEach((r) => r.item.targets && setTarget(r.id, ext))} />
              </label>
            )}
            {canCompressAll && (
              <button className="button secondary small" disabled={working} onClick={() => rows.forEach((r) => setTarget(r.id, r.item.format!.ext))}>
                Compress all
              </button>
            )}
          </div>
        </div>
        {allPdfs && onMergePdfs && (
          <p className="muted small">
            All PDFs? You can also{" "}
            <button className="link" onClick={onMergePdfs}>
              merge them into one
            </button>
            .
          </p>
        )}
        <ul className="file-list">
          {rows.map(({ id, file, item }) => (
            <li key={id} className="file-row">
              <div className="file-info">
                <strong className="filename">{file.name}</strong>
                <span className="muted small">
                  {item.format && <span className={`badge cat-${item.format.category}`}>{CATEGORY_LABELS[item.format.category]}</span>}{" "}
                  {item.result ? sizeChange(file.size, item.result.size) : formatBytes(file.size)}
                </span>
              </div>
              <div className="file-actions">
                {item.targets && item.targets.length > 0 && (
                  <TargetSelect targets={item.targets} source={item.format} value={item.target} disabled={item.status === "working"} onChange={(ext) => setTarget(id, ext)} />
                )}
                <span className="file-status">
                  {item.status === "detecting" && <span className="muted small">Detecting…</span>}
                  {item.status === "working" && <Progress upload={item.upload} />}
                  {item.status === "done" && item.result && (
                    <a className="button small" href={item.result.url} download={item.result.fileName}>
                      Download
                    </a>
                  )}
                  {item.status === "error" && <span className="error-text small">{item.error}</span>}
                </span>
                <button className="icon-button" aria-label={`Remove ${file.name}`} disabled={item.status === "working"} onClick={() => onRemove(id)}>
                  ✕
                </button>
              </div>
            </li>
          ))}
        </ul>
      </section>

      <OptionsPanel groups={groups} options={options} onChange={setOptions} disabled={working} />

      <div className="row actions">
        {working ? (
          <button className="button secondary" onClick={() => abort.current?.abort()}>
            Cancel
          </button>
        ) : (
          <button className="button" disabled={pending === 0} onClick={run}>
            {pending ? `Convert ${pending} file${pending === 1 ? "" : "s"}` : "Pick formats above"}
          </button>
        )}
        {doneCount > 1 && (
          <button className="button secondary" disabled={zipping} onClick={downloadAll}>
            {zipping ? "Zipping…" : `Download all (${doneCount}) as .zip`}
          </button>
        )}
        {!working && (
          <button className="link" onClick={onClear}>
            Clear all
          </button>
        )}
      </div>
    </div>
  );
}

function TargetSelect({
  targets,
  source,
  value,
  disabled,
  onChange,
}: {
  targets: Target[];
  source?: Format;
  value: string | null;
  disabled?: boolean;
  onChange: (ext: string) => void;
}) {
  const byCategory = new Map<Category, Target[]>();
  for (const t of targets) byCategory.set(t.format.category, [...(byCategory.get(t.format.category) ?? []), t]);
  return (
    <select className="select" value={value ?? ""} disabled={disabled} onChange={(e) => e.target.value && onChange(e.target.value)}>
      <option value="">Format…</option>
      {[...byCategory.entries()].map(([cat, list]) => (
        <optgroup key={cat} label={CATEGORY_LABELS[cat]}>
          {list.map((t) => (
            <option key={t.format.ext} value={t.format.ext}>
              {t.format.ext === source?.ext ? `${t.format.ext.toUpperCase()} (compress)` : t.format.ext.toUpperCase()}
            </option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}
