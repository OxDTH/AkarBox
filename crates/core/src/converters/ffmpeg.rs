use crate::converter::{
    all_pairs, file_size, on_path, run_command, run_command_output, same_format, Converter, Edge,
    Produced, Step,
};
use crate::options::Compression;
use crate::Result;
use std::process::Command;

const AUDIO: &[&str] = &["mp3", "wav", "flac", "ogg", "opus", "m4a", "aac"];
const VIDEO: &[&str] = &["mp4", "mkv", "webm", "mov", "avi"];
/// Lossy audio formats that can be re-encoded smaller (`mp3 -> mp3`).
const AUDIO_COMPRESSIBLE: &[&str] = &["mp3", "ogg", "opus", "m4a", "aac"];
const VIDEO_COMPRESSIBLE: &[&str] = &["mp4", "mkv", "webm", "mov"];
/// Outputs we encode as H.264, the codec hardware encoders support everywhere.
const H264_OUTPUTS: &[&str] = &["mp4", "mkv", "mov"];

/// The VAAPI device on Linux (Intel/AMD GPUs).
const VAAPI_DEVICE: &str = "/dev/dri/renderD128";

/// A GPU video encoder FFmpeg can use instead of the CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwEncoder {
    /// NVIDIA
    Nvenc,
    /// Intel Quick Sync
    Qsv,
    /// Intel/AMD on Linux
    Vaapi,
    /// Apple (macOS)
    VideoToolbox,
}

impl HwEncoder {
    /// Preference order when probing.
    const ALL: [HwEncoder; 4] = [Self::VideoToolbox, Self::Nvenc, Self::Qsv, Self::Vaapi];

    pub fn name(self) -> &'static str {
        match self {
            Self::Nvenc => "nvenc",
            Self::Qsv => "qsv",
            Self::Vaapi => "vaapi",
            Self::VideoToolbox => "videotoolbox",
        }
    }

    /// FFmpeg's name for this H.264 encoder.
    fn h264(self) -> &'static str {
        match self {
            Self::Nvenc => "h264_nvenc",
            Self::Qsv => "h264_qsv",
            Self::Vaapi => "h264_vaapi",
            Self::VideoToolbox => "h264_videotoolbox",
        }
    }

    /// Arguments that go *before* `-i` (opening the GPU device).
    fn input_args(self) -> Vec<String> {
        match self {
            Self::Vaapi => vec!["-vaapi_device".into(), VAAPI_DEVICE.into()],
            _ => vec![],
        }
    }

    /// Filters that must run last, to hand frames to the GPU.
    fn upload_filter(self) -> Option<&'static str> {
        match self {
            Self::Vaapi => Some("format=nv12,hwupload"),
            _ => None,
        }
    }

    /// Encoder + quality arguments. `crf` uses x264's scale: lower = better.
    fn encode_args(self, crf: u8) -> Vec<String> {
        let crf = crf.to_string();
        let args: Vec<&str> = match self {
            Self::Nvenc => vec![
                "-c:v",
                "h264_nvenc",
                "-rc",
                "vbr",
                "-cq",
                &crf,
                "-b:v",
                "0",
                "-pix_fmt",
                "yuv420p",
            ],
            Self::Qsv => vec![
                "-c:v",
                "h264_qsv",
                "-global_quality",
                &crf,
                "-pix_fmt",
                "nv12",
            ],
            Self::Vaapi => vec!["-c:v", "h264_vaapi", "-rc_mode", "CQP", "-qp", &crf],
            Self::VideoToolbox => {
                // VideoToolbox uses 1-100, higher = better. Map roughly from CRF.
                return vec![
                    "-c:v".into(),
                    "h264_videotoolbox".into(),
                    "-q:v".into(),
                    (106 - 2 * crf.parse::<i32>().unwrap_or(23))
                        .clamp(1, 100)
                        .to_string(),
                    "-pix_fmt".into(),
                    "yuv420p".into(),
                ];
            }
        };
        args.into_iter().map(String::from).collect()
    }

    /// Encode one tiny frame to check the GPU really works. FFmpeg *lists*
    /// NVENC even on machines without an NVIDIA card, so listing isn't enough.
    fn works(self) -> bool {
        if self == Self::Vaapi && !std::path::Path::new(VAAPI_DEVICE).exists() {
            return false;
        }
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "error"])
            .args(self.input_args())
            .args(["-f", "lavfi", "-i", "color=c=black:s=256x256:d=0.1"]);
        if let Some(filter) = self.upload_filter() {
            cmd.args(["-vf", filter]);
        }
        cmd.args(self.encode_args(23))
            .args(["-frames:v", "1", "-f", "null", "-"]);
        run_command(&mut cmd).is_ok()
    }
}

