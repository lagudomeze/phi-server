use ffmpeg_sidecar::command::FfmpegCommand;
use rand::{thread_rng, Rng};
use std::{ffi::OsStr, path::Path, process::Command};

use crate::common::Result;

fn duration(path: impl AsRef<Path>, ffprobe: impl AsRef<OsStr>) -> Result<f64> {
    println!("haha:{:?}, {:?}", path.as_ref(), ffprobe.as_ref());
    let output = dbg!(Command::new(ffprobe)
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

pub(crate) fn thumbnail(
    path: impl AsRef<Path>,
    image: impl AsRef<Path>,
    ffmpeg_path: &Path,
    ffprobe_path: &Path,
) -> Result<()> {
    let duration = duration(path.as_ref(), ffprobe_path)? as u64;

    let mut rand = thread_rng();

    let time = rand.gen_range((duration / 2)..duration) as f64;

    let mut child = FfmpegCommand::new_with_path(ffmpeg_path)
        .input(path.as_ref().to_string_lossy())
        .seek(time.to_string())
        .frames(1)
        .output(image.as_ref().to_string_lossy())
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
    use super::*;
    use ffmpeg_sidecar::{download::auto_download, ffprobe::ffprobe_path, paths::ffmpeg_path};

    #[test]
    fn test_thumbnail() {
        auto_download().unwrap();

        let time = std::time::Instant::now();

        let ffmpeg = ffmpeg_path();
        let ffprobe = ffprobe_path();
        thumbnail("./video_01.mp4", "1.jpeg", &ffmpeg, &ffprobe).expect("");

        dbg!(time.elapsed());
    }
}
