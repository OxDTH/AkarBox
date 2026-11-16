use crate::converter::{on_path, run_command, Converter, Edge, Step};
use crate::Result;
use std::process::Command;

/// Markup documents via Pandoc (https://pandoc.org).
pub struct Pandoc;

/// Map our extension to pandoc's format name. Returns `None` if pandoc can't read it.
fn reader(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "md" => "markdown",
        "html" => "html",
        "docx" => "docx",
        "odt" => "odt",
        "epub" => "epub",
        "rst" => "rst",
        "tex" => "latex",
        _ => return None,
    })
}

fn writer(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "txt" => "plain",
        "rtf" => "rtf",
        other => return reader(other),
    })
}

const INPUTS: &[&str] = &["md", "html", "docx", "odt", "epub", "rst", "tex"];
const OUTPUTS: &[&str] = &[
    "md", "html", "docx", "odt", "epub", "rst", "tex", "txt", "rtf",
];

impl Converter for Pandoc {
    fn id(&self) -> &'static str {
        "pandoc"
    }

    fn is_available(&self) -> bool {
        on_path("pandoc")
    }

    fn edges(&self) -> Vec<Edge> {
        crate::converter::all_pairs(INPUTS, OUTPUTS, 1)
    }

    fn convert(&self, step: &Step) -> Result<()> {
        let mut cmd = Command::new("pandoc");
        cmd.args(["--standalone", "-f"])
            .arg(reader(step.from).unwrap_or(step.from))
            .arg("-t")
            .arg(writer(step.to).unwrap_or(step.to))
            // Images inside .docx/.epub are extracted here instead of next to the input.
            .arg(format!("--extract-media={}", step.work_dir.display()))
            .arg("-o")
            .arg(step.output)
            .arg(step.input);
        run_command(&mut cmd)
    }
}
