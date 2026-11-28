import { useEffect, useMemo, useState } from "react";
import type { Options, Target } from "./api";
import { ErrorCard, Progress, ResultCard } from "./components";
import { groupsFor, OptionsPanel } from "./OptionsPanel";
import { FilePreview } from "./Preview";
import { useConversion } from "./useConversion";
import { CATEGORY_LABELS, previewKind } from "./util";

/**
 * The one-file workspace. It adapts to what was dropped: previews for images,
 * audio and video, target groups that make sense for the file, and only the
 * relevant options.
 */
export function SingleWorkspace({
  file,
  onRemove,
  initialTarget,
  initialOptions,
  filter,
  fixedTarget,
  heading,
}: {
  file: File;
  onRemove?: () => void;
  initialTarget?: string | "same";
  initialOptions?: Options;
  /** Hide some targets (e.g. PDF Studio's "to images" tab hides PDF). */
  filter?: (t: Target) => boolean;
  /** Skip the format picker and always convert to this format. */
  fixedTarget?: string;
  heading?: string;
}) {
  const conv = useConversion(file, fixedTarget ?? initialTarget);
  const [options, setOptions] = useState<Options>(initialOptions ?? {});
  const [query, setQuery] = useState("");

  const targets = useMemo(() => (filter ? conv.targets.filter(filter) : conv.targets), [conv.targets, filter]);
  const chosen = targets.find((t) => t.format.ext === conv.target);
  const groups = groupsFor(conv.format, chosen);
  const working = conv.status === "working";

  // Changing options after a result means the result is stale.
  const { reset } = conv;
  useEffect(() => {
    reset();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [options]);

  const sections = useMemo(() => groupTargets(targets, conv.format?.ext, conv.format?.category, query), [targets, conv.format, query]);

  return (
    <div className="workspace">
      <section className="card">
        <div className="row between top">
          <FilePreview file={file} format={conv.format} />
          {onRemove && (
            <button className="icon-button" aria-label="Remove file" disabled={working} onClick={onRemove}>
              ✕
            </button>
          )}
        </div>
      </section>

      {conv.status === "detecting" && <p className="muted">Detecting format…</p>}

      {!fixedTarget && targets.length > 0 && (
        <section className="card">
          <div className="row between">
            <h2>{heading ?? "Convert to…"}</h2>
            {targets.length > 10 && (
              <input className="search" placeholder="Search formats" value={query} onChange={(e) => setQuery(e.target.value)} />
            )}
          </div>
          {sections.map(([title, list]) => (
            <div key={title} className="group">
              <h3>{title}</h3>
              <div className="chips">
                {list.map((t) => (
                  <button
                    key={t.format.ext}
                    className={"chip" + (conv.target === t.format.ext ? " selected" : "")}
                    title={t.format.name}
                    disabled={working}
                    onClick={() => conv.setTarget(t.format.ext)}
                  >
                    {t.format.ext === conv.format?.ext ? `Smaller ${t.format.ext}` : t.format.ext}
                  </button>
                ))}
              </div>
            </div>
          ))}
          {chosen && <Route target={chosen} />}
        </section>
      )}

      {conv.target && <OptionsPanel groups={groups} options={options} onChange={setOptions} disabled={working} />}

      {conv.target && (
        <div className="row actions">
          {working ? (
            <>
              <Progress upload={conv.upload} />
              <button className="button secondary" onClick={conv.cancel}>
                Cancel
              </button>
            </>
          ) : (
            <button className="button" onClick={() => conv.run(options)}>
              {actionLabel(conv.format?.ext, conv.target)}
            </button>
          )}
        </div>
      )}

      {conv.status === "done" && conv.result && (
        <ResultCard result={conv.result} originalSize={file.size} preview={previewKind(chosen?.format)} />
      )}
      {conv.status === "error" && conv.error && <ErrorCard message={conv.error} />}
    </div>
  );
}

function actionLabel(from: string | undefined, to: string): string {
  return from === to ? `Compress ${to.toUpperCase()}` : `Convert to ${to.toUpperCase()}`;
}

/** "Route: md → docx → pdf via pandoc + libreoffice" */
export function Route({ target }: { target: Target }) {
  const steps = target.plan.steps;
  const same = steps.length === 1 && steps[0].from === steps[0].to;
  return (
    <p className="muted small route">
      {same ? "Compress" : "Route"}: {[steps[0].from, ...steps.map((s) => s.to)].join(" → ")} via{" "}
      {[...new Set(steps.map((s) => s.converter))].join(" + ")}
    </p>
  );
}

/** Group targets into titled sections, tailored to the kind of input. */
function groupTargets(targets: Target[], ext: string | undefined, category: string | undefined, query: string): [string, Target[]][] {
  const q = query.trim().toLowerCase();
  const visible = targets.filter((t) => !q || t.format.ext.includes(q) || t.format.name.toLowerCase().includes(q));
  const sections = new Map<string, Target[]>();
  const add = (title: string, t: Target) => sections.set(title, [...(sections.get(title) ?? []), t]);

  for (const t of visible) {
    if (t.format.ext === ext) add("Make it smaller", t);
    // From a video, only video, audio and GIF make sense (not "first frame as PDF").
    else if (category === "video" && !["video", "audio"].includes(t.format.category) && t.format.ext !== "gif") continue;
    else if (category === "video" && t.format.ext === "gif") add("Make a GIF", t);
    else if (category === "video" && t.format.category === "audio") add("Extract audio", t);
    else if (category === "video" && t.format.category === "video") add("Video formats", t);
    else add(CATEGORY_LABELS[t.format.category], t);
  }
  // Put the most relevant section first: same category as the input.
  return [...sections.entries()].sort(([a], [b]) => rank(a, category) - rank(b, category));
}

function rank(title: string, category: string | undefined): number {
  if (category === "video") return ["Video formats", "Extract audio", "Make a GIF", "Make it smaller"].indexOf(title) + 1 || 9;
  if (category && title === CATEGORY_LABELS[category as keyof typeof CATEGORY_LABELS]) return 0;
  if (title === "Make it smaller") return 1;
  return 5;
}
