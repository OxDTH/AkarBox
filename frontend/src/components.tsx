import { useRef, useState, type ReactNode } from "react";
import { formatBytes, sizeChange } from "./util";

export function DropZone({
  onFiles,
  multiple = false,
  accept,
  compact = false,
  disabled = false,
  children,
}: {
  onFiles: (files: File[]) => void;
  multiple?: boolean;
  accept?: string;
  compact?: boolean;
  disabled?: boolean;
  children: ReactNode;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [over, setOver] = useState(false);
  const open = () => !disabled && input.current?.click();

  return (
    <div
      className={"dropzone" + (over ? " over" : "") + (compact ? " compact" : "") + (disabled ? " disabled" : "")}
      onClick={open}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(false);
        const files = [...e.dataTransfer.files];
        if (files.length && !disabled) onFiles(multiple ? files : files.slice(0, 1));
      }}
      role="button"
      tabIndex={0}
      aria-disabled={disabled}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && open()}
    >
      <input
        ref={input}
        type="file"
        hidden
        multiple={multiple}
        accept={accept}
        onChange={(e) => {
          const files = [...(e.target.files ?? [])];
          if (files.length) onFiles(files);
          e.target.value = "";
        }}
      />
      {children}
    </div>
  );
}

export function Progress({ upload }: { upload: number }) {
  const uploading = upload < 1;
  return (
    <div className="progress">
      <div className="bar">
        <div
          className={"fill" + (uploading ? "" : " indeterminate")}
          style={{ width: uploading ? `${upload * 100}%` : undefined }}
        />
      </div>
      <span className="muted small">{uploading ? `Uploading ${Math.round(upload * 100)}%` : "Working…"}</span>
    </div>
  );
}

export interface Done {
  url: string;
  fileName: string;
  size: number;
}

/** Finished file: name, size change, optional preview, download button. */
export function ResultCard({
  result,
  originalSize,
  preview,
}: {
  result: Done;
  originalSize?: number;
  preview?: "image" | "audio" | "video" | null;
}) {
  return (
    <section className="card success">
      {preview === "image" && <img className="result-preview" src={result.url} alt="Converted result" />}
      {preview === "audio" && <audio className="result-preview" controls src={result.url} />}
      {preview === "video" && <video className="result-preview" controls muted src={result.url} />}
      <div className="row between">
        <div className="file-info">
          <strong className="filename">{result.fileName}</strong>
          <span className="muted small">
            {originalSize !== undefined ? sizeChange(originalSize, result.size) : formatBytes(result.size)}
          </span>
        </div>
        <a className="button" href={result.url} download={result.fileName}>
          Download
        </a>
      </div>
    </section>
  );
}

export function ErrorCard({ message }: { message: string }) {
  return (
    <section className="card error" role="alert">
      {message}
    </section>
  );
}
