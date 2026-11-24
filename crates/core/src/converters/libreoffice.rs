use crate::converter::{all_pairs, find_on_path, run_command, Converter, Edge, Produced, Step};
use crate::{Error, Result};
use std::path::PathBuf;
use std::process::Command;

const TEXT: &[&str] = &["docx", "doc", "odt", "rtf"];
const SHEET: &[&str] = &["xlsx", "xls", "ods", "csv"];
const SLIDES: &[&str] = &["pptx", "ppt", "odp"];

/// Office documents via LibreOffice in headless mode (https://libreoffice.org).
pub struct LibreOffice {
    program: Option<PathBuf>,
}

impl LibreOffice {
    pub fn detect() -> Self {
        let program = ["soffice", "libreoffice"]
            .into_iter()
            .find_map(find_on_path);
        Self { program }
    }
}

impl Converter for LibreOffice {
    fn id(&self) -> &'static str {
        "libreoffice"
    }

    fn is_available(&self) -> bool {
        self.program.is_some()
    }

    fn edges(&self) -> Vec<Edge> {
        // LibreOffice is slow to start, so give it a higher cost: the planner will
        // prefer a faster tool (e.g. pandoc) when one can do the same job.
        let mut edges = all_pairs(TEXT, TEXT, 3);
        edges.extend(all_pairs(TEXT, &["pdf", "txt", "html"], 3));
        edges.extend(all_pairs(&["txt", "html"], &["docx", "odt"], 4));
        edges.extend(all_pairs(SHEET, SHEET, 3));
        edges.extend(all_pairs(SHEET, &["pdf", "html"], 3));
        edges.extend(all_pairs(SLIDES, SLIDES, 3));
        edges.extend(all_pairs(SLIDES, &["pdf"], 3));
        edges
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        let program = self.program.as_ref().expect("checked by is_available");

        // Each run gets its own LibreOffice profile; otherwise two conversions at
        // the same time fight over the user's profile and one of them fails.
        let profile = step.work_dir.join("lo-profile");
        let mut cmd = Command::new(program);
        cmd.arg(format!(
            "-env:UserInstallation=file://{}",
            profile.display()
        ))
        .args([
            "--headless",
            "--norestore",
            "--convert-to",
            step.to,
            "--outdir",
        ])
        .arg(step.work_dir)
        .arg(step.input);
        run_command(&mut cmd)?;

        // LibreOffice names the result after the input file: `step0.docx` -> `step0.pdf`.
        let stem = step.input.file_stem().unwrap_or_default();
        let produced = step.work_dir.join(stem).with_extension(step.to);
        if !produced.is_file() {
            return Err(Error::MissingOutput(produced));
        }
        std::fs::rename(produced, step.output)?;
        Ok(Produced::File)
    }
}
