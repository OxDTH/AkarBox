# syntax=docker/dockerfile:1
# AkarBox self-hosted image: `docker compose up -d`, then open http://localhost:8080

# --- 1. Build the web UI -------------------------------------------------------
FROM node:22-bookworm-slim AS web
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# --- 2. Build the Rust server (the UI gets embedded into the binary) -----------
FROM rust:1-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY --from=web /src/frontend/dist ./frontend/dist
RUN cargo build --release --locked -p akarbox-server

# --- 3. Runtime: the server + the converter tools it drives --------------------
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates ffmpeg imagemagick ghostscript librsvg2-bin pandoc qpdf libheif-examples \
      libreoffice-writer-nogui libreoffice-calc-nogui libreoffice-impress-nogui \
      fonts-dejavu fonts-liberation \
 && rm -rf /var/lib/apt/lists/* \
 # Debian's ImageMagick policy blocks PDF by default; we need it for PDF <-> image.
 && sed -i 's/rights="none" pattern="PDF"/rights="read|write" pattern="PDF"/' /etc/ImageMagick-6/policy.xml \
 && useradd --system --create-home --uid 10001 akarbox

COPY --from=server /src/target/release/akarbox-server /usr/local/bin/akarbox-server

USER akarbox
ENV AKARBOX_ADDR=0.0.0.0:8080 \
    HOME=/home/akarbox \
    TMPDIR=/tmp
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD ["sh", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080"]
ENTRYPOINT ["akarbox-server"]
