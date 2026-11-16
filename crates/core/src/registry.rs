//! The registry holds every available converter and plans multi-step conversions.

use crate::converter::{Converter, Step};
use crate::format::{normalize_ext, Format};
use crate::{Error, Result};
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

    /// Run a plan. Intermediate files are written to `work_dir`; the final output
    /// path is returned.
    pub fn execute(&self, plan: &Plan, input: &Path, work_dir: &Path) -> Result<PathBuf> {
        let mut current = input.to_path_buf();
        for (i, planned) in plan.steps.iter().enumerate() {
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
            converter.convert(&Step {
                input: &current,
                output: &output,
                from: planned.from,
                to: planned.to,
                work_dir: &step_dir,
            })?;

            if !output.is_file() {
                return Err(Error::MissingOutput(output));
            }
            current = output;
        }
        Ok(current)
    }

    /// Convenience: plan + execute, using a temporary directory for intermediates.
    /// The result is copied to `output`.
    pub fn convert(&self, input: &Path, from: &str, to: &str, output: &Path) -> Result<()> {
        let plan = self.plan(from, to)?;
        let tmp = tempfile::tempdir()?; // deleted automatically when `tmp` goes out of scope
        let result = self.execute(&plan, input, tmp.path())?;
        std::fs::copy(result, output)?;
        Ok(())
    }
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
        fn convert(&self, step: &Step) -> Result<()> {
            std::fs::copy(step.input, step.output)?;
            Ok(())
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
        registry().convert(&input, "docx", "png", &out).unwrap();
        assert_eq!(std::fs::read(out).unwrap(), b"hello");
    }
}
