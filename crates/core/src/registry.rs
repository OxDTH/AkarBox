//! The registry holds every available converter and plans multi-step conversions.

use crate::converter::{Converter, Produced, Step};
use crate::format::{normalize_ext, Format};
use crate::{Error, Options, Result};
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};

/// Longest chain we will consider. Long chains are slow and lose quality.
const MAX_STEPS: usize = 3;

/// One step of a plan: "use converter X to go from A to B".
// `Ord` is needed so steps can sit inside the priority queue below.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlannedStep {
    pub converter: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub steps: Vec<PlannedStep>,
    pub cost: u32,
}

/// The final result of running a plan.
#[derive(Debug)]
pub struct Output {
    pub path: PathBuf,
    /// Extension of the result. Usually the requested format, but `"zip"` when a
    /// step produced several files (e.g. one image per PDF page).
    pub ext: String,
}

/// A format you can reach from some input, plus how to get there.
#[derive(Debug, Clone, Serialize)]
pub struct Target {
    pub format: &'static Format,
    pub plan: Plan,
}

pub struct Registry {
    /// `Box<dyn Converter>` = "some type that implements Converter, stored on the heap".
    /// This lets one Vec hold different converter types.
    converters: Vec<Box<dyn Converter>>,
    /// Adjacency list: format -> every direct step out of it.
    graph: HashMap<&'static str, Vec<(PlannedStep, u32)>>,
}

impl Registry {
    /// A registry with all built-in converters whose tools are installed.
    pub fn with_builtin() -> Self {
        Self::new(crate::converters::builtin())
    }

    pub fn new(all: Vec<Box<dyn Converter>>) -> Self {
        let mut converters = Vec::new();
        for c in all {
            if c.is_available() {
                tracing::info!(converter = c.id(), "converter available");
                converters.push(c);
            } else {
                tracing::warn!(
                    converter = c.id(),
                    "converter unavailable (tool not installed)"
                );
            }
        }

        let mut graph: HashMap<&'static str, Vec<(PlannedStep, u32)>> = HashMap::new();
        for c in &converters {
            for e in c.edges() {
                // `entry(..).or_default()` = "get the Vec for this key, creating it if missing".
                graph.entry(e.from).or_default().push((
                    PlannedStep {
                        converter: c.id(),
                        from: e.from,
                        to: e.to,
                    },
                    e.cost,
                ));
            }
        }
        Self { converters, graph }
    }

    /// Ids of the converters that are actually usable.
    pub fn converter_ids(&self) -> Vec<&'static str> {
        self.converters.iter().map(|c| c.id()).collect()
    }

    /// Every format reachable from `from`, each with its cheapest plan.
    ///
    /// This is Dijkstra's shortest-path algorithm over the format graph.
    pub fn targets(&self, from: &str) -> Vec<Target> {
        let Some(start) = Format::get(from) else {
            return Vec::new();
        };

        // best[format] = cheapest plan found so far to reach `format`.
        let mut best: HashMap<&'static str, Plan> = HashMap::new();
        // A max-heap; wrapping in `Reverse` turns it into a min-heap by cost.
        let mut queue = BinaryHeap::new();
        queue.push(Reverse((0u32, start.ext, Vec::<PlannedStep>::new())));

        while let Some(Reverse((cost, fmt, steps))) = queue.pop() {
            if fmt != start.ext {
                if best.get(fmt).is_some_and(|p| p.cost <= cost) {
                    continue; // already found something at least as good
                }
                best.insert(
                    fmt,
                    Plan {
                        steps: steps.clone(),
                        cost,
                    },
                );
            }
            if steps.len() == MAX_STEPS {
                continue;
            }
            for (step, edge_cost) in self.graph.get(fmt).into_iter().flatten() {
                if step.to == start.ext {
                    continue;
                }
                let mut next = steps.clone();
                next.push(step.clone());
                queue.push(Reverse((cost + edge_cost, step.to, next)));
            }
        }

        let mut targets: Vec<Target> = best
            .into_iter()
            .filter_map(|(ext, plan)| Format::get(ext).map(|format| Target { format, plan }))
            .collect();
        targets.sort_by_key(|t| (t.format.category as u8, t.format.ext));
        targets
    }

    /// The cheapest plan for `from -> to`.
    pub fn plan(&self, from: &str, to: &str) -> Result<Plan> {
        let to = normalize_ext(to);
        self.targets(from)
            .into_iter()
            .find(|t| t.format.ext == to)
            .map(|t| t.plan)
            .ok_or_else(|| Error::NoPath {
                from: from.to_string(),
                to,
            })
    }

    /// Run a plan. Intermediate files are written to `work_dir`.
    ///
    /// `options` are applied to the last step only (see [`Step::options`]).
    pub fn execute(
        &self,
        plan: &Plan,
        input: &Path,
        work_dir: &Path,
        options: &Options,
    ) -> Result<Output> {
        let defaults = Options::default();
        let mut current = input.to_path_buf();

        for (i, planned) in plan.steps.iter().enumerate() {
            let is_last = i + 1 == plan.steps.len();
            let converter = self
                .converters
                .iter()
                .find(|c| c.id() == planned.converter)
                .expect("plan only references registered converters");

            let step_dir = work_dir.join(format!("step{i}"));
            std::fs::create_dir_all(&step_dir)?;
            let output = work_dir.join(format!("step{i}.{}", planned.to));

            tracing::info!(
                converter = planned.converter,
                from = planned.from,
                to = planned.to,
                "converting"
            );
            let produced = converter.convert(&Step {
                input: &current,
                output: &output,
                from: planned.from,
                to: planned.to,
                work_dir: &step_dir,
                options: if is_last { options } else { &defaults },
            })?;

            match produced {
                Produced::File => {}
                // One page is just a normal file.
                Produced::Pages(pages) if pages.len() == 1 => {
                    std::fs::rename(&pages[0], &output)?;
                }
                Produced::Pages(pages) if is_last => {
                    let zip_path = work_dir.join("result.zip");
                    zip_files(&pages, &zip_path)?;
                    return Ok(Output {
                        path: zip_path,
                        ext: "zip".into(),
                    });
                }
                Produced::Pages(_) => {
                    return Err(Error::Unsupported(format!(
                        "{} produced several files, which can't be converted further",
                        planned.converter
                    )));
                }
            }

            if !output.is_file() {
                return Err(Error::MissingOutput(output));
            }
            current = output;
        }
        let ext = plan.steps.last().map_or("", |s| s.to).to_string();
        Ok(Output { path: current, ext })
    }

    /// Convenience: plan + execute with default options, using a temporary
    /// directory for intermediates. The result is copied to `output`; the
    /// returned string is the result's extension (see [`Output::ext`]).
    pub fn convert(&self, input: &Path, from: &str, to: &str, output: &Path) -> Result<String> {
        let plan = self.plan(from, to)?;
        let tmp = tempfile::tempdir()?; // deleted automatically when `tmp` goes out of scope
        let result = self.execute(&plan, input, tmp.path(), &Options::default())?;
        std::fs::copy(&result.path, output)?;
        Ok(result.ext)
    }
}

