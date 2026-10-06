# AkarBox

A lightweight, self-hosted file converter for documents, media, and web content.

> **Status:** Work in progress. Active development is underway; installation and usage guides will be available once the initial release is ready.

## Planned Features

- **PDF Utilities**: Merge, split, and compress PDF documents.
- **Document & Office Conversions**: Convert PDF to Word, PowerPoint, and Excel formats.
- **Image Conversions**: Convert between PDF and JPG, alongside general image format conversions.
- **Audio Processing**: Convert and process various audio formats.
- **Media Download**: Download media directly from YouTube.
- **Modular Converter Architecture**: Simple module interface allowing contributors to easily implement and plug in new converters.
- **Single-Binary Deployment**: High-performance Go backend with the web frontend embedded directly into the Go binary.
- **Self-Hosting & Distribution**: Ready-to-use Docker container for self-hosting, with cross-platform desktop installers planned for later releases.

## Privacy & Security

All file processing is performed entirely locally on your machine or host server. AkarBox includes zero telemetry, makes no outbound network calls, and processes files inside an isolated temporary directory where they are immediately deleted after conversion.
