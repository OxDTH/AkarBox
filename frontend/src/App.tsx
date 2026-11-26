import { useEffect, useState } from "react";
import { getInfo, type Info } from "./api";
import { ErrorCard } from "./components";
import { ConvertView } from "./ConvertView";
import { MergeView, SplitView } from "./PdfTools";

type Tab = "convert" | "merge" | "split";

const TABS: { id: Tab; label: string; pdf?: boolean }[] = [
  { id: "convert", label: "Convert" },
  { id: "merge", label: "Merge PDFs", pdf: true },
  { id: "split", label: "Split PDF", pdf: true },
];

export default function App() {
  const [info, setInfo] = useState<Info | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("convert");

  useEffect(() => {
    getInfo().then(setInfo).catch((e: Error) => setError(e.message));
  }, []);

  const tabs = TABS.filter((t) => !t.pdf || info?.pdf_tools);

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

      {tabs.length > 1 && (
        <nav className="tabs" role="tablist">
          {tabs.map((t) => (
            <button
              key={t.id}
              role="tab"
              aria-selected={tab === t.id}
              className={"tab" + (tab === t.id ? " active" : "")}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </nav>
      )}

      <main className="main">
        {error && <ErrorCard message={`Can't reach the AkarBox server: ${error}`} />}
        {/* Views stay mounted (just hidden) so switching tabs keeps your files. */}
        <div className="view" hidden={tab !== "convert"}>
          <ConvertView />
        </div>
        {info?.pdf_tools && (
          <>
            <div className="view" hidden={tab !== "merge"}>
              <MergeView />
            </div>
            <div className="view" hidden={tab !== "split"}>
              <SplitView />
            </div>
          </>
        )}
      </main>

      <footer className="footer muted small">Files are processed on this server and deleted right after conversion.</footer>
    </div>
  );
}
