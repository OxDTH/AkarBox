use crate::converter::{all_pairs, on_path, run_command, Converter, Edge, Produced, Step};
use crate::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

const RASTER: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "avif"];

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

    /// Convert one image file to another, applying the user's options.
    fn convert_image(&self, step: &Step, input: &Path, output: &Path) -> Result<()> {
        let mut cmd = self.command();

        // `[0]` = first frame only, so a multi-frame input (animated GIF, multi-page
        // TIFF, ICO with several sizes) doesn't turn into many output files.
        let keep_frames = step.to == "gif" && step.from == "gif";
        let frame = if keep_frames { "" } else { "[0]" };
        cmd.arg(format!("{}{frame}", input.display()));
        cmd.arg("-auto-orient"); // respect the EXIF rotation from phone cameras

        if let Some(geometry) = step.options.resize_geometry() {
            cmd.args(["-resize", &geometry]);
        }
        if step.to == "ico" {
            cmd.args(["-resize", "256x256>"]); // icons max out at 256px
        }
        if step.to == "jpg" {
            cmd.args(["-background", "white", "-flatten"]); // JPEG has no transparency
        }
        if let Some(q) = step.options.quality {
            cmd.args(["-quality", &q.to_string()]);
        }

        // Prefixing the output with the format (`png:out`) avoids surprises with
        // odd file names.
        cmd.arg(format!("{}:{}", step.to, output.display()));
        run_command(&mut cmd)
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
            self.convert_image(step, page, &out)?;
            outputs.push(out);
        }
        Ok(Produced::Pages(outputs))
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

    fn convert(&self, step: &Step) -> Result<Produced> {
        if step.from == "pdf" {
            return self.convert_pdf(step);
        }
        self.convert_image(step, step.input, step.output)?;
        Ok(Produced::File)
    }
}
