//! PDF tools that aren't format conversions: merge several PDFs, or keep only
//! some pages. Both use qpdf (https://qpdf.sourceforge.io), which edits PDFs
//! structurally, so text stays selectable and quality is untouched.

use crate::converter::{on_path, run_command, run_command_output};
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Is qpdf installed?
pub fn available() -> bool {
    on_path("qpdf")
}

/// How many pages does this PDF have?
pub fn page_count(pdf: &Path) -> Result<u32> {
    let out = run_command_output(Command::new("qpdf").arg("--show-npages").arg(pdf))?;
    out.trim()
        .parse()
        .map_err(|_| Error::Unsupported(format!("could not read page count: {out:?}")))
}

/// Join `inputs` (in order) into one PDF at `output`.
pub fn merge(inputs: &[PathBuf], output: &Path) -> Result<()> {
    if inputs.len() < 2 {
        return Err(Error::Unsupported("merging needs at least two PDFs".into()));
    }
    // qpdf --empty --pages a.pdf b.pdf c.pdf -- out.pdf
    let mut cmd = Command::new("qpdf");
    cmd.args(["--empty", "--pages"])
        .args(inputs)
        .arg("--")
        .arg(output);
    run_qpdf(&mut cmd)
}

/// Keep only the pages described by `selection` (e.g. `"1-3, 5, 8-10"`).
pub fn extract_pages(input: &Path, selection: &str, output: &Path) -> Result<()> {
    let pages = page_count(input)?;
    let ranges = parse_page_ranges(selection, pages)?;
    // qpdf in.pdf --pages . 1-3,5 -- out.pdf   (`.` means "the input file")
    let mut cmd = Command::new("qpdf");
    cmd.arg(input)
        .args(["--pages", ".", &to_qpdf_ranges(&ranges), "--"])
        .arg(output);
    run_qpdf(&mut cmd)
}

/// qpdf exits with code 3 for "succeeded with warnings", which is fine for us.
fn run_qpdf(cmd: &mut Command) -> Result<()> {
    cmd.arg("--warning-exit-0");
    run_command(cmd)
}

/// An inclusive page range, 1-based. `(2, 4)` means pages 2, 3 and 4.
pub type PageRange = (u32, u32);

/// Parse a human page selection such as `"1-3, 5, 8-10"`.
///
/// Also accepts `"end"` (or `"last"`) for the last page, e.g. `"5-end"`.
/// Every page must exist in a document with `page_count` pages.
pub fn parse_page_ranges(selection: &str, page_count: u32) -> Result<Vec<PageRange>> {
    let invalid = |msg: String| Error::InvalidPages(msg);

    // Parse one page number. A nested function can't capture variables, so
    // `page_count` is passed in.
    fn page(s: &str, page_count: u32) -> Option<u32> {
        match s.trim() {
            "end" | "last" => Some(page_count),
            n => n.parse().ok(),
        }
    }

    let mut ranges = Vec::new();
    for part in selection
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        // `split_once` returns `Some((left, right))` if the separator is present.
        let (start, end) = match part.split_once('-') {
            Some((a, b)) => (page(a, page_count), page(b, page_count)),
            None => (page(part, page_count), page(part, page_count)),
        };
        let (Some(start), Some(end)) = (start, end) else {
            return Err(invalid(format!("{part:?} is not a page number or range")));
        };
        if start == 0 || end == 0 {
            return Err(invalid("pages are numbered from 1".into()));
        }
        if start > end {
            return Err(invalid(format!("{part:?} goes backwards")));
        }
        if end > page_count {
            return Err(invalid(format!(
                "page {end} doesn't exist, the document has {page_count} pages"
            )));
        }
        ranges.push((start, end));
    }
    if ranges.is_empty() {
        return Err(invalid("no pages selected".into()));
    }
    Ok(ranges)
}

fn to_qpdf_ranges(ranges: &[PageRange]) -> String {
    ranges
        .iter()
        .map(|&(a, b)| {
            if a == b {
                a.to_string()
            } else {
                format!("{a}-{b}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page_ranges() {
        let r = parse_page_ranges("1-3, 5, 8-10", 10).unwrap();
        assert_eq!(r, vec![(1, 3), (5, 5), (8, 10)]);
        assert_eq!(to_qpdf_ranges(&r), "1-3,5,8-10");
        assert_eq!(parse_page_ranges("7-end", 9).unwrap(), vec![(7, 9)]);
    }

    #[test]
    fn rejects_bad_ranges() {
        for bad in ["", "0", "3-1", "1-11", "a", "1--2", "1,,x"] {
            assert!(
                parse_page_ranges(bad, 10).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }
}
