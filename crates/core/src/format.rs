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
        _ => ext,
    }
}

/// Guess a file's format from its name, e.g. `"Holiday.JPEG"` -> `jpg`.
///
/// TODO(good first issue): also sniff the first bytes of the file ("magic numbers")
/// so files with a wrong or missing extension still work.
pub fn detect_format(file_name: &str) -> Option<&'static Format> {
    let ext = Path::new(file_name).extension()?.to_str()?;
    //                                           ^ `?` on an Option returns `None` early.
    Format::get(ext)
}

// Unit tests live right next to the code. Run them with `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aliases_and_case() {
        assert_eq!(detect_format("Holiday.JPEG").unwrap().ext, "jpg");
        assert_eq!(detect_format("notes.markdown").unwrap().ext, "md");
        assert_eq!(detect_format("archive.tar.xz").map(|f| f.ext), None);
        assert!(detect_format("no_extension").is_none());
    }

    #[test]
    fn extensions_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for f in FORMATS {
            assert!(seen.insert(f.ext), "duplicate format {}", f.ext);
        }
    }
}
