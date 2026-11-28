import { useRef, useState } from "react";
import type { Info } from "./api";
import { DropZone } from "./components";
import { acceptFor, availablePresets, HUBS, hubOf, type Hub, type Preset } from "./presets";

/**
 * The landing page: a drop zone plus the five areas AkarBox covers, with the
 * formats this server can actually handle (from `/api/info`).
 */
export function Home({ info, onFiles, onPreset, onOpenPdf }: {
  info: Info | null;
  onFiles: (files: File[]) => void;
  onPreset: (preset: Preset, files: File[]) => void;
  onOpenPdf: (preset?: Preset) => void;
}) {
  const [open, setOpen] = useState<Hub | null>(null);
  const picker = useRef<HTMLInputElement>(null);
  const pending = useRef<Preset | null>(null);
  const presets = info ? availablePresets(info) : [];

  function startPreset(p: Preset) {
    if (p.pdfTab) return onOpenPdf(p);
    pending.current = p;
    if (picker.current) {
      picker.current.accept = acceptFor(p.from);
      picker.current.click();
    }
  }

  const formatsIn = (hub: Hub, list: string[]) =>
    (info?.formats ?? []).filter((f) => list.includes(f.ext) && hubOf(f.ext, f.category) === hub);

  return (
    <div className="home">
      <DropZone onFiles={onFiles} multiple>
        <span className="drop-icon" aria-hidden>
          ⇪
        </span>
        <strong>Drop files to convert</strong>
        <span className="muted">click to browse, or paste an image with Ctrl+V / ⌘V</span>
      </DropZone>

      <input
        ref={picker}
        type="file"
        hidden
        multiple
        onChange={(e) => {
          const files = [...(e.target.files ?? [])];
          if (files.length && pending.current) onPreset(pending.current, files);
          e.target.value = "";
        }}
      />

      <div className="hub-grid">
        {HUBS.map((hub) => {
          const reads = formatsIn(hub.id, info?.inputs ?? []);
          const writes = formatsIn(hub.id, info?.outputs ?? []);
          const hubPresets = presets.filter((p) => p.hub === hub.id);
          const available = reads.length > 0;
          const expanded = open === hub.id;
          return (
            <section key={hub.id} className={"hub-card" + (expanded ? " expanded" : "") + (available ? "" : " unavailable")}>
              <button
                className="hub-head"
                aria-expanded={expanded}
                onClick={() => (hub.id === "pdf" && available ? onOpenPdf() : setOpen(expanded ? null : hub.id))}
              >
                <span className="hub-icon" aria-hidden>
                  {hub.icon}
                </span>
                <span className="hub-text">
                  <strong>{hub.title}</strong>
                  <span className="muted small">{available ? hub.blurb : "Not available on this server"}</span>
                </span>
              </button>
              {available && (
                <div className="formats">
                  {reads.slice(0, expanded ? undefined : 6).map((f) => (
                    <span key={f.ext} className="format-tag" title={f.name}>
                      {f.ext}
                    </span>
                  ))}
                  {!expanded && reads.length > 6 && (
                    <button className="link small" onClick={() => setOpen(hub.id)}>
                      +{reads.length - 6} more
                    </button>
                  )}
                </div>
              )}
              {expanded && writes.length > 0 && (
                <p className="muted small">
                  Can produce: {writes.map((f) => f.ext.toUpperCase()).join(", ")}
                </p>
              )}
              {hubPresets.length > 0 && (
                <div className="presets">
                  {hubPresets.map((p) => (
                    <button key={p.id} className="preset" onClick={() => startPreset(p)}>
                      {p.label}
                    </button>
                  ))}
                </div>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}
