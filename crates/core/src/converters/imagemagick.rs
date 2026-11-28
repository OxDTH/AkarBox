use crate::converter::{
    all_pairs, file_size, on_path, run_command, same_format, Converter, Edge, Produced, Step,
};
use crate::options::Compression;
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const RASTER: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "avif"];
/// Formats where a "quality" number trades size for looks.
const LOSSY: &[&str] = &["jpg", "webp", "avif"];
/// Formats we offer to compress in place (`png -> png`).
const COMPRESSIBLE: &[&str] = &["png", "jpg", "webp", "avif"];

/// Images (and PDF <-> image) via ImageMagick (https://imagemagick.org).
pub struct ImageMagick {
    /// ImageMagick 7 ships `magick`; version 6 (common on Debian/Ubuntu) ships `convert`.
    /// `Option` because neither may be installed.
    program: Option<&'static str>,
    /// Ghostscript renders PDF pages for us.
    ghostscript: bool,
    /// ImageMagick hands SVG files to `rsvg-convert` (from librsvg).
    rsvg: bool,
}

/// Extra knobs used while searching for a small enough file. They are set by
/// [`ImageMagick::fit_to_size`] on top of the user's options.
#[derive(Debug, Clone, Copy, Default)]
struct Tuning {
    quality: Option<u8>,
    /// Scale to this percentage of the (already size-limited) image.
    scale_percent: Option<f64>,
    /// Reduce to at most this many colours (helps PNG/GIF a lot).
    colors: Option<u32>,
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

    fn command(&self) -> Command {
        // `.expect` is fine here: we are only called when `is_available()` was true.
        Command::new(self.program.expect("checked by is_available"))
    }

    /// Convert one image file to another, applying the user's options plus `tuning`.
    fn convert_image(
        &self,
        step: &Step,
        input: &Path,
        output: &Path,
        tuning: Tuning,
    ) -> Result<()> {
        let opts = step.options;
        let compression = opts.compression_for(step.from, step.to);
        let mut cmd = self.command();

        // `[0]` = first frame only, so a multi-frame input (animated GIF, multi-page
        // TIFF, ICO with several sizes) doesn't turn into many output files.
        let keep_frames = step.to == "gif" && step.from == "gif";
        let frame = if keep_frames { "" } else { "[0]" };
        cmd.arg(format!("{}{frame}", input.display()));
        cmd.arg("-auto-orient"); // respect the EXIF rotation from phone cameras

        if let Some(geometry) = opts.resize_geometry() {
            cmd.args(["-resize", &geometry]);
        }
        if let Some(pct) = tuning.scale_percent {
            cmd.args(["-resize", &format!("{pct:.2}%")]);
        }
        if step.to == "ico" {
            cmd.args(["-resize", "256x256>"]); // icons max out at 256px
        }
        if step.to == "jpg" {
            cmd.args(["-background", "white", "-flatten"]); // JPEG has no transparency
        }
        if compression.is_some() || opts.target_size_kb.is_some() {
            // Drop metadata (camera info, thumbnails, colour profiles' comments).
            cmd.arg("-strip");
            if step.to == "jpg" {
                cmd.args(["-sampling-factor", "4:2:0", "-interlace", "JPEG"]);
            }
            if step.to == "png" {
                cmd.args(["-define", "png:compression-level=9"]);
            }
        }
        if let Some(colors) = tuning.colors {
            cmd.args(["-colors", &colors.to_string()]);
        }

        // Explicit quality wins, then the search's quality, then the compression preset.
        let preset = compression.and_then(|level| preset_quality(step.to, level));
        if let Some(q) = tuning.quality.or(opts.quality).or(preset) {
            cmd.args(["-quality", &q.to_string()]);
        }

        // Prefixing the output with the format (`png:out`) avoids surprises with
        // odd file names.
        cmd.arg(format!("{}:{}", step.to, output.display()));
        run_command(&mut cmd)
    }