/// Audio & video via FFmpeg (https://ffmpeg.org).
pub struct Ffmpeg {
    /// GPU encoder to use for H.264 output, if any.
    hw: Option<HwEncoder>,
}

impl Ffmpeg {
    /// Use the `AKARBOX_HWACCEL` setting: `auto` (default: use a GPU if one
    /// works), `off`, or one of `nvenc`, `qsv`, `vaapi`, `videotoolbox`.
    pub fn detect() -> Self {
        let setting = std::env::var("AKARBOX_HWACCEL").unwrap_or_else(|_| "auto".into());
        Self {
            hw: Self::probe(&setting.to_ascii_lowercase()),
        }
    }

    /// Software encoding only.
    pub fn software() -> Self {
        Self { hw: None }
    }

    /// Force a specific hardware encoder without probing (used by tests to
    /// check the software fallback).
    pub fn with_hardware(hw: HwEncoder) -> Self {
        Self { hw: Some(hw) }
    }

    fn probe(setting: &str) -> Option<HwEncoder> {
        if matches!(setting, "off" | "none" | "false" | "0") || !on_path("ffmpeg") {
            return None;
        }
        let candidates: Vec<HwEncoder> = match setting {
            "auto" | "" => HwEncoder::ALL.to_vec(),
            name => match HwEncoder::ALL.into_iter().find(|hw| hw.name() == name) {
                Some(hw) => vec![hw],
                None => {
                    tracing::warn!(
                        setting,
                        "unknown AKARBOX_HWACCEL value, using software encoding"
                    );
                    return None;
                }
            },
        };
        let listed = run_command_output(Command::new("ffmpeg").args(["-hide_banner", "-encoders"]))
            .unwrap_or_default();
        let found = candidates
            .into_iter()
            .filter(|hw| listed.contains(hw.h264()))
            .find(|hw| hw.works());
        match found {
            Some(hw) => tracing::info!(encoder = hw.name(), "hardware video encoding enabled"),
            None if setting != "auto" => {
                tracing::warn!(
                    setting,
                    "requested hardware encoder doesn't work here, using software"
                )
            }
            None => tracing::info!("no hardware video encoder found, using software encoding"),
        }
        found
    }

    /// Build the FFmpeg command for `step`, using `hw` for H.264 if given.
    fn command(&self, step: &Step, hw: Option<HwEncoder>) -> Command {
        let opts = step.options;
        let to_audio = AUDIO.contains(&step.to);
        let level = opts.compression_for(step.from, step.to);
        let hw = hw.filter(|_| H264_OUTPUTS.contains(&step.to));

        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
        if let Some(hw) = hw {
            cmd.args(hw.input_args());
        }
        cmd.arg("-i").arg(step.input);

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
            // Build the video filter chain. `min(...)` means we only ever shrink;
            // `-2` keeps the aspect ratio with an even size, which codecs need.
            let mut filters = Vec::new();
            if let Some(h) = opts.video_height {
                filters.push(format!("scale=-2:'min({h},ih)'"));
            } else if let Some(w) = opts.max_width {
                filters.push(format!("scale='min({w},iw)':-2"));
            }
            if let Some(upload) = hw.and_then(HwEncoder::upload_filter) {
                filters.push(upload.into());
            }
            if !filters.is_empty() {
                cmd.args(["-vf", &filters.join(",")]);
            }

            let crf = level.map(x264_crf);
            if let Some(hw) = hw {
                cmd.args(hw.encode_args(crf.unwrap_or(23)));
            } else if H264_OUTPUTS.contains(&step.to) {
                cmd.args([
                    "-c:v", "libx264", "-preset", "medium", "-pix_fmt", "yuv420p",
                ]);
                if let Some(crf) = crf {
                    cmd.args(["-crf", &crf.to_string()]);
                }
            } else if step.to == "webm" {
                if let Some(level) = level {
                    cmd.args([
                        "-c:v",
                        "libvpx-vp9",
                        "-b:v",
                        "0",
                        "-crf",
                        &vp9_crf(level).to_string(),
                    ]);
                }
            }
            if matches!(step.to, "mp4" | "mov") {
                cmd.args(["-movflags", "+faststart"]); // start playing before fully downloaded
            }
        }

