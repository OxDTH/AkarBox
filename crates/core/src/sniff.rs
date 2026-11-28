//! Recognise a file from its first bytes ("magic numbers").
//!
//! Most binary formats start with a fixed signature: every PNG begins with
//! `\x89PNG`, every PDF with `%PDF-`. Reading those bytes is more reliable than
//! trusting the file name.

/// Returns the canonical extension of the format `header` looks like, or `None`
/// if we can't tell (plain-text formats, unknown data, or ambiguous containers
/// such as old `.doc`/`.xls` files).
pub fn sniff(header: &[u8]) -> Option<&'static str> {
    // `starts_with` on a byte slice compares the first bytes.
    let h = header;
    let at = |offset: usize, magic: &[u8]| h.get(offset..offset + magic.len()) == Some(magic);

    // A `match` with no value to match on isn't allowed, so we use an if/else chain.
    // Order matters: more specific checks come first.
    if h.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if h.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if h.starts_with(b"GIF87a") || h.starts_with(b"GIF89a") {
        Some("gif")
    } else if h.starts_with(b"%PDF-") {
        Some("pdf")
    } else if h.starts_with(b"BM") && h.len() >= 14 {
        Some("bmp")
    } else if h.starts_with(b"FUJIFILMCCD-RAW") {
        Some("raf")
    } else if h.starts_with(b"IIU\0") {
        Some("rw2")
    } else if h.starts_with(b"IIRO") || h.starts_with(b"IIRS") || h.starts_with(b"MMOR") {
        Some("orf")
    } else if h.starts_with(b"II*\0") && at(8, b"CR") {
        Some("cr2")
    } else if h.starts_with(b"II*\0") || h.starts_with(b"MM\0*") {
        // Plain TIFF -- but NEF, ARW and DNG are TIFF files too, so
        // `same_family` lets their extension win.
        Some("tiff")
    } else if h.starts_with(&[0, 0, 1, 0]) {
        Some("ico")
    } else if h.starts_with(b"RIFF") && at(8, b"WEBP") {
        Some("webp")
    } else if h.starts_with(b"RIFF") && at(8, b"WAVE") {
        Some("wav")
    } else if h.starts_with(b"RIFF") && at(8, b"AVI ") {
        Some("avi")
    } else if h.starts_with(b"fLaC") {
        Some("flac")
    } else if h.starts_with(b"OggS") {
        // Opus and Vorbis both live in Ogg; the codec header follows the page header.
        if contains(h, b"OpusHead") {
            Some("opus")
        } else {
            Some("ogg")
        }
    } else if h.starts_with(b"ID3") || is_mp3_frame(h) {
        Some("mp3")
    } else if h.len() >= 2 && h[0] == 0xFF && (h[1] & 0xF6) == 0xF0 {
        Some("aac") // ADTS frame header
    } else if at(4, b"ftyp") {
        ftyp(h)
    } else if h.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        // Matroska. WebM is a restricted Matroska that declares "webm" as its doctype.
        if contains(h, b"webm") {
            Some("webm")
        } else {
            Some("mkv")
        }
    } else if h.starts_with(b"PK\x03\x04") {
        zip_based(h)
    } else if h.starts_with(b"{\\rtf") {
        Some("rtf")
    } else if looks_like_svg(h) {
        Some("svg")
    } else {
        None
    }
}

/// ISO base media files (MP4, MOV, HEIC, AVIF...) start with a `ftyp` box whose
/// "major brand" (bytes 8..12) says what the file is.
fn ftyp(h: &[u8]) -> Option<&'static str> {
    let brand = h.get(8..12)?;
    Some(match brand {
        b"heic" | b"heix" | b"hevc" | b"heim" | b"heis" | b"hevx" => "heic",
        b"mif1" | b"msf1" => {
            if contains(h, b"avif") {
                "avif"
            } else {
                "heif"
            }
        }
        b"avif" | b"avis" => "avif",
        b"qt  " => "mov",
        b"crx " => "cr3", // Canon CR3 is an ISO media file
        b"M4A " | b"M4B " => "m4a",
        _ => "mp4",
    })
}

