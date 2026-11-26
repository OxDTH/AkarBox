import type { Options } from "./api";

/** Which groups of settings make sense for the formats the user picked. */
export interface OptionGroups {
  image: boolean;
  audio: boolean;
  video: boolean;
  pdfRender: boolean;
}

export function OptionsPanel({
  groups,
  options,
  onChange,
}: {
  groups: OptionGroups;
  options: Options;
  onChange: (o: Options) => void;
}) {
  if (!groups.image && !groups.audio && !groups.video && !groups.pdfRender) return null;
  const set = (patch: Partial<Options>) => onChange({ ...options, ...patch });
  const num = (v: string) => (v === "" ? undefined : Number(v));
  const changed = Object.values(options).some((v) => v !== undefined && v !== false);

  return (
    <details className="options" open={changed}>
      <summary>Advanced options{changed && <span className="badge">customised</span>}</summary>
      <div className="options-grid">
        {groups.image && (
          <>
            <label>
              <span>
                Quality <span className="muted">{options.quality ?? "default"}</span>
              </span>
              <input
                type="range"
                min={1}
                max={100}
                value={options.quality ?? 90}
                onChange={(e) => set({ quality: Number(e.target.value) })}
              />
            </label>
            <label>
              <span>Max width (px)</span>
              <input
                type="number"
                min={1}
                placeholder="original"
                value={options.max_width ?? ""}
                onChange={(e) => set({ max_width: num(e.target.value) })}
              />
            </label>
            <label>
              <span>Max height (px)</span>
              <input
                type="number"
                min={1}
                placeholder="original"
                value={options.max_height ?? ""}
                onChange={(e) => set({ max_height: num(e.target.value) })}
              />
            </label>
          </>
        )}
        {groups.pdfRender && (
          <label>
            <span>PDF page resolution</span>
            <select value={options.dpi ?? ""} onChange={(e) => set({ dpi: num(e.target.value) })}>
              <option value="">Default (150 DPI)</option>
              <option value="72">72 DPI (screen)</option>
              <option value="100">100 DPI</option>
              <option value="200">200 DPI</option>
              <option value="300">300 DPI (print)</option>
            </select>
          </label>
        )}
        {groups.video && (
          <>
            <label>
              <span>Resolution</span>
              <select value={options.video_height ?? ""} onChange={(e) => set({ video_height: num(e.target.value) })}>
                <option value="">Original</option>
                {[2160, 1440, 1080, 720, 480, 360].map((h) => (
                  <option key={h} value={h}>
                    {h}p
                  </option>
                ))}
              </select>
            </label>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={options.strip_audio ?? false}
                onChange={(e) => set({ strip_audio: e.target.checked || undefined })}
              />
              <span>Remove audio</span>
            </label>
          </>
        )}
        {(groups.audio || groups.video) && (
          <label>
            <span>Audio bitrate</span>
            <select
              value={options.audio_bitrate_kbps ?? ""}
              onChange={(e) => set({ audio_bitrate_kbps: num(e.target.value) })}
            >
              <option value="">Default</option>
              {[64, 96, 128, 192, 256, 320].map((k) => (
                <option key={k} value={k}>
                  {k} kbps
                </option>
              ))}
            </select>
          </label>
        )}
      </div>
      {changed && (
        <button className="link" onClick={() => onChange({})}>
          Reset to defaults
        </button>
      )}
    </details>
  );
}
