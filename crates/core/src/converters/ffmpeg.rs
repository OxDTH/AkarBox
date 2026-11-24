use crate::converter::{all_pairs, on_path, run_command, Converter, Edge, Produced, Step};
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

    fn convert(&self, step: &Step) -> Result<Produced> {
        let opts = step.options;
        let to_audio = AUDIO.contains(&step.to);
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(step.input);

        if to_audio {
            cmd.arg("-vn"); // drop any video stream
        } else if opts.strip_audio {
            cmd.arg("-an"); // drop the audio stream
        }

        if step.to == "gif" {
            // 12 fps and a generated palette give small, good-looking GIFs.
            let width = opts.max_width.unwrap_or(480);
            cmd.args([
                "-vf",
                &format!(
                    "fps=12,scale='min({width},iw)':-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse"
                ),
            ]);
        } else if !to_audio {
            // `min(...,ih)` means we only ever shrink. `-2` keeps the aspect ratio
            // and an even width, which most video codecs require.
            if let Some(h) = opts.video_height {
                cmd.args(["-vf", &format!("scale=-2:'min({h},ih)'")]);
            } else if let Some(w) = opts.max_width {
                cmd.args(["-vf", &format!("scale='min({w},iw)':-2")]);
            }
        }

        if let Some(kbps) = opts.audio_bitrate_kbps {
            if step.to != "gif" && !(opts.strip_audio && !to_audio) {
                cmd.args(["-b:a", &format!("{kbps}k")]);
            }
        }

        cmd.arg(step.output);
        run_command(&mut cmd)?;
        Ok(Produced::File)
    }
}
