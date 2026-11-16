use crate::converter::{all_pairs, on_path, run_command, Converter, Edge, Step};
use crate::Result;
use std::process::Command;

const RASTER: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "avif"];

/// Images (and PDF <-> image) via ImageMagick (https://imagemagick.org).
pub struct ImageMagick {
    /// ImageMagick 7 ships `magick`; version 6 (common on Debian/Ubuntu) ships `convert`.
    /// `Option` because neither may be installed.
    program: Option<&'static str>,
    /// Ghostscript is needed to read PDFs.
    ghostscript: bool,
    /// ImageMagick hands SVG files to `rsvg-convert` (from librsvg).
    rsvg: bool,
}

impl ImageMagick {
    pub fn detect() -> Self {
        let program = ["magick", "convert"].into_iter().find(|p| on_path(p));
        Self {
            program,
            ghostscript: on_path("gs"),
            rsvg: on_path("rsvg-convert"),
        }
    }
}

impl Converter for ImageMagick {
    fn id(&self) -> &'static str {
        "imagemagick"
    }

    fn is_available(&self) -> bool {
        self.program.is_some()
    }

    fn edges(&self) -> Vec<Edge> {
        let mut edges = all_pairs(RASTER, RASTER, 1);
        if self.rsvg {
            edges.extend(all_pairs(&["svg"], RASTER, 1));
        }
        edges.extend(all_pairs(RASTER, &["pdf"], 1));
        if self.ghostscript {
            edges.extend(all_pairs(&["pdf"], RASTER, 1));
        }
        edges
    }

    fn convert(&self, step: &Step) -> Result<()> {
        // `.expect` is fine here: `convert` is only called when `is_available()` was true.
        let mut cmd = Command::new(self.program.expect("checked by is_available"));

        if step.from == "pdf" {
            // Render at 150 DPI and take the first page only.
            // TODO(good first issue): convert every page and return a .zip.
            cmd.args(["-density", "150"]);
            cmd.arg(format!("{}[0]", step.input.display()));
            cmd.args(["-background", "white", "-alpha", "remove"]);
        } else {
            // `[0]` = first frame, so animated GIFs don't explode into many files.
            let frame = if step.to == "gif" && step.from == "gif" {
                ""
            } else {
                "[0]"
            };
            cmd.arg(format!("{}{frame}", step.input.display()));
        }

        if step.to == "ico" {
            cmd.args(["-resize", "256x256>"]); // icons max out at 256px
        }
        if step.to == "jpg" {
            cmd.args(["-background", "white", "-flatten"]); // JPEG has no transparency
        }

        // Prefixing the output with the format (`png:out`) avoids surprises with
        // odd file names.
        cmd.arg(format!("{}:{}", step.to, step.output.display()));
        run_command(&mut cmd)
    }
}
