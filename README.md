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

## Supported formats

| Category      | Formats                                              | Tool        |
|---------------|------------------------------------------------------|-------------|
| Images        | png, jpg, webp, gif, bmp, tiff, ico, avif, svg (in)  | ImageMagick |
| Audio         | mp3, wav, flac, ogg, opus, m4a, aac                  | FFmpeg      |
| Video         | mp4, mkv, webm, mov, avi, → gif, → audio             | FFmpeg      |
| Office        | docx, doc, odt, rtf, xlsx, xls, ods, csv, pptx, ppt, odp → pdf | LibreOffice |
| Markup/ebooks | md, html, rst, tex, epub, txt                        | Pandoc      |
| PDF           | pdf ↔ images                                         | ImageMagick + Ghostscript |

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
| `RUST_LOG`             | `info`         | Log level (`debug` shows tool commands)  |

### Desktop app (Tauri)

The desktop app runs the same server privately on `127.0.0.1` and shows it in a native window. You need the converter tools installed on your system. On Debian/Ubuntu:

```bash
sudo apt install ffmpeg imagemagick ghostscript librsvg2-bin pandoc libreoffice
```

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

- PDF merge / split / compress
- Multi-page PDF → images (as a zip)
- Batch conversion and a job queue with progress
- Conversion options (quality, resolution, bitrate)
- Detect formats from file contents, not just the extension
- Optional media downloader (off by default, since it needs internet access)

## Privacy & Security

All processing happens on your machine or server. AkarBox has no telemetry and makes no outbound network calls. Each conversion runs in its own temporary directory that is deleted as soon as the response is sent.