/// Put `files` into a new zip archive at `dest`, keeping their file names.
pub fn zip_files(files: &[PathBuf], dest: &Path) -> Result<()> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::fs::File::create(dest)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for file in files {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        zip.start_file(name, options)?;
        zip.write_all(&std::fs::read(file)?)?;
    }
    zip.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converter::Edge;

    /// A fake converter for testing the planner without real tools.
    struct Fake(&'static str, Vec<Edge>);

    impl Converter for Fake {
        fn id(&self) -> &'static str {
            self.0
        }
        fn is_available(&self) -> bool {
            true
        }
        fn edges(&self) -> Vec<Edge> {
            self.1.clone()
        }
        fn convert(&self, step: &Step) -> Result<Produced> {
            // "split" pretends every input has 3 pages.
            if self.0 == "split" {
                let pages: Vec<PathBuf> = (1..=3)
                    .map(|n| step.work_dir.join(format!("page-{n}.{}", step.to)))
                    .collect();
                for p in &pages {
                    std::fs::copy(step.input, p)?;
                }
                return Ok(Produced::Pages(pages));
            }
            std::fs::copy(step.input, step.output)?;
            Ok(Produced::File)
        }
    }

    fn e(from: &'static str, to: &'static str, cost: u32) -> Edge {
        Edge { from, to, cost }
    }

    fn registry() -> Registry {
        Registry::new(vec![
            Box::new(Fake("office", vec![e("docx", "pdf", 3)])),
            Box::new(Fake("image", vec![e("pdf", "png", 1), e("png", "jpg", 1)])),
            Box::new(Fake("direct", vec![e("docx", "jpg", 10)])),
            Box::new(Fake("split", vec![e("pdf", "gif", 1)])),
        ])
    }

    #[test]
    fn finds_multi_step_path() {
        let plan = registry().plan("docx", "png").unwrap();
        let route: Vec<_> = plan.steps.iter().map(|s| s.converter).collect();
        assert_eq!(route, ["office", "image"]);
        assert_eq!(plan.cost, 4);
    }

    #[test]
    fn prefers_cheaper_path_over_fewer_steps() {
        // docx->pdf->png->jpg costs 5, the direct edge costs 10.
        let plan = registry().plan("docx", "jpg").unwrap();
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.cost, 5);
    }

    #[test]
    fn reports_missing_path() {
        assert!(matches!(
            registry().plan("png", "docx"),
            Err(Error::NoPath { .. })
        ));
    }

    #[test]
    fn executes_plan() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.docx");
        std::fs::write(&input, b"hello").unwrap();
        let out = dir.path().join("out.png");
        let ext = registry().convert(&input, "docx", "png", &out).unwrap();
        assert_eq!(ext, "png");
        assert_eq!(std::fs::read(out).unwrap(), b"hello");
    }

    #[test]
    fn multiple_pages_become_a_zip() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.pdf");
        std::fs::write(&input, b"%PDF-").unwrap();
        let out = dir.path().join("out.zip");
        let ext = registry().convert(&input, "pdf", "gif", &out).unwrap();
        assert_eq!(ext, "zip");

        let archive = zip::ZipArchive::new(std::fs::File::open(out).unwrap()).unwrap();
        let mut names: Vec<String> = archive
            .file_names()
            .map(|n| n.unwrap().to_string())
            .collect();
        names.sort();
        assert_eq!(names, ["page-1.gif", "page-2.gif", "page-3.gif"]);
    }
}
