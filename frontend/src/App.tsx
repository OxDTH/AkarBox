import { useCallback, useEffect, useState } from "react";
import { getInfo, type Info, type Options } from "./api";
import { BatchView, type Entry } from "./BatchView";
import { DropZone, ErrorCard } from "./components";
import { Home } from "./Home";
import { PdfStudio } from "./PdfStudio";
import type { PdfTab, Preset } from "./presets";
import { SingleWorkspace } from "./SingleWorkspace";
import { useTheme, type Theme } from "./theme";
import { isPdf, newId } from "./util";

type View =
  | { kind: "home" }
  | { kind: "files"; target?: string | "same"; options?: Options }
  | { kind: "pdf"; tab: PdfTab };

export default function App() {
  const [info, setInfo] = useState<Info | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [view, setView] = useState<View>({ kind: "home" });
  // Bumped to remount the workspace when we start over with new files.
  const [session, setSession] = useState(0);
  const [theme, nextTheme] = useTheme();

  useEffect(() => {
    getInfo().then(setInfo).catch((e: Error) => setError(e.message));
  }, []);

  const goHome = () => {
    setEntries([]);
    setView({ kind: "home" });
    setSession((s) => s + 1);
  };

  const addFiles = useCallback((files: File[], preset?: Preset) => {
    const fresh = files.map((file) => ({ id: newId(), file }));
    setEntries((cur) => [...cur, ...fresh]);
    setView((v) => {
      if (preset) return { kind: "files", target: preset.to, options: preset.options };
      if (v.kind === "home") {
        // A single PDF opens PDF Studio; anything else opens the converter.
        return files.length === 1 && isPdf(files[0]) ? { kind: "pdf", tab: "images" } : { kind: "files" };
      }
      return v;
    });
  }, []);

  // Paste an image (e.g. a screenshot) from the clipboard anywhere on the page.
  useEffect(() => {
    function onPaste(e: ClipboardEvent) {
      const images = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith("image/"));
      if (images.length === 0) return; // plain text etc.: let the browser handle it
      e.preventDefault();
      const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
      const named = images.map((f, i) => {
        const ext = f.type.split("/")[1]?.replace("jpeg", "jpg") ?? "png";
        return new File([f], `pasted-${stamp}${i ? `-${i + 1}` : ""}.${ext}`, { type: f.type });
      });
      if (view.kind === "pdf") setEntries([]);
      addFiles(named);
      if (view.kind === "pdf") setView({ kind: "files" });
    }
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [addFiles, view.kind]);

  const removeEntry = (id: string) => {
    const left = entries.filter((e) => e.id !== id);
    setEntries(left);
    if (left.length === 0) goHome();
  };

  return (
    <div className="page">
      <header className="header">
        <button className="brand" onClick={goHome} aria-label="AkarBox home">
          <img src="/favicon.svg" alt="" width={28} height={28} />
          <span>AkarBox</span>
        </button>
        <div className="row tight header-right">
          <span className="muted small server-info">
            {info
              ? `v${info.version} · ${info.converters.length} tools${info.hardware_video ? ` · GPU: ${info.hardware_video}` : ""}`
              : error
                ? "offline"
                : "connecting…"}
          </span>
          <button className="icon-button" onClick={nextTheme} aria-label={`Theme: ${theme}. Click to change.`} title={`Theme: ${theme}`}>
            <ThemeIcon theme={theme} />
          </button>
        </div>
      </header>

      <main className="main">
        {error && <ErrorCard message={`Can't reach the AkarBox server: ${error}`} />}

        {view.kind !== "home" && (
          <button className="link back" onClick={goHome}>
            ← All tools
          </button>
        )}

        {view.kind === "home" && (
          <Home
            info={info}
            onFiles={(f) => addFiles(f)}
            onPreset={(p, f) => addFiles(f, p)}
            onOpenPdf={(p) => setView({ kind: "pdf", tab: p?.pdfTab ?? "images" })}
          />
        )}

        {view.kind === "pdf" && (
          <PdfStudio key={session} initial={entries.filter((e) => isPdf(e.file))} initialTab={view.tab} pdfTools={info?.pdf_tools ?? false} />
        )}

        {view.kind === "files" && entries.length > 0 && (
          <>
            <DropZone onFiles={(f) => addFiles(f)} multiple compact>
              <span className="muted small">Drop, click or paste to add more files</span>
            </DropZone>
            {entries.length === 1 ? (
              <SingleWorkspace
                key={entries[0].id}
                file={entries[0].file}
                onRemove={goHome}
                initialTarget={view.target}
                initialOptions={view.options}
              />
            ) : (
              <BatchView
                key={session}
                entries={entries}
                onRemove={removeEntry}
                onClear={goHome}
                onMergePdfs={info?.pdf_tools ? () => setView({ kind: "pdf", tab: "merge" }) : undefined}
                initialTarget={view.target}
                initialOptions={view.options}
              />
            )}
          </>
        )}
      </main>

      <footer className="footer muted small">Files are processed on this server and deleted right after conversion.</footer>
    </div>
  );
}

function ThemeIcon({ theme }: { theme: Theme }) {
  const common = { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round" as const, "aria-hidden": true };
  if (theme === "light")
    return (
      <svg {...common}>
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
      </svg>
    );
  if (theme === "dark")
    return (
      <svg {...common}>
        <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
      </svg>
    );
  return (
    <svg {...common}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor" />
    </svg>
  );
}
