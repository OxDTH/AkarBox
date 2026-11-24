use crate::converter::{
    all_pairs, find_on_path, run_command, run_command_output, Converter, Edge, Produced, Step,
};
use crate::{Error, Result};
use std::path::PathBuf;
use std::process::Command;

/// HEIC/HEIF photos (the iPhone default) via libheif (https://github.com/strukturag/libheif).
///
/// libheif only decodes to lossless PNG here; the planner chains ImageMagick
/// after it for every other format (`heic -> png -> jpg`). That way the user's
/// options (quality, max size) are always applied by ImageMagick in the last step.
pub struct Heif {
    /// Newer libheif ships `heif-dec`; older versions call it `heif-convert`.
    program: Option<PathBuf>,
}

impl Heif {
    pub fn detect() -> Self {
        let program = ["heif-dec", "heif-convert"]
            .into_iter()
            .find_map(find_on_path)
            .filter(|p| has_heic_decoder(p));
        Self { program }
    }
}

/// Newer libheif loads codecs as plugins, so the tool can be installed while
/// HEIC decoding is not (on Ubuntu the plugin is `libheif-plugin-libde265`).
fn has_heic_decoder(program: &std::path::Path) -> bool {
    let Ok(list) = run_command_output(Command::new(program).arg("--list-decoders")) else {
        return true; // old versions have no `--list-decoders`, and decode HEIC built-in
    };
    // The output looks like:
    //   HEIC decoders:
    //   - libde265 = libde265 HEVC decoder, version 1.0.15
    //   AVIF decoders:
    let mut lines = list.lines().skip_while(|l| !l.starts_with("HEIC decoders"));
    lines
        .nth(1)
        .is_some_and(|l| l.trim_start().starts_with('-'))
}

impl Converter for Heif {
    fn id(&self) -> &'static str {
        "libheif"
    }

    fn is_available(&self) -> bool {
        self.program.is_some()
    }

    fn edges(&self) -> Vec<Edge> {
        all_pairs(&["heic", "heif"], &["png"], 1)
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        // libheif decides the output format from the file name, and when a file
        // holds several images (bursts, live photos) it writes `out-1.jpg`,
        // `out-2.jpg`, ... So we write into our scratch folder and pick the result.
        let out = step.work_dir.join(format!("out.{}", step.to));
        let mut cmd = Command::new(self.program.as_ref().expect("checked by is_available"));
        cmd.arg(step.input).arg(&out);
        run_command(&mut cmd)?;

        // libheif applies the photo's rotation/mirroring while decoding, so the
        // result is already upright.
        let produced = if out.is_file() {
            out
        } else {
            // Several images: take the first one (the primary photo).
            let mut candidates: Vec<PathBuf> = std::fs::read_dir(step.work_dir)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|ext| ext == step.to))
                .collect();
            candidates.sort();
            candidates
                .into_iter()
                .next()
                .ok_or_else(|| Error::MissingOutput(step.output.into()))?
        };
        std::fs::rename(produced, step.output)?;
        Ok(Produced::File)
    }
}
