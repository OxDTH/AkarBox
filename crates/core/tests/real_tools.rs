//! Integration tests that run the real external tools.
//!
//! Files in `tests/` are compiled as separate programs that use the crate like
//! any other user would (only `pub` items are visible). Each test skips itself
//! when a tool it needs isn't installed, so `cargo test` works everywhere.

use akarbox_core::{detect_format, pdf, Options, Registry};
use std::path::{Path, PathBuf};
use std::process::Command;

fn has(program: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {program}"))
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Skip the test (by returning early) unless every program is installed.
macro_rules! require {
    ($($program:expr),+) => {
        $(if !has($program) {
            eprintln!("skipping: {} not installed", $program);
            return;
        })+
    };
}

fn run(cmd: &mut Command) {
    let status = cmd.status().expect("spawn");
    assert!(status.success(), "{cmd:?} failed");
}

/// Make a PDF with `pages` pages using ImageMagick.
fn make_pdf(dir: &Path, name: &str, pages: usize) -> PathBuf {
    let path = dir.join(name);
    let mut cmd = Command::new("convert");
    for i in 0..pages {
        cmd.args(["-size", "200x100", &format!("xc:rgb({},100,200)", i * 40)]);
    }
    cmd.arg(&path);
    run(&mut cmd);
    path
}

fn header(path: &Path) -> Vec<u8> {
    let mut bytes = std::fs::read(path).unwrap();
    bytes.truncate(4096);
    bytes
}

#[test]
fn multi_page_pdf_becomes_zip_of_pages() {
    require!("convert", "gs");
    let dir = tempfile::tempdir().unwrap();
    let pdf_file = make_pdf(dir.path(), "doc.pdf", 3);

    let registry = Registry::with_builtin();
    let plan = registry.plan("pdf", "png").unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let out = registry
        .execute(&plan, &pdf_file, &work, &Options::default())
        .unwrap();

    assert_eq!(out.ext, "zip");
    let archive = zip::ZipArchive::new(std::fs::File::open(&out.path).unwrap()).unwrap();
    assert_eq!(archive.len(), 3);
}

#[test]
fn single_page_pdf_stays_a_single_image() {
    require!("convert", "gs");
    let dir = tempfile::tempdir().unwrap();
    let pdf_file = make_pdf(dir.path(), "one.pdf", 1);
    let out = dir.path().join("out.jpg");
    let ext = Registry::with_builtin()
        .convert(&pdf_file, "pdf", "jpg", &out)
        .unwrap();
    assert_eq!(ext, "jpg");
    assert_eq!(detect_format("x", &header(&out)).unwrap().ext, "jpg");
}

#[test]
fn image_options_resize_and_quality() {
    require!("convert", "identify");
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("big.png");
    run(Command::new("convert")
        .args(["-size", "1000x500", "plasma:"])
        .arg(&input));

    let registry = Registry::with_builtin();
    let plan = registry.plan("png", "jpg").unwrap();
    let options = Options {
        quality: Some(40),
        max_width: Some(200),
        ..Options::default()
    };
    let out = registry
        .execute(&plan, &input, dir.path(), &options)
        .unwrap();

    let size = Command::new("identify")
        .args(["-format", "%wx%h %Q"])
        .arg(&out.path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&size.stdout), "200x100 40");
}

#[test]
fn heic_converts_via_libheif() {
    require!("heif-enc", "convert", "identify");
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("photo.png");
    run(Command::new("convert")
        .args(["-size", "64x48", "gradient:red-blue"])
        .arg(&png));
    let heic = dir.path().join("photo.heic");
    run(Command::new("heif-enc").arg("-o").arg(&heic).arg(&png));

    // Detected from its bytes even with a misleading name.
    assert_eq!(
        detect_format("IMG_0001.jpg", &header(&heic)).unwrap().ext,
        "heic"
    );

    let registry = Registry::with_builtin();
    let targets: Vec<_> = registry
        .targets("heic")
        .iter()
        .map(|t| t.format.ext)
        .collect();
    for ext in ["jpg", "png", "webp"] {
        assert!(targets.contains(&ext), "heic should convert to {ext}");
    }
    // heic -> png (libheif) -> jpg (ImageMagick), and the options still apply.
    let plan = registry.plan("heic", "jpg").unwrap();
    let options = Options {
        max_width: Some(32),
        ..Options::default()
    };
    let out = registry
        .execute(&plan, &heic, dir.path(), &options)
        .unwrap();
    assert_eq!(detect_format("x", &header(&out.path)).unwrap().ext, "jpg");
    let size = Command::new("identify")
        .args(["-format", "%wx%h"])
        .arg(&out.path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&size.stdout), "32x24");
}

#[test]
fn audio_bitrate_option() {
    require!("ffmpeg", "ffprobe");
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("tone.wav");
    run(Command::new("ffmpeg")
        .args(["-loglevel", "error", "-f", "lavfi", "-i", "sine=d=2"])
        .arg(&wav));

    let registry = Registry::with_builtin();
    let plan = registry.plan("wav", "mp3").unwrap();
    let options = Options {
        audio_bitrate_kbps: Some(64),
        ..Options::default()
    };
    let out = registry.execute(&plan, &wav, dir.path(), &options).unwrap();

    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=bit_rate",
            "-of",
            "csv=p=0",
        ])
        .arg(&out.path)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "64000");
}

#[test]
fn pdf_merge_and_split() {
    require!("qpdf", "convert");
    let dir = tempfile::tempdir().unwrap();
    let a = make_pdf(dir.path(), "a.pdf", 2);
    let b = make_pdf(dir.path(), "b.pdf", 3);

    let merged = dir.path().join("merged.pdf");
    pdf::merge(&[a, b], &merged).unwrap();
    assert_eq!(pdf::page_count(&merged).unwrap(), 5);

    let split = dir.path().join("split.pdf");
    pdf::extract_pages(&merged, "1-2, 5", &split).unwrap();
    assert_eq!(pdf::page_count(&split).unwrap(), 3);

    assert!(pdf::extract_pages(&merged, "6", &split).is_err());
}
