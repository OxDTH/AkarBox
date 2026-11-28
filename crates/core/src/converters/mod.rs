//! Built-in converters. Each one wraps a well-known open-source tool.
//!
//! **Adding a converter:** create a new file here, implement [`Converter`] for a
//! struct, and add it to [`builtin`]. That's it: the planner will automatically
//! combine your edges with everyone else's. See `docs/ADDING_A_CONVERTER.md`.

mod ffmpeg;
mod ghostscript;
mod heif;
mod imagemagick;
mod libreoffice;
mod pandoc;
mod raw;

pub use ffmpeg::{Ffmpeg, HwEncoder};
pub use ghostscript::Ghostscript;
pub use heif::Heif;
pub use imagemagick::ImageMagick;
pub use libreoffice::LibreOffice;
pub use pandoc::Pandoc;
pub use raw::Raw;

use crate::Converter;

pub fn builtin() -> Vec<Box<dyn Converter>> {
    vec![
        Box::new(Ffmpeg::detect()),
        Box::new(Heif::detect()),
        Box::new(Raw::detect()),
        Box::new(ImageMagick::detect()),
        Box::new(Ghostscript),
        Box::new(LibreOffice::detect()),
        Box::new(Pandoc),
    ]
}
