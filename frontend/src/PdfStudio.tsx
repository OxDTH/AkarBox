import { useEffect, useRef, useState } from "react";
import { mergePdfs, splitPdf, type Result } from "./api";
import type { Entry } from "./BatchView";
import { DropZone, ErrorCard, Progress, ResultCard, type Done } from "./components";
import type { PdfTab } from "./presets";
import { SingleWorkspace } from "./SingleWorkspace";
import { formatBytes, isPdf, newId } from "./util";

const TABS: { id: PdfTab; label: string; needsQpdf?: boolean }[] = [
  { id: "images", label: "To images" },
  { id: "compress", label: "Compress" },
  { id: "merge", label: "Merge", needsQpdf: true },
  { id: "split", label: "Split", needsQpdf: true },
];

/** One place for everything PDF: convert pages, compress, merge, split. */
export function PdfStudio({ initial, initialTab, pdfTools }: { initial: Entry[]; initialTab: PdfTab; pdfTools: boolean }) {
  const [pdfs, setPdfs] = useState<Entry[]>(initial);
  const [selected, setSelected] = useState<string | null>(initial[0]?.id ?? null);
  const [tab, setTab] = useState<PdfTab>(initialTab);
  const [rejected, setRejected] = useState<string[]>([]);
  const tabs = TABS.filter((t) => !t.needsQpdf || pdfTools);
  const current = pdfs.find((p) => p.id === selected) ?? pdfs[0];

  function add(files: File[]) {
    setRejected(files.filter((f) => !isPdf(f)).map((f) => f.name));
    const fresh = files.filter(isPdf).map((file) => ({ id: newId(), file }));
    setPdfs((cur) => [...cur, ...fresh]);
    if (!current && fresh[0]) setSelected(fresh[0].id);
  }

  return (
    <div className="workspace">
      <nav className="tabs" role="tablist" aria-label="PDF tools">
        {tabs.map((t) => (
          <button key={t.id} role="tab" aria-selected={tab === t.id} className={"tab" + (tab === t.id ? " active" : "")} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
      </nav>

      <DropZone onFiles={add} multiple accept="application/pdf,.pdf" compact={pdfs.length > 0}>
        {pdfs.length ? (
          <span className="muted small">Drop or click to add more PDFs</span>
        ) : (
          <>
            <strong>Drop PDF files here</strong>
            <span className="muted">or click to browse</span>
          </>
        )}
      </DropZone>
      {rejected.length > 0 && <ErrorCard message={`Not a PDF, skipped: ${rejected.join(", ")}`} />}

      {tab !== "merge" && pdfs.length > 1 && (
        <label className="row tight">
          <span className="muted small">Working on</span>
          <select className="select" value={current?.id} onChange={(e) => setSelected(e.target.value)}>
            {pdfs.map((p) => (
              <option key={p.id} value={p.id}>
                {p.file.name}
              </option>
            ))}
          </select>
        </label>
      )}

      {tab === "images" && current && (
        <SingleWorkspace key={`img-${current.id}`} file={current.file} initialTarget="png" heading="Turn pages into…" filter={(t) => t.format.ext !== "pdf"} />
      )}
      {tab === "compress" && current && <SingleWorkspace key={`cmp-${current.id}`} file={current.file} fixedTarget="pdf" />}
      {tab === "merge" && <MergeList pdfs={pdfs} setPdfs={setPdfs} />}
      {tab === "split" && current && <SplitForm key={current.id} file={current.file} />}
    </div>
  );
}

/** Shared state machine for "upload, wait, show result or error". */
function useJob() {
  const [state, setState] = useState<
    { kind: "idle" } | { kind: "working"; upload: number } | { kind: "done"; result: Done } | { kind: "error"; message: string }
  >({ kind: "idle" });
  const abort = useRef<AbortController | null>(null);

  useEffect(() => () => {
    if (state.kind === "done") URL.revokeObjectURL(state.result.url);
  }, [state]);

  async function run(job: (progress: (f: number) => void, signal: AbortSignal) => Promise<Result>) {
    const controller = new AbortController();
    abort.current = controller;
    setState({ kind: "working", upload: 0 });
    try {
      const { blob, fileName } = await job((upload) => setState({ kind: "working", upload }), controller.signal);
      setState({ kind: "done", result: { url: URL.createObjectURL(blob), fileName, size: blob.size } });
    } catch (e) {
      if ((e as Error).name === "AbortError") setState({ kind: "idle" });
      else setState({ kind: "error", message: (e as Error).message });
    }
  }
  return { state, run, reset: () => setState({ kind: "idle" }), cancel: () => abort.current?.abort() };
}

function JobStatus({ job, originalSize }: { job: ReturnType<typeof useJob>; originalSize?: number }) {
  const { state } = job;
  if (state.kind === "working")
    return (
      <div className="row">
        <Progress upload={state.upload} />
        <button className="button secondary" onClick={job.cancel}>
          Cancel
        </button>
      </div>
    );
  if (state.kind === "done") return <ResultCard result={state.result} originalSize={originalSize} />;
  if (state.kind === "error") return <ErrorCard message={state.message} />;
  return null;
}

function MergeList({ pdfs, setPdfs }: { pdfs: Entry[]; setPdfs: React.Dispatch<React.SetStateAction<Entry[]>> }) {
  const [dragging, setDragging] = useState<number | null>(null);
  const job = useJob();
  const working = job.state.kind === "working";

  function move(from: number, to: number) {
    if (to < 0 || to >= pdfs.length || from === to) return;
    setPdfs((cur) => {
      const next = [...cur];
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return next;
    });
    job.reset();
  }

  if (pdfs.length === 0) return <p className="muted">Add two or more PDFs to merge them into one.</p>;

  return (
    <>
      <section className="card">
        <div className="row between">
          <h2>Merge order</h2>
          <span className="muted small">Drag rows or use the arrows</span>
        </div>
        <ol className="file-list">
          {pdfs.map(({ id, file }, i) => (
            <li
              key={id}
              className={"file-row draggable" + (dragging === i ? " dragging" : "")}
              draggable={!working}
              onDragStart={() => setDragging(i)}
              onDragOver={(e) => {
                e.preventDefault();
                if (dragging !== null && dragging !== i) {
                  move(dragging, i);
                  setDragging(i);
                }
              }}
              onDragEnd={() => setDragging(null)}
            >
              <span className="order">{i + 1}</span>
              <div className="file-info">
                <strong className="filename">{file.name}</strong>
                <span className="muted small">{formatBytes(file.size)}</span>
              </div>
              <div className="file-actions">
                <button className="icon-button" aria-label="Move up" disabled={working || i === 0} onClick={() => move(i, i - 1)}>
                  ↑
                </button>
                <button className="icon-button" aria-label="Move down" disabled={working || i === pdfs.length - 1} onClick={() => move(i, i + 1)}>
                  ↓
                </button>
                <button
                  className="icon-button"
                  aria-label={`Remove ${file.name}`}
                  disabled={working}
                  onClick={() => {
                    setPdfs((cur) => cur.filter((f) => f.id !== id));
                    job.reset();
                  }}
                >
                  ✕
                </button>
              </div>
            </li>
          ))}
        </ol>
        <div className="row actions">
          <button className="button" disabled={pdfs.length < 2 || working} onClick={() => job.run((p, s) => mergePdfs(pdfs.map((f) => f.file), p, s))}>
            {pdfs.length < 2 ? "Add at least 2 PDFs" : `Merge ${pdfs.length} PDFs`}
          </button>
        </div>
      </section>
      <JobStatus job={job} />
    </>
  );
}

function SplitForm({ file }: { file: File }) {
  const [pages, setPages] = useState("");
  const job = useJob();
  const working = job.state.kind === "working";

  return (
    <>
      <section className="card">
        <form
          className="stack"
          onSubmit={(e) => {
            e.preventDefault();
            if (pages.trim()) job.run((p, s) => splitPdf(file, pages, p, s));
          }}
        >
          <label className="stack tight">
            <span>
              <strong>Pages to keep</strong> <span className="muted small">from {file.name}</span>
            </span>
            <input
              className="text-input"
              placeholder="e.g. 1-3, 5, 8-end"
              value={pages}
              onChange={(e) => {
                setPages(e.target.value);
                if (!working) job.reset();
              }}
            />
            <span className="muted small">Use commas between pages and dashes for ranges. “end” means the last page.</span>
          </label>
          <div className="row actions">
            <button className="button" type="submit" disabled={!pages.trim() || working}>
              Extract pages
            </button>
          </div>
        </form>
      </section>
      <JobStatus job={job} originalSize={file.size} />
    </>
  );
}
