//! The catalogue of file formats AkarBox knows about.
//!
//! A format is identified by its canonical file extension (`"jpg"`, `"docx"`...).
//! Converters refer to formats by these strings.

use serde::Serialize;
use std::path::Path;

/// `#[derive(...)]` asks the compiler to write trait implementations for us:
/// `Serialize` (to JSON), `Clone`/`Copy` (cheap duplication), `PartialEq` (`==`), ...
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Image,
    Audio,
    Video,
    Document,
    Spreadsheet,
    Presentation,
    Ebook,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Format {
    /// Canonical extension, lowercase, no dot.
    pub ext: &'static str,
    /// Human-friendly name for the UI.
    pub name: &'static str,
    pub category: Category,
}

// A tiny helper so the table below stays readable.
const fn f(ext: &'static str, name: &'static str, category: Category) -> Format {
    Format {
        ext,
        name,
        category,
    }
}

use Category::*;

/// Every format the UI can show. `&'static [Format]` is a slice that lives for the
/// whole program (it is baked into the binary), so no allocation is needed.
pub static FORMATS: &[Format] = &[
    // Images
    f("png", "PNG image", Image),
    f("jpg", "JPEG image", Image),
    f("webp", "WebP image", Image),
    f("gif", "GIF image", Image),
    f("bmp", "Bitmap image", Image),
    f("tiff", "TIFF image", Image),
    f("ico", "Icon", Image),
    f("avif", "AVIF image", Image),
    f("svg", "SVG vector image", Image),
    f("heic", "HEIC photo (iPhone)", Image),
    f("heif", "HEIF image", Image),
    // Audio
    f("mp3", "MP3 audio", Audio),
    f("wav", "WAV audio", Audio),
    f("flac", "FLAC audio", Audio),
    f("ogg", "Ogg Vorbis audio", Audio),
    f("opus", "Opus audio", Audio),
    f("m4a", "M4A (AAC) audio", Audio),
    f("aac", "AAC audio", Audio),
    // Video
    f("mp4", "MP4 video", Video),
    f("mkv", "Matroska video", Video),
    f("webm", "WebM video", Video),
    f("mov", "QuickTime video", Video),
    f("avi", "AVI video", Video),
    // Documents
    f("pdf", "PDF document", Document),
    f("docx", "Word document", Document),
    f("doc", "Word 97-2003 document", Document),
    f("odt", "OpenDocument text", Document),
    f("rtf", "Rich Text Format", Document),
    f("txt", "Plain text", Document),
    f("md", "Markdown", Document),
    f("html", "HTML page", Document),
    f("rst", "reStructuredText", Document),
    f("tex", "LaTeX source", Document),
    // Spreadsheets
    f("xlsx", "Excel spreadsheet", Spreadsheet),
    f("xls", "Excel 97-2003 spreadsheet", Spreadsheet),
    f("ods", "OpenDocument spreadsheet", Spreadsheet),
    f("csv", "Comma-separated values", Spreadsheet),
    // Presentations
    f("pptx", "PowerPoint presentation", Presentation),
    f("ppt", "PowerPoint 97-2003 presentation", Presentation),
    f("odp", "OpenDocument presentation", Presentation),
    // Ebooks
    f("epub", "EPUB ebook", Ebook),
];

impl Format {
    /// Look a format up by extension. Returns `None` if we don't know it.
    ///
    /// `Option<T>` is Rust's "maybe a value": either `Some(value)` or `None`.
    pub fn get(ext: &str) -> Option<&'static Format> {
        let ext = normalize_ext(ext);
        FORMATS.iter().find(|f| f.ext == ext)
    }
}

/// Lowercase an extension and map common aliases to the canonical one.
pub fn normalize_ext(ext: &str) -> String {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    // `match` is like a switch statement, but the compiler checks you covered every case.
    match ext.as_str() {
        "jpeg" | "jpe" => "jpg".into(),
        "tif" => "tiff".into(),
        "htm" => "html".into(),
        "markdown" => "md".into(),
        "text" => "txt".into(),
        "hif" => "heif".into(),
        _ => ext,
    }
}

/// Guess a file's format from its name only, e.g. `"Holiday.JPEG"` -> `jpg`.
pub fn format_from_name(file_name: &str) -> Option<&'static Format> {
    let ext = Path::new(file_name).extension()?.to_str()?;
    //                                           ^ `?` on an Option returns `None` early.
    Format::get(ext)
}

/// Work out a file's format from its name *and* its first bytes.
///
/// `header` should be the first few KB of the file (4096 bytes is plenty).
/// The content wins when it is unambiguous, so a PNG renamed to `photo.jpg`, or a
/// file with no extension at all, is still detected correctly. Formats that can't
/// be recognised from their bytes (Markdown, CSV, ...) fall back to the extension.
pub fn detect_format(file_name: &str, header: &[u8]) -> Option<&'static Format> {
    let by_name = format_from_name(file_name);
    let Some(sniffed) = crate::sniff::sniff(header) else {
        return by_name;
    };
    match by_name {
        // Some containers are shared by several formats (an `.m4a` *is* an MP4
        // file). If the extension is a sibling of what we sniffed, trust the user.
        Some(named) if crate::sniff::same_family(named.ext, sniffed) => Some(named),
        _ => Format::get(sniffed),
    }
}

// Unit tests live right next to the code. Run them with `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aliases_and_case() {
        assert_eq!(format_from_name("Holiday.JPEG").unwrap().ext, "jpg");
        assert_eq!(format_from_name("notes.markdown").unwrap().ext, "md");
        assert_eq!(format_from_name("archive.tar.xz").map(|f| f.ext), None);
        assert!(format_from_name("no_extension").is_none());
    }

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

    #[test]
    fn content_beats_a_wrong_extension() {
        assert_eq!(detect_format("photo.jpg", PNG).unwrap().ext, "png");
        assert_eq!(detect_format("no_extension", PNG).unwrap().ext, "png");
    }

    #[test]
    fn falls_back_to_extension_for_text_formats() {
        assert_eq!(detect_format("notes.md", b"# Hello").unwrap().ext, "md");
        assert_eq!(detect_format("data.csv", b"a,b\n1,2").unwrap().ext, "csv");
        assert!(detect_format("mystery", b"just some text").is_none());
    }

    #[test]
    fn keeps_extension_within_the_same_family() {
        let m4a = b"\0\0\0\x20ftypisom\0\0\x02\0isomiso2mp41";
        assert_eq!(detect_format("song.m4a", m4a).unwrap().ext, "m4a");
        assert_eq!(detect_format("clip", m4a).unwrap().ext, "mp4");
    }

    #[test]
    fn extensions_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for f in FORMATS {
            assert!(seen.insert(f.ext), "duplicate format {}", f.ext);
        }
    }
}
