import { useEffect, useMemo, useRef, useState } from "react";
import { convert, extensionOf, getInfo, getTargets, type Category, type Info, type Target } from "./api";

const CATEGORY_LABELS: Record<Category, string> = {
  image: "Images",
  audio: "Audio",
  video: "Video",
  document: "Documents",
  spreadsheet: "Spreadsheets",
  presentation: "Presentations",
  ebook: "Ebooks",
};

type Status =
  | { kind: "idle" }
  | { kind: "working"; upload: number }
  | { kind: "done"; url: string; fileName: string; size: number }
  | { kind: "error"; message: string };

export default function App() {
  const [info, setInfo] = useState<Info | null>(null);
  const [file, setFile] = useState<File | null>(null);
  const [targets, setTargets] = useState<Target[] | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const abort = useRef<AbortController | null>(null);

  useEffect(() => {
    getInfo().then(setInfo).catch((e: Error) => setStatus({ kind: "error", message: e.message }));
  }, []);

  // Free the memory held by a previous download link.
  useEffect(() => () => {
    if (status.kind === "done") URL.revokeObjectURL(status.url);
  }, [status]);

  const ext = file ? extensionOf(file.name) : null;
  const sourceFormat = info?.formats.find((f) => f.ext === ext);

  async function pick(f: File) {
    abort.current?.abort();
    setFile(f);
    setSelected(null);
    setFilter("");
    setTargets(null);
    setStatus({ kind: "idle" });
    const e = extensionOf(f.name);
    if (!e) {
      setStatus({ kind: "error", message: "This file has no extension, so its format can't be detected." });
      return;
    }
    try {
      setTargets(await getTargets(e));
    } catch (err) {
      setStatus({ kind: "error", message: (err as Error).message });
    }
  }

  async function run() {
    if (!file || !selected) return;
    const controller = new AbortController();
    abort.current = controller;
    setStatus({ kind: "working", upload: 0 });
    try {
      const result = await convert(file, selected, (upload) => setStatus({ kind: "working", upload }), controller.signal);
      setStatus({ kind: "done", url: URL.createObjectURL(result.blob), fileName: result.fileName, size: result.blob.size });
    } catch (err) {
      if ((err as Error).name === "AbortError") setStatus({ kind: "idle" });
      else setStatus({ kind: "error", message: (err as Error).message });
    }
  }

  // Group the reachable formats by category, applying the search box.
  const groups = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const map = new Map<Category, Target[]>();
    for (const t of targets ?? []) {
      if (q && !t.format.ext.includes(q) && !t.format.name.toLowerCase().includes(q)) continue;
      map.set(t.format.category, [...(map.get(t.format.category) ?? []), t]);
    }
    return [...map.entries()];
  }, [targets, filter]);

  const chosen = targets?.find((t) => t.format.ext === selected);
  const working = status.kind === "working";

  return (
    <div className="page">
      <header className="header">
        <div className="brand">
          <img src="/favicon.svg" alt="" width={28} height={28} />
          <span>AkarBox</span>
        </div>
        <span className="muted small">
          {info ? `v${info.version} · ${info.converters.join(", ") || "no converters installed"}` : "connecting…"}
        </span>
      </header>

      <main className="main">
        <DropZone file={file} onFile={pick} disabled={working} />

        {file && targets && (
          <section className="card">
            <div className="row between">
              <h2>
                Convert <code>{sourceFormat?.name ?? ext}</code> to…
              </h2>
              {targets.length > 8 && (
                <input
                  className="search"
                  placeholder="Search formats"
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                />
              )}
            </div>

            {targets.length === 0 && <p className="muted">No conversions available for this format yet.</p>}

            {groups.map(([category, list]) => (
              <div key={category} className="group">
                <h3>{CATEGORY_LABELS[category]}</h3>
                <div className="chips">
                  {list.map((t) => (
                    <button
                      key={t.format.ext}
                      className={"chip" + (selected === t.format.ext ? " selected" : "")}
                      title={t.format.name}
                      disabled={working}
                      onClick={() => {
                        setSelected(t.format.ext);
                        if (status.kind !== "working") setStatus({ kind: "idle" });
                      }}
                    >
                      {t.format.ext}
                    </button>
                  ))}
                </div>
              </div>
            ))}

            {chosen && (
              <p className="muted small route">
                Route: {[chosen.plan.steps[0].from, ...chosen.plan.steps.map((s) => s.to)].join(" → ")}
                {" "}via {[...new Set(chosen.plan.steps.map((s) => s.converter))].join(" + ")}
              </p>
            )}

            <div className="row actions">
              {working ? (
                <>
                  <Progress upload={status.upload} />
                  <button className="button secondary" onClick={() => abort.current?.abort()}>
                    Cancel
                  </button>
                </>
              ) : (
                <button className="button" disabled={!selected} onClick={run}>
                  {selected ? `Convert to ${selected.toUpperCase()}` : "Pick a format"}
                </button>
              )}
            </div>
          </section>
        )}

        {status.kind === "done" && (
          <section className="card success">
            <div>
              <strong>{status.fileName}</strong>
              <div className="muted small">{formatBytes(status.size)}</div>
            </div>
            <a className="button" href={status.url} download={status.fileName}>
              Download
            </a>
          </section>
        )}

        {status.kind === "error" && (
          <section className="card error" role="alert">
            {status.message}
          </section>
        )}
      </main>

      <footer className="footer muted small">Files are processed on this server and deleted right after conversion.</footer>
    </div>
  );
}

function DropZone({ file, onFile, disabled }: { file: File | null; onFile: (f: File) => void; disabled: boolean }) {
  const input = useRef<HTMLInputElement>(null);
  const [over, setOver] = useState(false);

  return (
    <div
      className={"dropzone" + (over ? " over" : "") + (file ? " compact" : "")}
      onClick={() => !disabled && input.current?.click()}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(false);
        const f = e.dataTransfer.files[0];
        if (f && !disabled) onFile(f);
      }}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && input.current?.click()}
    >
      <input
        ref={input}
        type="file"
        hidden
        onChange={(e) => {
          const f = e.target.files?.[0];
          if (f) onFile(f);
          e.target.value = "";
        }}
      />
      {file ? (
        <>
          <strong className="filename">{file.name}</strong>
          <span className="muted small">{formatBytes(file.size)} · click or drop to choose another file</span>
        </>
      ) : (
        <>
          <strong>Drop a file here</strong>
          <span className="muted">or click to browse</span>
        </>
      )}
    </div>
  );
}

function Progress({ upload }: { upload: number }) {
  const uploading = upload < 1;
  return (
    <div className="progress">
      <div className="bar">
        <div className={"fill" + (uploading ? "" : " indeterminate")} style={{ width: uploading ? `${upload * 100}%` : undefined }} />
      </div>
      <span className="muted small">{uploading ? `Uploading ${Math.round(upload * 100)}%` : "Converting…"}</span>
    </div>
  );
}

function formatBytes(n: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
