import { useEffect, useMemo, useRef, useState } from "react";
import { convert, detect, type Category, type Format, type Options, type Target } from "./api";
import { DropZone, ErrorCard, Progress, ResultCard, type Done } from "./components";
import { OptionsPanel, type OptionGroups } from "./OptionsPanel";
import { CATEGORY_LABELS, formatBytes, newId, runLimited, saveBlob, zipBlobs } from "./util";

/** How many files convert at the same time in a batch. */
const PARALLEL = 2;

interface Item {
  id: string;
  file: File;
  format?: Format;
  targets?: Target[];
  target: string | null;
  status: "detecting" | "ready" | "working" | "done" | "error";
  upload: number;
  result?: Done & { blob: Blob };
  error?: string;
}

export function ConvertView() {
  const [items, setItems] = useState<Item[]>([]);
  const [options, setOptions] = useState<Options>({});
  const [zipping, setZipping] = useState(false);
  const abort = useRef<AbortController | null>(null);

  const update = (id: string, patch: Partial<Item>) =>
    setItems((list) => list.map((it) => (it.id === id ? { ...it, ...patch } : it)));

  // Free download URLs when the view goes away.
  const itemsRef = useRef(items);
  itemsRef.current = items;
  useEffect(() => () => itemsRef.current.forEach((it) => it.result && URL.revokeObjectURL(it.result.url)), []);

  function add(files: File[]) {
    const fresh: Item[] = files.map((file) => ({ id: newId(), file, target: null, status: "detecting", upload: 0 }));
    setItems((list) => [...list, ...fresh]);
    for (const item of fresh) {
      detect(item.file)
        .then(({ format, targets }) =>
          update(item.id, {
            format,
            targets,
            status: targets.length ? "ready" : "error",
            error: targets.length ? undefined : `No conversions available for ${format.name} yet.`,
          }),
        )
        .catch((e: Error) => update(item.id, { status: "error", error: e.message }));
    }
  }

  function remove(id: string) {
    const item = items.find((it) => it.id === id);
    if (item?.result) URL.revokeObjectURL(item.result.url);
    setItems((list) => list.filter((it) => it.id !== id));
  }

  function clear() {
    abort.current?.abort();
    items.forEach((it) => it.result && URL.revokeObjectURL(it.result.url));
    setItems([]);
  }

  function setTarget(id: string, target: string) {
    const item = items.find((it) => it.id === id);
    if (item?.result) URL.revokeObjectURL(item.result.url);
    update(id, { target, status: "ready", result: undefined, error: undefined });
  }

  async function run(list: Item[]) {
    const controller = new AbortController();
    abort.current = controller;
    await runLimited(list, PARALLEL, async (item) => {
      if (controller.signal.aborted || !item.target) return;
      update(item.id, { status: "working", upload: 0, error: undefined });
      try {
        const { blob, fileName } = await convert(
          item.file,
          item.target,
          options,
          (upload) => update(item.id, { upload }),
          controller.signal,
        );
        const result = { blob, fileName, size: blob.size, url: URL.createObjectURL(blob) };
        update(item.id, { status: "done", result });
      } catch (e) {
        const cancelled = (e as Error).name === "AbortError";
        update(item.id, cancelled ? { status: "ready" } : { status: "error", error: (e as Error).message });
      }
    });
  }

  async function downloadAll() {
    setZipping(true);
    try {
      const done = items.filter((it) => it.result).map((it) => ({ name: it.result!.fileName, blob: it.result!.blob }));
      saveBlob(await zipBlobs(done), "akarbox-converted.zip");
    } finally {
      setZipping(false);
    }
  }

  // Formats that *every* detected file can be converted to.
  const common = useMemo(() => {
    const detected = items.filter((it) => it.targets?.length);
    if (detected.length < 2) return [];
    const [first, ...rest] = detected;
    return first.targets!.filter((t) => rest.every((it) => it.targets!.some((o) => o.format.ext === t.format.ext)));
  }, [items]);

  const groups: OptionGroups = useMemo(() => {
    const chosen = items.map((it) => ({ it, t: it.targets?.find((t) => t.format.ext === it.target) })).filter((x) => x.t);
    const cat = (c: Category) => chosen.some((x) => x.t!.format.category === c);
    return {
      image: cat("image"),
      audio: cat("audio"),
      video: cat("video") || chosen.some((x) => x.t!.format.ext === "gif" && x.it.format?.category === "video"),
      pdfRender: chosen.some((x) => x.it.format?.ext === "pdf" && x.t!.format.category === "image"),
    };
  }, [items]);

  const working = items.some((it) => it.status === "working");
  const runnable = items.filter((it) => it.target && (it.status === "ready" || it.status === "error") && it.targets);
  const doneCount = items.filter((it) => it.status === "done").length;
  const single = items.length === 1 ? items[0] : null;

  return (
    <>
      <DropZone onFiles={add} multiple compact={items.length > 0}>
        {items.length ? (
          <span className="muted small">Drop or click to add more files</span>
        ) : (
          <>
            <strong>Drop files here</strong>
            <span className="muted">or click to browse · several at once is fine</span>
          </>
        )}
      </DropZone>

      {single && <SingleFile item={single} onTarget={(t) => setTarget(single.id, t)} onRemove={clear} />}

      {items.length > 1 && (
        <section className="card">
          <div className="row between">
            <h2>{items.length} files</h2>
            {common.length > 0 && (
              <label className="row tight">
                <span className="muted small">Convert all to</span>
                <TargetSelect
                  targets={common}
                  value={null}
                  disabled={working}
                  onChange={(ext) => items.forEach((it) => it.targets && setTarget(it.id, ext))}
                />
              </label>
            )}
          </div>
          <ul className="file-list">
            {items.map((it) => (
              <li key={it.id} className="file-row">
                <div className="file-info">
                  <strong className="filename">{it.file.name}</strong>
                  <span className="muted small">
                    {formatBytes(it.file.size)}
                    {it.format && ` · ${it.format.name}`}
                  </span>
                </div>
                <div className="file-actions">
                  {it.targets && it.targets.length > 0 && (
                    <TargetSelect
                      targets={it.targets}
                      value={it.target}
                      disabled={it.status === "working"}
                      onChange={(ext) => setTarget(it.id, ext)}
                    />
                  )}
                  <span className="file-status">
                    {it.status === "detecting" && <span className="muted small">Detecting…</span>}
                    {it.status === "working" && <Progress upload={it.upload} />}
                    {it.status === "done" && it.result && (
                      <a className="button small" href={it.result.url} download={it.result.fileName}>
                        Download
                      </a>
                    )}
                    {it.status === "error" && <span className="error-text small">{it.error}</span>}
                  </span>
                  <button
                    className="icon-button"
                    aria-label={`Remove ${it.file.name}`}
                    disabled={it.status === "working"}
                    onClick={() => remove(it.id)}
                  >
                    ✕
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </section>
      )}

      {items.length > 0 && <OptionsPanel groups={groups} options={options} onChange={setOptions} />}

      {items.length > 0 && (
        <div className="row actions">
          {working ? (
            <button className="button secondary" onClick={() => abort.current?.abort()}>
              Cancel
            </button>
          ) : (
            <button className="button" disabled={runnable.length === 0} onClick={() => run(runnable)}>
              {single
                ? single.target
                  ? `Convert to ${single.target.toUpperCase()}`
                  : "Pick a format"
                : `Convert ${runnable.length || ""} file${runnable.length === 1 ? "" : "s"}`}
            </button>
          )}
          {items.length > 1 && doneCount > 1 && (
            <button className="button secondary" disabled={zipping} onClick={downloadAll}>
              {zipping ? "Zipping…" : `Download all (${doneCount}) as .zip`}
            </button>
          )}
          {items.length > 1 && !working && (
            <button className="link" onClick={clear}>
              Clear all
            </button>
          )}
        </div>
      )}

      {single?.status === "working" && <Progress upload={single.upload} />}
      {single?.status === "done" && single.result && <ResultCard result={single.result} />}
      {single?.status === "error" && single.error && <ErrorCard message={single.error} />}
    </>
  );
}

/** The one-file view: formats shown as chips, grouped by category. */
function SingleFile({ item, onTarget, onRemove }: { item: Item; onTarget: (ext: string) => void; onRemove: () => void }) {
  const [filter, setFilter] = useState("");
  const targets = item.targets ?? [];

  const groups = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const map = new Map<Category, Target[]>();
    for (const t of targets) {
      if (q && !t.format.ext.includes(q) && !t.format.name.toLowerCase().includes(q)) continue;
      map.set(t.format.category, [...(map.get(t.format.category) ?? []), t]);
    }
    return [...map.entries()];
  }, [targets, filter]);
  const chosen = targets.find((t) => t.format.ext === item.target);

  return (
    <section className="card">
      <div className="row between">
        <div className="file-info">
          <strong className="filename">{item.file.name}</strong>
          <span className="muted small">
            {formatBytes(item.file.size)}
            {item.format && ` · ${item.format.name}`}
          </span>
        </div>
        <button className="icon-button" aria-label="Remove file" disabled={item.status === "working"} onClick={onRemove}>
          ✕
        </button>
      </div>

      {item.status === "detecting" && <p className="muted">Detecting format…</p>}

      {targets.length > 0 && (
        <>
          <div className="row between">
            <h2>Convert to…</h2>
            {targets.length > 8 && (
              <input className="search" placeholder="Search formats" value={filter} onChange={(e) => setFilter(e.target.value)} />
            )}
          </div>
          {groups.map(([category, list]) => (
            <div key={category} className="group">
              <h3>{CATEGORY_LABELS[category]}</h3>
              <div className="chips">
                {list.map((t) => (
                  <button
                    key={t.format.ext}
                    className={"chip" + (item.target === t.format.ext ? " selected" : "")}
                    title={t.format.name}
                    disabled={item.status === "working"}
                    onClick={() => onTarget(t.format.ext)}
                  >
                    {t.format.ext}
                  </button>
                ))}
              </div>
            </div>
          ))}
          {chosen && (
            <p className="muted small route">
              Route: {[chosen.plan.steps[0].from, ...chosen.plan.steps.map((s) => s.to)].join(" → ")} via{" "}
              {[...new Set(chosen.plan.steps.map((s) => s.converter))].join(" + ")}
            </p>
          )}
        </>
      )}
    </section>
  );
}

function TargetSelect({
  targets,
  value,
  disabled,
  onChange,
}: {
  targets: Target[];
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
              {t.format.ext.toUpperCase()}
            </option>
          ))}
        </optgroup>
      ))}
    </select>
  );
}
