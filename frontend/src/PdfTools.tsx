import { useEffect, useRef, useState } from "react";
import { mergePdfs, splitPdf, type Result } from "./api";
import { DropZone, ErrorCard, Progress, ResultCard, type Done } from "./components";
import { formatBytes, newId } from "./util";

const isPdf = (f: File) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf");

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

function JobStatus({ job }: { job: ReturnType<typeof useJob> }) {
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
  if (state.kind === "done") return <ResultCard result={state.result} />;
  if (state.kind === "error") return <ErrorCard message={state.message} />;
  return null;
}

export function MergeView() {
  const [files, setFiles] = useState<{ id: string; file: File }[]>([]);
  const [rejected, setRejected] = useState<string[]>([]);
  const [dragging, setDragging] = useState<number | null>(null);
  const job = useJob();
  const working = job.state.kind === "working";

  function add(list: File[]) {
    setRejected(list.filter((f) => !isPdf(f)).map((f) => f.name));
    setFiles((cur) => [...cur, ...list.filter(isPdf).map((file) => ({ id: newId(), file }))]);
    job.reset();
  }

  function move(from: number, to: number) {
    if (to < 0 || to >= files.length || from === to) return;
    setFiles((cur) => {
      const next = [...cur];
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return next;
    });
    job.reset();
  }

  return (
    <>
      <DropZone onFiles={add} multiple accept="application/pdf,.pdf" compact={files.length > 0} disabled={working}>
        {files.length ? (
          <span className="muted small">Drop or click to add more PDFs</span>
        ) : (
          <>
            <strong>Drop PDFs to merge</strong>
            <span className="muted">or click to browse · you can reorder them next</span>
          </>
        )}
      </DropZone>
      {rejected.length > 0 && <ErrorCard message={`Not a PDF, skipped: ${rejected.join(", ")}`} />}

      {files.length > 0 && (
        <section className="card">
          <div className="row between">
            <h2>Merge order</h2>
            <span className="muted small">Drag rows or use the arrows</span>
          </div>
          <ol className="file-list">
            {files.map(({ id, file }, i) => (
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
                  <button
                    className="icon-button"
                    aria-label="Move down"
                    disabled={working || i === files.length - 1}
                    onClick={() => move(i, i + 1)}
                  >
                    ↓
                  </button>
                  <button
                    className="icon-button"
                    aria-label={`Remove ${file.name}`}
                    disabled={working}
                    onClick={() => {
                      setFiles((cur) => cur.filter((f) => f.id !== id));
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
            <button
              className="button"
              disabled={files.length < 2 || working}
              onClick={() => job.run((p, s) => mergePdfs(files.map((f) => f.file), p, s))}
            >
              {files.length < 2 ? "Add at least 2 PDFs" : `Merge ${files.length} PDFs`}
            </button>
            {!working && (
              <button className="link" onClick={() => (setFiles([]), job.reset())}>
                Clear
              </button>
            )}
          </div>
        </section>
      )}
      <JobStatus job={job} />
    </>
  );
}

export function SplitView() {
  const [file, setFile] = useState<File | null>(null);
  const [pages, setPages] = useState("");
  const [rejected, setRejected] = useState<string | null>(null);
  const job = useJob();
  const working = job.state.kind === "working";

  return (
    <>
      <DropZone
        accept="application/pdf,.pdf"
        compact={!!file}
        disabled={working}
        onFiles={([f]) => {
          job.reset();
          if (isPdf(f)) {
            setFile(f);
            setRejected(null);
          } else setRejected(`${f.name} is not a PDF.`);
        }}
      >
        {file ? (
          <>
            <strong className="filename">{file.name}</strong>
            <span className="muted small">{formatBytes(file.size)} · click or drop to choose another PDF</span>
          </>
        ) : (
          <>
            <strong>Drop a PDF</strong>
            <span className="muted">to keep only the pages you need</span>
          </>
        )}
      </DropZone>
      {rejected && <ErrorCard message={rejected} />}

      {file && (
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
                <strong>Pages to keep</strong>
              </span>
              <input
                className="text-input"
                placeholder="e.g. 1-3, 5, 8-end"
                value={pages}
                onChange={(e) => {
                  setPages(e.target.value);
                  if (job.state.kind !== "working") job.reset();
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
      )}
      <JobStatus job={job} />
    </>
  );
}
