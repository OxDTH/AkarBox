//! Built-in converters. Each one wraps a well-known open-source tool.
//!
//! **Adding a converter:** create a new file here, implement [`Converter`] for a
//! struct, and add it to [`builtin`]. That's it: the planner will automatically
//! combine your edges with everyone else's. See `docs/ADDING_A_CONVERTER.md`.

mod ffmpeg;
mod heif;
mod imagemagick;
mod libreoffice;
mod pandoc;

pub use ffmpeg::Ffmpeg;
pub use heif::Heif;
pub use imagemagick::ImageMagick;
pub use libreoffice::LibreOffice;
pub use pandoc::Pandoc;

use crate::Converter;

pub fn builtin() -> Vec<Box<dyn Converter>> {
    vec![
        Box::new(Ffmpeg),
        Box::new(Heif::detect()),
        Box::new(ImageMagick::detect()),
        Box::new(LibreOffice::detect()),
        Box::new(Pandoc),
    ]
}
