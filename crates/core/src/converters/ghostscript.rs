use crate::converter::{
    file_size, on_path, run_command, same_format, Converter, Edge, Produced, Step,
};
use crate::options::Compression;
use crate::{Error, Result};
use std::path::Path;
use std::process::Command;

/// Compresses PDFs with Ghostscript's `pdfwrite` (https://ghostscript.com).
///
/// Ghostscript rewrites the PDF: it downsamples big images, recompresses them
/// and drops duplicates. Text stays text, so it's still searchable.
pub struct Ghostscript;

/// Compression profiles from mildest to strongest.
///
/// An array of tuples: (name used in logs, extra Ghostscript arguments).
const PROFILES: &[(&str, &[&str])] = &[
    ("printer", &["-dPDFSETTINGS=/printer"]), // ~300 DPI images
    ("ebook", &["-dPDFSETTINGS=/ebook"]),     // ~150 DPI
    ("screen", &["-dPDFSETTINGS=/screen"]),   // ~72 DPI
    (
        "extreme",
        &[
            "-dPDFSETTINGS=/screen",
            "-dDownsampleColorImages=true",
            "-dColorImageResolution=50",
            "-dDownsampleGrayImages=true",
            "-dGrayImageResolution=50",
            "-dDownsampleMonoImages=true",
            "-dMonoImageResolution=150",
            "-dColorImageDownsampleThreshold=1.0",
            "-dGrayImageDownsampleThreshold=1.0",
        ],
    ),
];

/// Which profile each user-facing level maps to.
fn profile_index(level: Compression) -> usize {
    match level {
        Compression::Light => 0,
        Compression::Balanced => 1,
        Compression::Strong => 2,
    }
}

fn compress(input: &Path, output: &Path, profile: usize) -> Result<()> {
    let (name, args) = PROFILES[profile];
    tracing::debug!(profile = name, "compressing PDF");
    let mut cmd = Command::new("gs");
    cmd.args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite"])
        .args([
            "-dCompatibilityLevel=1.5",
            "-dDetectDuplicateImages=true",
            "-dCompressFonts=true",
        ])
        .args(args)
        .arg(format!("-sOutputFile={}", output.display()))
        .arg(input);
    run_command(&mut cmd)
}

impl Converter for Ghostscript {
    fn id(&self) -> &'static str {
        "ghostscript"
    }

    fn is_available(&self) -> bool {
        on_path("gs")
    }

    fn edges(&self) -> Vec<Edge> {
        same_format(&["pdf"], 1)
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        let original = file_size(step.input)?;
        let level = step
            .options
            .compression_for(step.from, step.to)
            .unwrap_or(Compression::Balanced);

        match step.options.target_bytes() {
            // Already small enough: nothing to do.
            Some(limit) if original <= limit => {
                std::fs::copy(step.input, step.output)?;
            }
            // Try the profiles from mildest to strongest and keep the first that
            // fits, which is the best-looking result under the limit.
            Some(limit) => {
                let mut smallest = original;
                let mut fitted = false;
                for profile in 0..PROFILES.len() {
                    let attempt = step.work_dir.join(format!("attempt-{profile}.pdf"));
                    compress(step.input, &attempt, profile)?;
                    let size = file_size(&attempt)?;
                    smallest = smallest.min(size);
                    if size <= limit {
                        std::fs::rename(&attempt, step.output)?;
                        fitted = true;
                        break;
                    }
                }
                if !fitted {
                    return Err(Error::Unsupported(format!(
                        "couldn't get this PDF under {} KB; the smallest we managed was {} KB",
                        limit / 1000,
                        smallest.div_ceil(1000)
                    )));
                }
            }
            None => {
                compress(step.input, step.output, profile_index(level))?;
                // A PDF that was already optimised can grow; keep the original then.
                if file_size(step.output)? >= original {
                    std::fs::copy(step.input, step.output)?;
                }
            }
        }
        Ok(Produced::File)
    }
}
