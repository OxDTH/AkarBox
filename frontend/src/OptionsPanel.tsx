import type { Compression, Format, Options, Target } from "./api";

/** Which settings make sense for the chosen conversion(s). */
export interface OptionGroups {
  image: boolean;
  audio: boolean;
  video: boolean;
  pdfRender: boolean;
  /** Show the compression level picker. */
  compression: boolean;
  /** Show "target file size" (images and PDF compression). */
  targetSize: boolean;
}

export const NO_OPTIONS: OptionGroups = {
  image: false, audio: false, video: false, pdfRender: false, compression: false, targetSize: false,
};

/** Work out the groups for one `source -> target` conversion. */
export function groupsFor(source: Format | undefined, target: Target | undefined): OptionGroups {
  if (!source || !target) return NO_OPTIONS;
  const to = target.format;
  const same = source.ext === to.ext;
  const rasterOut = to.category === "image" && to.ext !== "svg";
  const lastStep = target.plan.steps[target.plan.steps.length - 1];
  const viaImageMagick = lastStep?.converter === "imagemagick";
  return {
    image: rasterOut && viaImageMagick && to.ext !== "ico",
    audio: to.category === "audio" || (to.category === "video" && to.ext !== "gif"),
    video: to.category === "video",
    pdfRender: source.ext === "pdf" && rasterOut,
    compression: same || (to.category === "video" && to.ext !== "avi") || (to.category === "audio" && !["wav", "flac"].includes(to.ext)),
    targetSize: (rasterOut && viaImageMagick && to.ext !== "ico") || (same && to.ext === "pdf"),
  };
}

export function mergeGroups(list: OptionGroups[]): OptionGroups {
  return list.reduce(
    (acc, g) => ({
      image: acc.image || g.image,
      audio: acc.audio || g.audio,
      video: acc.video || g.video,
      pdfRender: acc.pdfRender || g.pdfRender,
      compression: acc.compression || g.compression,
      targetSize: acc.targetSize || g.targetSize,
    }),
    NO_OPTIONS,
  );
}

const SIZE_PRESETS = [20, 50, 100, 200, 500, 1000, 2000];

/** Inline settings card. Only the groups that apply are shown. */
export function OptionsPanel({
  groups,
  options,
  onChange,
  disabled,
}: {
  groups: OptionGroups;
  options: Options;
  onChange: (o: Options) => void;
  disabled?: boolean;
}) {
  if (!Object.values(groups).some(Boolean)) return null;
  const set = (patch: Partial<Options>) => onChange({ ...options, ...patch });
  const num = (v: string) => (v === "" ? undefined : Number(v));
  const changed = Object.values(options).some((v) => v !== undefined && v !== false);
  const customSize = options.target_size_kb !== undefined && !SIZE_PRESETS.includes(options.target_size_kb);

  return (
    <section className="card options" aria-label="Options">
      <div className="row between">
        <h2>Options</h2>
        {changed && (
          <button className="link" onClick={() => onChange({})} disabled={disabled}>
            Reset
          </button>
        )}
      </div>
      <fieldset className="options-grid" disabled={disabled}>
        {groups.targetSize && (
          <div className="option-wide">
            <span className="label">Maximum file size</span>
            <div className="chips">
              <button className={"chip" + (options.target_size_kb === undefined ? " selected" : "")} onClick={() => set({ target_size_kb: undefined })}>
                No limit
              </button>
              {SIZE_PRESETS.map((kb) => (
                <button
                  key={kb}
                  className={"chip" + (options.target_size_kb === kb ? " selected" : "")}
                  onClick={() => set({ target_size_kb: kb })}
                >
                  {kb >= 1000 ? `${kb / 1000} MB` : `${kb} KB`}
                </button>
              ))}
              <label className={"chip custom" + (customSize ? " selected" : "")}>
                <input
                  type="number"
                  min={1}
                  placeholder="Custom"
                  aria-label="Custom maximum size in KB"
                  value={customSize ? options.target_size_kb : ""}
                  onChange={(e) => set({ target_size_kb: num(e.target.value) })}
                />
                KB
              </label>
            </div>
          </div>
        )}
        {groups.compression && (
          <label>
            <span className="label">Compression</span>
            <select value={options.compression ?? ""} onChange={(e) => set({ compression: (e.target.value || undefined) as Compression | undefined })}>
              <option value="">Default</option>
              <option value="light">Light: best quality</option>
              <option value="balanced">Balanced</option>
              <option value="strong">Strong: smallest file</option>
            </select>
          </label>
        )}
        {groups.image && (
          <>
            <label>
              <span className="label">
                Quality <span className="muted">{options.quality ?? "auto"}</span>
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
              <span className="label">Max width (px)</span>
              <input type="number" min={1} placeholder="original" value={options.max_width ?? ""} onChange={(e) => set({ max_width: num(e.target.value) })} />
            </label>
            <label>
              <span className="label">Max height (px)</span>
              <input type="number" min={1} placeholder="original" value={options.max_height ?? ""} onChange={(e) => set({ max_height: num(e.target.value) })} />
            </label>
          </>
        )}
        {groups.pdfRender && (
          <label>
            <span className="label">Page resolution</span>
            <select value={options.dpi ?? ""} onChange={(e) => set({ dpi: num(e.target.value) })}>
              <option value="">150 DPI (default)</option>
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
              <span className="label">Resolution</span>
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
              <input type="checkbox" checked={options.strip_audio ?? false} onChange={(e) => set({ strip_audio: e.target.checked || undefined })} />
              <span>Remove audio</span>
            </label>
          </>
        )}
        {groups.audio && !(groups.video && options.strip_audio && !groups.compression) && (
          <label>
            <span className="label">Audio bitrate</span>
            <select value={options.audio_bitrate_kbps ?? ""} onChange={(e) => set({ audio_bitrate_kbps: num(e.target.value) })}>
              <option value="">Default</option>
              {[64, 96, 128, 192, 256, 320].map((k) => (
                <option key={k} value={k}>
                  {k} kbps
                </option>
              ))}
            </select>
          </label>
        )}
      </fieldset>
    </section>
  );
}