/// DOCX, XLSX, PPTX, ODT, EPUB... are all ZIP files. Peek at the first entries.
fn zip_based(h: &[u8]) -> Option<&'static str> {
    // OpenDocument and EPUB store an uncompressed `mimetype` file first.
    let checks: &[(&[u8], &str)] = &[
        (b"application/epub+zip", "epub"),
        (b"application/vnd.oasis.opendocument.text", "odt"),
        (b"application/vnd.oasis.opendocument.spreadsheet", "ods"),
        (b"application/vnd.oasis.opendocument.presentation", "odp"),
        (b"word/", "docx"),
        (b"xl/", "xlsx"),
        (b"ppt/", "pptx"),
    ];
    checks
        .iter()
        .find(|(needle, _)| contains(h, needle))
        .map(|(_, ext)| *ext)
}

fn is_mp3_frame(h: &[u8]) -> bool {
    // MPEG audio frame sync: 11 set bits, then layer bits != 00 and MPEG version.
    h.len() >= 2 && h[0] == 0xFF && (h[1] & 0xE0) == 0xE0 && (h[1] & 0x06) != 0
}

fn looks_like_svg(h: &[u8]) -> bool {
    let text = String::from_utf8_lossy(&h[..h.len().min(1024)]).to_ascii_lowercase();
    let text = text.trim_start_matches('\u{feff}').trim_start();
    (text.starts_with("<?xml") || text.starts_with("<svg") || text.starts_with("<!--"))
        && text.contains("<svg")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Formats that share a container, where the extension is a better guess than
/// the bytes (an `.m4a` and an `.mp4` look the same).
pub fn same_family(a: &str, b: &str) -> bool {
    const FAMILIES: &[&[&str]] = &[
        &["mp4", "m4a", "mov"],
        &["mkv", "webm"],
        &["ogg", "opus"],
        &["heic", "heif", "avif"],
        &["mp3", "aac"],
        &["tiff", "dng", "nef", "arw", "cr2"],
    ];
    a == b || FAMILIES.iter().any(|f| f.contains(&a) && f.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::sniff;

    #[test]
    fn recognises_common_signatures() {
        // (bytes, expected) pairs. `b"..."` is a byte-string literal.
        let cases: &[(&[u8], &str)] = &[
            (b"\x89PNG\r\n\x1a\n....", "png"),
            (b"\xFF\xD8\xFF\xE0..JFIF", "jpg"),
            (b"GIF89a....", "gif"),
            (b"%PDF-1.7\n", "pdf"),
            (b"RIFF\0\0\0\0WEBPVP8 ", "webp"),
            (b"RIFF\0\0\0\0WAVEfmt ", "wav"),
            (b"fLaC\0\0\0\x22", "flac"),
            (b"OggS\0\x02..........OpusHead", "opus"),
            (b"OggS\0\x02...........vorbis", "ogg"),
            (b"ID3\x04\0\0\0\0\0\0", "mp3"),
            (b"\0\0\0\x18ftypheic\0\0\0\0mif1heic", "heic"),
            (b"\0\0\0\x1cftypavif\0\0\0\0avifmif1", "avif"),
            (b"\0\0\0\x14ftypqt  \0\0\0\0", "mov"),
            (
                b"\x1A\x45\xDF\xA3\x9f\x42\x86\x81\x01\x42\x82\x84webm",
                "webm",
            ),
            (
                b"PK\x03\x04....[Content_Types].xml....word/document.xml",
                "docx",
            ),
            (b"PK\x03\x04....mimetypeapplication/epub+zip", "epub"),
            (b"{\\rtf1\\ansi", "rtf"),
            (b"II*\0\x10\0\0\0CR\x02\0", "cr2"),
            (b"FUJIFILMCCD-RAW 0201", "raf"),
            (b"IIU\0\x18\0\0\0", "rw2"),
            (b"\0\0\0\x18ftypcrx \0\0\0\x01", "cr3"),
            (b"<?xml version=\"1.0\"?>\n<svg xmlns=", "svg"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(
                sniff(bytes),
                Some(*expected),
                "for {:?}",
                String::from_utf8_lossy(bytes)
            );
        }
    }

    #[test]
    fn unknown_or_text_returns_none() {
        assert_eq!(sniff(b"# Markdown title"), None);
        assert_eq!(sniff(b"a,b,c\n1,2,3"), None);
        assert_eq!(sniff(b""), None);
        // Legacy Office (OLE) files are ambiguous: .doc, .xls and .ppt look alike.
        assert_eq!(sniff(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1"), None);
    }
}
