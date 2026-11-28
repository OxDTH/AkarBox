//! User-tunable conversion settings (quality, size, bitrate...).
//!
//! Every field is optional: `None` means "use the converter's default", so
//! leaving options out behaves exactly like before.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// How hard to squeeze a file when compressing.
///
/// `rename_all = "lowercase"` means the JSON spells these `"light"`, `"balanced"`
/// and `"strong"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compression {
    /// Small savings, quality nearly untouched.
    Light,
    /// A good default: clearly smaller, still looks good.
    Balanced,
    /// As small as possible while staying usable.
    Strong,
}

/// `#[serde(default)]` lets the JSON omit any field; missing ones get `Default`.
/// `deny_unknown_fields` turns typos like `"qualty"` into an error instead of
/// being silently ignored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// Image quality, 1-100 (JPEG, WebP, AVIF; for PNG it sets compression effort).
    pub quality: Option<u8>,
    /// Shrink images or videos so they are at most this wide (pixels). Never enlarges.
    pub max_width: Option<u32>,
    /// Shrink images so they are at most this tall (pixels). Never enlarges.
    pub max_height: Option<u32>,
    /// Audio bitrate in kbit/s, e.g. 128, 192, 320.
    pub audio_bitrate_kbps: Option<u32>,
    /// Video height preset, e.g. 720 or 1080. Never enlarges.
    pub video_height: Option<u32>,
    /// Remove the audio track from video outputs.
    pub strip_audio: bool,
    /// Resolution used when rendering PDF pages to images (default 150).
    pub dpi: Option<u32>,
    /// Compress the output. Converting a file to its *own* format (e.g. `pdf -> pdf`)
    /// always compresses, using `Balanced` unless this says otherwise.
    pub compression: Option<Compression>,
    /// Keep making the result smaller until it is at most this many kilobytes
    /// (1 KB = 1000 bytes). Supported for images and PDFs.
    pub target_size_kb: Option<u32>,
}

impl Options {
    /// Reject values that make no sense, *before* we start any work.
    pub fn validate(&self) -> Result<()> {
        // A nested helper function: checks an optional value is inside a range.
        fn check<T: PartialOrd + std::fmt::Display + Copy>(
            name: &str,
            value: Option<T>,
            min: T,
            max: T,
        ) -> Result<()> {
            match value {
                Some(v) if v < min || v > max => Err(Error::InvalidOptions(format!(
                    "{name} must be between {min} and {max}, got {v}"
                ))),
                _ => Ok(()),
            }
        }
        check("quality", self.quality, 1, 100)?;
        check("max_width", self.max_width, 1, 20_000)?;
        check("max_height", self.max_height, 1, 20_000)?;
        check("audio_bitrate_kbps", self.audio_bitrate_kbps, 8, 512)?;
        check("video_height", self.video_height, 144, 4320)?;
        check("dpi", self.dpi, 36, 600)?;
        check("target_size_kb", self.target_size_kb, 1, 10_000_000)?;
        Ok(())
    }

    /// Parse options from JSON (as sent by the web UI) and validate them.
    pub fn from_json(json: &str) -> Result<Self> {
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        let options: Self =
            serde_json::from_str(json).map_err(|e| Error::InvalidOptions(e.to_string()))?;
        options.validate()?;
        Ok(options)
    }

    /// The compression level a step should use, if any.
    ///
    /// `from == to` (e.g. `png -> png`) only makes sense as compression, so it
    /// defaults to `Balanced`.
    pub fn compression_for(&self, from: &str, to: &str) -> Option<Compression> {
        match self.compression {
            Some(level) => Some(level),
            None if from == to => Some(Compression::Balanced),
            None => None,
        }
    }

    /// `target_size_kb` in bytes.
    pub fn target_bytes(&self) -> Option<u64> {
        self.target_size_kb.map(|kb| u64::from(kb) * 1000)
    }

    /// ImageMagick resize geometry such as `800x600>` (`>` = only shrink).
    pub(crate) fn resize_geometry(&self) -> Option<String> {
        match (self.max_width, self.max_height) {
            (None, None) => None,
            (w, h) => Some(format!(
                "{}x{}>",
                w.map(|w| w.to_string()).unwrap_or_default(),
                h.map(|h| h.to_string()).unwrap_or_default()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_json_means_defaults() {
        assert_eq!(Options::from_json("").unwrap(), Options::default());
        assert_eq!(Options::from_json("{}").unwrap(), Options::default());
    }

    #[test]
    fn parses_and_validates() {
        let o = Options::from_json(r#"{"quality": 80, "max_width": 800}"#).unwrap();
        assert_eq!(o.quality, Some(80));
        assert_eq!(o.resize_geometry().as_deref(), Some("800x>"));

        assert!(Options::from_json(r#"{"quality": 0}"#).is_err());
        assert!(Options::from_json(r#"{"qualty": 80}"#).is_err()); // typo
    }

    #[test]
    fn compression_defaults_for_same_format() {
        let o = Options::from_json(r#"{"target_size_kb": 100}"#).unwrap();
        assert_eq!(o.compression_for("png", "png"), Some(Compression::Balanced));
        assert_eq!(o.compression_for("png", "jpg"), None);
        assert_eq!(o.target_bytes(), Some(100_000));

        let o = Options::from_json(r#"{"compression": "strong"}"#).unwrap();
        assert_eq!(o.compression_for("png", "jpg"), Some(Compression::Strong));
        assert!(Options::from_json(r#"{"compression": "extreme"}"#).is_err());
    }
}