        let bitrate = opts.audio_bitrate_kbps.or(level.map(audio_kbps));
        let has_audio = to_audio || !opts.strip_audio;
        if let (Some(kbps), true, false) = (bitrate, has_audio, step.to == "gif") {
            if !matches!(step.to, "wav" | "flac") {
                cmd.args(["-b:a", &format!("{kbps}k")]);
            }
        }

        cmd.arg(step.output);
        cmd
    }
}

fn x264_crf(level: Compression) -> u8 {
    match level {
        Compression::Light => 23,
        Compression::Balanced => 28,
        Compression::Strong => 32,
    }
}

fn vp9_crf(level: Compression) -> u8 {
    match level {
        Compression::Light => 32,
        Compression::Balanced => 37,
        Compression::Strong => 42,
    }
}

fn audio_kbps(level: Compression) -> u32 {
    match level {
        Compression::Light => 192,
        Compression::Balanced => 128,
        Compression::Strong => 96,
    }
}

impl Converter for Ffmpeg {
    fn id(&self) -> &'static str {
        "ffmpeg"
    }

    fn is_available(&self) -> bool {
        on_path("ffmpeg")
    }

    fn hardware_acceleration(&self) -> Option<&'static str> {
        self.hw.map(HwEncoder::name)
    }

    fn edges(&self) -> Vec<Edge> {
        let mut edges = all_pairs(AUDIO, AUDIO, 1);
        edges.extend(all_pairs(VIDEO, VIDEO, 2));
        edges.extend(all_pairs(VIDEO, AUDIO, 1)); // extract the soundtrack
        edges.extend(all_pairs(VIDEO, &["gif"], 2));
        edges.extend(same_format(AUDIO_COMPRESSIBLE, 1));
        edges.extend(same_format(VIDEO_COMPRESSIBLE, 2));
        edges
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        let result = run_command(&mut self.command(step, self.hw));
        match (result, self.hw) {
            (Ok(()), _) => {}
            // The GPU can fail on unusual input (odd sizes, 10-bit video...).
            // Fall back to the CPU rather than failing the conversion.
            (Err(err), Some(hw)) if H264_OUTPUTS.contains(&step.to) => {
                tracing::warn!(encoder = hw.name(), %err, "hardware encoding failed, retrying in software");
                run_command(&mut self.command(step, None))?;
            }
            (Err(err), _) => return Err(err),
        }
        if step.from == step.to && file_size(step.output)? >= file_size(step.input)? {
            std::fs::copy(step.input, step.output)?; // compression didn't help
        }
        Ok(Produced::File)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;

    fn args(cmd: &Command) -> String {
        cmd.get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn step<'a>(from: &'a str, to: &'a str, options: &'a Options) -> Step<'a> {
        let p = std::path::Path::new("/tmp/x");
        Step {
            input: p,
            output: p,
            from,
            to,
            work_dir: p,
            options,
        }
    }

    #[test]
    fn compression_sets_crf_and_bitrate() {
        let opts = Options::default();
        let cmd = Ffmpeg::software().command(&step("mp4", "mp4", &opts), None);
        let a = args(&cmd);
        assert!(a.contains("-c:v libx264"), "{a}");
        assert!(
            a.contains("-crf 28"),
            "same-format defaults to balanced: {a}"
        );
        assert!(a.contains("-b:a 128k"), "{a}");
    }

    #[test]
    fn hardware_encoder_only_for_h264_outputs() {
        let opts = Options {
            video_height: Some(720),
            ..Options::default()
        };
        let ff = Ffmpeg::software();
        let a = args(&ff.command(&step("mov", "mp4", &opts), Some(HwEncoder::Vaapi)));
        assert!(a.contains("-vaapi_device"), "{a}");
        assert!(
            a.contains("scale=-2:'min(720,ih)',format=nv12,hwupload"),
            "{a}"
        );
        assert!(a.contains("h264_vaapi"), "{a}");

        let a = args(&ff.command(&step("mp4", "webm", &opts), Some(HwEncoder::Nvenc)));
        assert!(!a.contains("nvenc"), "webm stays on the CPU: {a}");
    }
}
