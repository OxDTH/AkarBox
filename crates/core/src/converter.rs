//! The `Converter` trait: the plug-in interface every converter implements.

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// One conversion a converter can do, e.g. `png -> jpg`.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: &'static str,
    pub to: &'static str,
    /// Lower is better. The planner adds costs along a path and picks the cheapest,
    /// so use a higher cost for slow or lossy conversions.
    pub cost: u32,
}

/// Everything a converter needs to perform one step.
#[derive(Debug)]
pub struct Step<'a> {
    pub input: &'a Path,
    pub output: &'a Path,
    pub from: &'a str,
    pub to: &'a str,
    /// A private scratch directory for this step (deleted afterwards).
    pub work_dir: &'a Path,
}

/// A trait is like an interface: a set of methods a type promises to provide.
///
/// `Send + Sync` means a converter can be shared between threads, which the server
/// needs because many requests run at the same time.
///
/// Conversions are plain blocking functions (they mostly wait on an external
/// program). The server runs them on a background thread pool so they don't block
/// the async web server.
pub trait Converter: Send + Sync {
    /// Short, stable id such as `"ffmpeg"`. Shown in the UI.
    fn id(&self) -> &'static str;

    /// Is the backing tool installed? Unavailable converters are skipped, so AkarBox
    /// still works (with fewer formats) on a machine without, say, LibreOffice.
    fn is_available(&self) -> bool;

    /// All conversions this converter supports.
    fn edges(&self) -> Vec<Edge>;

    /// Convert `step.input` into `step.output`.
    fn convert(&self, step: &Step) -> Result<()>;
}

/// Helper: build every `from -> to` edge between two lists, skipping `x -> x`.
pub(crate) fn all_pairs(from: &[&'static str], to: &[&'static str], cost: u32) -> Vec<Edge> {
    // Iterators are lazy pipelines; `collect()` runs them and builds the Vec.
    from.iter()
        .flat_map(|&f| to.iter().map(move |&t| (f, t)))
        .filter(|(f, t)| f != t)
        .map(|(from, to)| Edge { from, to, cost })
        .collect()
}

/// Run an external program and turn a non-zero exit code into an [`Error`].
pub fn run_command(cmd: &mut Command) -> Result<()> {
    let program = cmd.get_program().to_string_lossy().into_owned();
    tracing::debug!(?cmd, "running");
    let out = cmd.output()?; // `?` converts io::Error -> Error::Io via `#[from]`
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Keep the message short: only the last few lines are usually useful.
        let tail: Vec<&str> = stderr.lines().rev().take(8).collect();
        let stderr = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
        Err(Error::CommandFailed { program, stderr })
    }
}

/// Returns true if `program` can be found on `PATH`.
pub(crate) fn on_path(program: &str) -> bool {
    find_on_path(program).is_some()
}

pub(crate) fn find_on_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|p| p.is_file())
}
