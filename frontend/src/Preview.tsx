import { useEffect, useState } from "react";
import type { Format } from "./api";
import { formatBytes, formatDuration, previewKind } from "./util";

/**
 * Shows the dropped file: an image thumbnail, an audio player or a video player,
 * plus what we can learn from it (pixel size, duration). Nothing is uploaded.
 */
export function FilePreview({ file, format }: { file: File; format?: Format }) {
  const kind = previewKind(format);
  const [url, setUrl] = useState<string | null>(null);
  const [meta, setMeta] = useState<string | null>(null);
  const [broken, setBroken] = useState(false);

  useEffect(() => {
    if (!kind) return;
    const u = URL.createObjectURL(file);
    setUrl(u);
    setMeta(null);
    setBroken(false);
    return () => URL.revokeObjectURL(u);
  }, [file, kind]);

  const facts = [formatBytes(file.size), format?.name, meta].filter(Boolean).join(" · ");

  return (
    <div className="preview">
      {kind === "image" && url && !broken && (
        <img
          src={url}
          alt={`Preview of ${file.name}`}
          onLoad={(e) => setMeta(`${e.currentTarget.naturalWidth} × ${e.currentTarget.naturalHeight} px`)}
          onError={() => setBroken(true)}
        />
      )}
      {kind === "audio" && url && (
        <audio controls src={url} onLoadedMetadata={(e) => setMeta(formatDuration(e.currentTarget.duration))} />
      )}
      {kind === "video" && url && !broken && (
        <video
          controls
          muted
          preload="metadata"
          src={url}
          onError={() => setBroken(true)}
          onLoadedMetadata={(e) => {
            const v = e.currentTarget;
            setMeta(`${v.videoWidth} × ${v.videoHeight} · ${formatDuration(v.duration)}`);
          }}
        />
      )}
      {(!kind || broken) && (
        <div className="preview-placeholder" aria-hidden>
          {format?.ext.toUpperCase() ?? "FILE"}
        </div>
      )}
      <div className="file-info">
        <strong className="filename">{file.name}</strong>
        <span className="muted small">{facts}</span>
        {(!kind || broken) && format?.category === "image" && (
          <span className="muted small">Your browser can't preview {format.ext.toUpperCase()}, but AkarBox can convert it.</span>
        )}
      </div>
    </div>
  );
}
