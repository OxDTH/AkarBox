use crate::converter::{all_pairs, on_path, run_command, Converter, Edge, Step};
use crate::Result;
use std::process::Command;

const AUDIO: &[&str] = &["mp3", "wav", "flac", "ogg", "opus", "m4a", "aac"];
const VIDEO: &[&str] = &["mp4", "mkv", "webm", "mov", "avi"];

/// Audio & video via FFmpeg (https://ffmpeg.org).
///
/// A "unit struct" (no fields): we only need it to hang the trait methods on.
pub struct Ffmpeg;

impl Converter for Ffmpeg {
    fn id(&self) -> &'static str {
        "ffmpeg"
    }

    fn is_available(&self) -> bool {
        on_path("ffmpeg")
    }

    fn edges(&self) -> Vec<Edge> {
        let mut edges = all_pairs(AUDIO, AUDIO, 1);
        edges.extend(all_pairs(VIDEO, VIDEO, 2));
        edges.extend(all_pairs(VIDEO, AUDIO, 1)); // extract the soundtrack
        edges.extend(all_pairs(VIDEO, &["gif"], 2));
        edges
    }

    fn convert(&self, step: &Step) -> Result<()> {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(step.input);

        if AUDIO.contains(&step.to) {
            cmd.arg("-vn"); // drop any video stream
        }
        if step.to == "gif" {
            // Reasonable default: 12 fps, 480px wide, good palette.
            cmd.args([
                "-vf",
                "fps=12,scale=480:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse",
            ]);
        }
        cmd.arg(step.output);
        run_command(&mut cmd)
    }
}
