import type { Category, Info, Options } from "./api";

/** The five areas shown on the home screen. */
export type Hub = "images" | "pdf" | "audio" | "video" | "docs";

export const HUBS: { id: Hub; title: string; blurb: string; icon: string }[] = [
  { id: "images", title: "Images", blurb: "Photos, screenshots, iPhone HEIC and camera RAW", icon: "🖼" },
  { id: "pdf", title: "PDF Studio", blurb: "Pages to images, compress, merge and split", icon: "📄" },
  { id: "audio", title: "Audio", blurb: "Songs, voice notes and podcasts", icon: "🎵" },
  { id: "video", title: "Video", blurb: "Convert, shrink, make GIFs, extract audio", icon: "🎬" },
  { id: "docs", title: "Documents & Ebooks", blurb: "Word, Markdown, spreadsheets, slides, EPUB", icon: "📚" },
];

/** Which hub a format belongs to. */
export function hubOf(ext: string, category: Category): Hub {
  if (ext === "pdf") return "pdf";
  if (category === "image") return "images";
  if (category === "audio") return "audio";
  if (category === "video") return "video";
  return "docs";
}

export type PdfTab = "images" | "compress" | "merge" | "split";

export interface Preset {
  id: string;
  hub: Hub;
  label: string;
  /** Input formats this preset accepts. */
  from: string[];
  /** Output format, or "same" to compress without changing the format. */
  to?: string | "same";
  options?: Options;
  /** PDF Studio presets open a tab instead of a file picker. */
  pdfTab?: PdfTab;
}

const RAW = ["cr2", "cr3", "nef", "arw", "dng", "raf", "rw2", "orf"];
const VIDEO = ["mp4", "mkv", "webm", "mov", "avi"];

export const PRESETS: Preset[] = [
  { id: "heic-jpg", hub: "images", label: "HEIC → JPG", from: ["heic", "heif"], to: "jpg" },
  { id: "raw-jpg", hub: "images", label: "Camera RAW → JPG", from: RAW, to: "jpg" },
  { id: "png-webp", hub: "images", label: "PNG → WebP", from: ["png"], to: "webp" },
  {
    id: "photo-100k",
    hub: "images",
    label: "Photo under 100 KB",
    from: ["jpg", "png", "webp", "heic", "heif", ...RAW],
    to: "jpg",
    options: { target_size_kb: 100 },
  },
  { id: "pdf-images", hub: "pdf", label: "PDF → PNG pages", from: ["pdf"], pdfTab: "images" },
  { id: "pdf-compress", hub: "pdf", label: "Compress PDF", from: ["pdf"], pdfTab: "compress" },
  { id: "pdf-merge", hub: "pdf", label: "Merge PDFs", from: ["pdf"], pdfTab: "merge" },
  { id: "pdf-split", hub: "pdf", label: "Split PDF", from: ["pdf"], pdfTab: "split" },
  { id: "wav-mp3", hub: "audio", label: "WAV / FLAC → MP3", from: ["wav", "flac"], to: "mp3" },
  { id: "mp3-small", hub: "audio", label: "Shrink MP3", from: ["mp3"], to: "same" },
  { id: "video-mp3", hub: "video", label: "Video → MP3", from: VIDEO, to: "mp3" },
  { id: "video-gif", hub: "video", label: "Video → GIF", from: VIDEO, to: "gif" },
  { id: "video-small", hub: "video", label: "Compress video", from: ["mp4", "mkv", "webm", "mov"], to: "same" },
  { id: "docx-pdf", hub: "docs", label: "Word → PDF", from: ["docx", "doc", "odt"], to: "pdf" },
  { id: "md-pdf", hub: "docs", label: "Markdown → PDF", from: ["md"], to: "pdf" },
  { id: "pptx-pdf", hub: "docs", label: "Slides → PDF", from: ["pptx", "ppt", "odp"], to: "pdf" },
];

/** Only offer presets the server can actually do. */
export function availablePresets(info: Info): Preset[] {
  return PRESETS.filter((p) => {
    if (!p.from.some((f) => info.inputs.includes(f))) return false;
    if (p.pdfTab === "merge" || p.pdfTab === "split") return info.pdf_tools;
    if (p.pdfTab === "images") return info.outputs.includes("png");
    if (p.pdfTab === "compress" || p.to === "same") return true;
    return !p.to || info.outputs.includes(p.to);
  });
}

/** `accept` attribute for a file picker limited to these formats. */
export const acceptFor = (exts: string[]) => exts.map((e) => "." + e).join(",") + (exts.includes("jpg") ? ",.jpeg" : "");
