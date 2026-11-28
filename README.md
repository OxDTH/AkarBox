<p align="center">
  <img src="frontend/public/favicon.svg" alt="AkarBox logo" width="96" height="96">
</p>

<h1 align="center">AkarBox</h1>

<p align="center">
  A lightweight, self-hosted file converter: drop in a file, pick any format it can become, download the result.
</p>

> **Status:** early development. The core engine, web UI and desktop app work; expect rough edges.

## About us

We're students learning Rust by building something useful. AkarBox is our way to learn the language on a real project while giving the self-hosting community a free, private, open-source converter.

That means the code is written to be readable for beginners: Rust concepts are explained in comments, and [docs/LEARNING_RUST.md](docs/LEARNING_RUST.md) maps each concept to where it appears. If you're learning Rust too, you're very welcome here. Questions, reviews and pull requests are all appreciated, and if you're experienced with Rust, we'd love your feedback on how to write it better.

## How it works

AkarBox wraps proven open-source tools (**FFmpeg**, **ImageMagick**, **LibreOffice**, **Pandoc**) behind one interface. Each tool declares the conversions it can do, and AkarBox treats them as a graph. When no single tool can do what you ask, it chains them by picking the cheapest route:

```
md ──pandoc──▶ docx ──libreoffice──▶ pdf ──imagemagick──▶ png
```

So adding one converter makes new combinations possible with every other converter, without extra work.

## Features

- **Convert anything to anything** that the installed tools can reach, chaining tools when needed
- **A home screen that shows what this server can do**, with one-click presets like *HEIC → JPG* or *Compress PDF*
- **Adaptive workspaces**: image previews with pixel size, audio and video players with duration, options that match the file
- **Batch conversion**: drop many files, pick a format per file or one for all, then download everything as a `.zip`
- **Paste a screenshot** with Ctrl+V / ⌘V
- **Compress without changing format**: PDFs, images, video and audio, with Light / Balanced / Strong levels
- **Hit a file-size limit**: "under 100 KB" for photos or "under 1 MB" for PDFs (for forms and email)
- **Smart format detection** from the file's contents, so wrong or missing extensions still work
- **PDF Studio**: pages to images (zipped), compress, merge (drag to reorder), extract page ranges like `1-3, 5, 8-end`
- **iPhone photos (HEIC) and camera RAW** (CR2, CR3, NEF, ARW, DNG, RAF, RW2, ORF)
- **GPU video encoding** (NVIDIA NVENC, Intel Quick Sync, VAAPI, Apple VideoToolbox) when available, with automatic CPU fallback
- **Size comparison** on every result (`4.2 MB → 680 KB (−84%)`) and a light / dark theme switch

## Supported formats

| Category      | Formats                                              | Tool        |
|---------------|------------------------------------------------------|-------------|
| Images        | png, jpg, webp, gif, bmp, tiff, ico, avif, svg (in)  | ImageMagick |
| iPhone photos | heic, heif (in)                                      | libheif     |
| Camera RAW    | cr2, cr3, nef, arw, dng, raf, rw2, orf (in)          | LibRaw / dcraw |
| Audio         | mp3, wav, flac, ogg, opus, m4a, aac                  | FFmpeg      |
| Video         | mp4, mkv, webm, mov, avi, → gif, → audio             | FFmpeg      |
| Office        | docx, doc, odt, rtf, xlsx, xls, ods, csv, pptx, ppt, odp → pdf | LibreOffice |
| Markup/ebooks | md, html, rst, tex, epub, txt                        | Pandoc      |
| PDF           | pdf ↔ images, compress, merge, extract pages         | Ghostscript, ImageMagick, qpdf |

Converters whose tool isn't installed are skipped automatically.

## Run it

### Docker (home server)

```bash
docker compose up -d
```

Then open <http://localhost:8080>.

| Variable               | Default        | Meaning                                  |
|------------------------|----------------|------------------------------------------|
| `AKARBOX_ADDR`         | `0.0.0.0:8080` | Listen address                           |
| `AKARBOX_MAX_UPLOAD_MB`| `1024`         | Largest accepted upload                  |
| `AKARBOX_MAX_JOBS`     | CPU count      | Conversions allowed to run at once       |
| `AKARBOX_HWACCEL`      | `auto`         | GPU video encoding: `auto`, `off`, `nvenc`, `qsv`, `vaapi`, `videotoolbox` |
| `RUST_LOG`             | `info`         | Log level (`debug` shows tool commands)  |

**GPU video encoding.** With `auto`, AkarBox test-encodes one frame on each GPU encoder at startup and uses the first one that works. If a GPU encode fails later, it retries on the CPU. To use a GPU from Docker, uncomment the `devices` (Intel/AMD) or `deploy` (NVIDIA, needs the NVIDIA Container Toolkit) section in `docker-compose.yml`.

### Desktop app (Tauri)

The desktop app runs the same server privately on `127.0.0.1` and shows it in a native window. You need the converter tools installed on your system. On Debian/Ubuntu:

```bash
sudo apt install ffmpeg imagemagick ghostscript librsvg2-bin pandoc libreoffice qpdf libheif-examples libraw-bin
```

On Ubuntu 24.04 and newer, HEIC decoding also needs `libheif-plugin-libde265`.

## Develop

Prerequisites: [Rust](https://rustup.rs), Node.js 22+, and the converter tools above. The desktop app also needs the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/).

```bash
npm install                      # Tauri CLI
npm --prefix frontend install    # web UI dependencies
```

Run the server and the UI with hot reload (two terminals):

```bash
cargo run -p akarbox-server -- # API on :8080 (set AKARBOX_ADDR to change)
```

```bash
npm run dev:web                  # UI on :5173, proxies /api to :8080
```

Run the desktop app (build the UI first, since it is embedded into the server):

```bash
npm run build:web && cargo run -p akarbox-desktop
```

Build installers (`.deb`, `.AppImage`, `.msi`, `.dmg` depending on your OS):

```bash
npm run tauri build
```

Run tests and lints:

```bash
cargo test && cargo clippy --workspace --all-targets
```

Tests in `crates/core/tests/real_tools.rs` run the real converter tools and skip themselves if a tool isn't installed.

### Project layout

```
crates/
  core/      akarbox-core:    formats, the Converter trait, the route planner, built-in converters
  server/    akarbox-server:  axum HTTP API + embedded web UI (the Docker image)
  desktop/   akarbox-desktop: Tauri shell that runs the server and opens a window
frontend/    React + TypeScript + Vite web UI
docs/        Contributor and learning guides
```

## Contributing

New converters are the easiest way to help. See [docs/ADDING_A_CONVERTER.md](docs/ADDING_A_CONVERTER.md). New to Rust? [docs/LEARNING_RUST.md](docs/LEARNING_RUST.md) maps Rust concepts to where they appear in this codebase.

## Roadmap

- Target file size for video and audio
- Page thumbnails and page picking for PDFs
- Optional media downloader (off by default, since it needs internet access)

## Privacy & Security

All processing happens on your machine or server. AkarBox has no telemetry and makes no outbound network calls. Each conversion runs in its own temporary directory that is deleted as soon as the response is sent.
