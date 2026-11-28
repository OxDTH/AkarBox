use crate::converter::{all_pairs, find_on_path, run_command, Converter, Edge, Produced, Step};
use crate::Result;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Camera RAW formats: Canon, Nikon, Sony, Adobe DNG, Fujifilm, Panasonic, Olympus.
pub const RAW: &[&str] = &["cr2", "cr3", "nef", "arw", "dng", "raf", "rw2", "orf"];

/// Camera RAW photos via LibRaw's `dcraw_emu` (https://www.libraw.org), or the
/// classic `dcraw` as a fallback.
///
/// The camera's sensor data is "developed" into an ordinary 8-bit TIFF (using the
/// camera's white balance and rotation); the planner then chains ImageMagick for
/// JPG, PNG, WebP and friends.
pub struct Raw {
    tool: Option<Tool>,
}

/// The two tools we know how to drive. They take slightly different arguments.
enum Tool {
    DcrawEmu(PathBuf),
    Dcraw(PathBuf),
}

impl Raw {
    pub fn detect() -> Self {
        // LibRaw supports newer cameras (e.g. Canon CR3), so prefer it.
        let tool = find_on_path("dcraw_emu")
            .map(Tool::DcrawEmu)
            .or_else(|| find_on_path("dcraw").map(Tool::Dcraw));
        Self { tool }
    }
}

impl Converter for Raw {
    fn id(&self) -> &'static str {
        "libraw"
    }

    fn is_available(&self) -> bool {
        self.tool.is_some()
    }

    fn edges(&self) -> Vec<Edge> {
        // Cost 2: developing a RAW file is much heavier than decoding a JPEG.
        all_pairs(RAW, &["tiff"], 2)
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        // -w: use the camera's white balance, -T: write TIFF.
        match self.tool.as_ref().expect("checked by is_available") {
            Tool::DcrawEmu(program) => {
                let mut cmd = Command::new(program);
                cmd.args(["-w", "-T", "-Z"])
                    .arg(step.output)
                    .arg(step.input);
                run_command(&mut cmd)?;
            }
            Tool::Dcraw(program) => {
                // dcraw writes to stdout with -c, so we point stdout at the output file.
                let out = std::fs::File::create(step.output)?;
                let mut cmd = Command::new(program);
                cmd.args(["-c", "-w", "-T"])
                    .arg(step.input)
                    .stdout(Stdio::from(out));
                run_command(&mut cmd)?;
            }
        }
        Ok(Produced::File)
    }
}
