//! # akarbox-core
//!
//! The conversion engine. It knows nothing about HTTP or windows; the server and
//! the desktop app are thin shells around it.
//!
//! The big idea:
//! 1. Every [`Converter`] declares *edges* like `docx -> pdf` that it can perform.
//! 2. The [`Registry`] puts all edges into one graph.
//! 3. To convert `docx -> png`, the planner finds the cheapest path through the
//!    graph (e.g. `docx -LibreOffice-> pdf -ImageMagick-> png`) and runs each step.
//!
//! That is how "anything to anything" works without writing N×N converters.

// `mod foo;` tells Rust to compile the file `foo.rs` (or `foo/mod.rs`) as a module.
mod converter;
pub mod converters;
mod error;
mod format;
mod options;
pub mod pdf;
mod registry;
mod sniff;

// `pub use` re-exports items so users can write `akarbox_core::Registry`
// instead of `akarbox_core::registry::Registry`.
pub use converter::{run_command, run_command_output, Converter, Edge, Produced, Step};
pub use error::{Error, Result};
pub use format::{detect_format, format_from_name, normalize_ext, Category, Format, FORMATS};
pub use options::{Compression, Options};
pub use registry::{zip_files, Output, Plan, PlannedStep, Registry, Target};