    /// Convert, then keep shrinking until the file is at most `limit` bytes.
    ///
    /// Strategy: for lossy formats, find the highest quality that fits (binary
    /// search). If even low quality is too big, scale the picture down and try
    /// again. Lossless formats can only shrink by reducing colours and size.
    fn fit_to_size(&self, step: &Step, input: &Path, output: &Path, limit: u64) -> Result<()> {
        self.convert_image(step, input, output, Tuning::default())?;
        if file_size(output)? <= limit {
            return Ok(());
        }

        let lossy = LOSSY.contains(&step.to);
        let attempt = step.work_dir.join(format!("attempt.{}", step.to));
        let mut scale = 100.0_f64;
        // A closure that tries one setting and reports the resulting size.
        let try_with = |tuning: Tuning| -> Result<u64> {
            self.convert_image(step, input, &attempt, tuning)?;
            file_size(&attempt)
        };
        let keep = |from: &Path| std::fs::copy(from, output).map(|_| ());

        for _round in 0..12 {
            let base = Tuning {
                scale_percent: (scale < 100.0).then_some(scale),
                colors: (!lossy && step.to != "jpg").then_some(256),
                ..Tuning::default()
            };

            let smallest = if lossy {
                // Lowest quality we're willing to use before shrinking the picture.
                const MIN_QUALITY: u8 = 35;
                let top = step.options.quality.unwrap_or(92);
                let (mut low, mut high) = (MIN_QUALITY, top.max(MIN_QUALITY));
                let mut best: Option<u8> = None;
                while low <= high {
                    let mid = low + (high - low) / 2;
                    if try_with(Tuning {
                        quality: Some(mid),
                        ..base
                    })? <= limit
                    {
                        best = Some(mid);
                        low = mid + 1;
                    } else {
                        high = mid - 1; // can't underflow: mid >= MIN_QUALITY
                    }
                }
                if let Some(q) = best {
                    try_with(Tuning {
                        quality: Some(q),
                        ..base
                    })?;
                    return keep(&attempt).map_err(Error::from);
                }
                try_with(Tuning {
                    quality: Some(MIN_QUALITY),
                    ..base
                })?
            } else {
                let size = try_with(base)?;
                if size <= limit {
                    return keep(&attempt).map_err(Error::from);
                }
                size
            };

            // Area scales with the square of the side, so shrink by sqrt(ratio),
            // a bit extra to avoid many near-misses.
            let ratio = (limit as f64 / smallest as f64).sqrt() * 0.92;
            scale *= ratio.clamp(0.1, 0.9);
            if scale < 1.0 {
                break;
            }
        }
        Err(Error::Unsupported(format!(
            "couldn't make this image smaller than {} KB",
            limit / 1000
        )))
    }

    /// Render every page of a PDF with Ghostscript, then convert each page.
    fn convert_pdf(&self, step: &Step) -> Result<Produced> {
        let dpi = step.options.dpi.unwrap_or(150);
        let rendered = step.work_dir.join("rendered");
        std::fs::create_dir_all(&rendered)?;

        // Ghostscript streams page by page, so even a 500-page PDF won't need
        // much memory. `%03d` becomes 001, 002, ...
        let mut gs = Command::new("gs");
        gs.args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=png16m"])
            .args(["-dTextAlphaBits=4", "-dGraphicsAlphaBits=4"])
            .arg(format!("-r{dpi}"))
            .arg(format!("-sOutputFile={}/page-%03d.png", rendered.display()))
            .arg(step.input);
        run_command(&mut gs)?;

        let mut pages: Vec<PathBuf> = std::fs::read_dir(&rendered)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .collect();
        pages.sort(); // zero-padded names sort in page order

        // Turn each rendered PNG into the requested format.
        let mut outputs = Vec::with_capacity(pages.len());
        for page in &pages {
            let stem = page.file_stem().and_then(|s| s.to_str()).unwrap_or("page");
            let out = step.work_dir.join(format!("{stem}.{}", step.to));
            self.convert_image(step, page, &out, Tuning::default())?;
            outputs.push(out);
        }
        Ok(Produced::Pages(outputs))
    }
}

/// Quality used for each compression level, for formats that have one.
fn preset_quality(format: &str, level: Compression) -> Option<u8> {
    if !LOSSY.contains(&format) {
        return None;
    }
    Some(match level {
        Compression::Light => 85,
        Compression::Balanced => 75,
        Compression::Strong => 55,
    })
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
        edges.extend(same_format(COMPRESSIBLE, 1));
        if self.rsvg {
            edges.extend(all_pairs(&["svg"], RASTER, 1));
        }
        edges.extend(all_pairs(RASTER, &["pdf"], 1));
        if self.ghostscript {
            edges.extend(all_pairs(&["pdf"], RASTER, 1));
        }
        edges
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        if step.from == "pdf" {
            return self.convert_pdf(step);
        }
        match step.options.target_bytes() {
            Some(limit) if step.to != "pdf" => {
                self.fit_to_size(step, step.input, step.output, limit)?
            }
            _ => self.convert_image(step, step.input, step.output, Tuning::default())?,
        }
        // Compressing an already well-optimised file can make it *bigger*. If so,
        // hand back the original.
        if step.from == step.to && file_size(step.output)? >= file_size(step.input)? {
            std::fs::copy(step.input, step.output)?;
        }
        Ok(Produced::File)
    }
}
