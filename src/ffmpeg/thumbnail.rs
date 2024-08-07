use std::{path::Path, process::Command};

use ffmpeg_sidecar::{command::FfmpegCommand, ffprobe::ffprobe_path};
use rand::{thread_rng, Rng};

use crate::common::Result;

fn duration(path: impl AsRef<Path>) -> Result<f64> {
    let output = dbg!(Command::new(ffprobe_path())
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1")
        .arg(path.as_ref())
        .output()?);

    if output.status.success() {
        let string = dbg!(String::from_utf8_lossy(&output.stdout));
        if dbg!(string.starts_with("duration=")) {
            Ok(string.trim_start_matches("duration=").trim().parse()?)
        } else {
            Err(anyhow::anyhow!("Failed to parse duration"))?
        }
    } else {
        Err(anyhow::anyhow!("Failed to get duration"))?
    }
}

fn thumbnail(path: impl AsRef<Path>, image: impl AsRef<Path>) -> Result<()> {
    let duration = duration(path.as_ref())? as u64;

    let mut rand = thread_rng();

    let time = rand.gen_range((duration / 2)..duration) as f64;

    let mut child = FfmpegCommand::new()
        .input(&path.as_ref().to_string_lossy())
        .seek(time.to_string())
        .frames(1)
        .output(&image.as_ref().to_string_lossy())
        .spawn()?;

    let status = child.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to get thumbnail"))?
    }
}

#[cfg(test)]
mod tests {
    use ffmpeg_sidecar::download::auto_download;

    use super::*;

    #[test]
    fn test_thumbnail() {
        auto_download().unwrap();

        let time = std::time::Instant::now();

        thumbnail("./video_01.mp4", "1.jpeg").expect("");

        dbg!(time.elapsed());
    }
}
