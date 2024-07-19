use std::{
    path::Path,
    process::Command,
};

use ffmpeg_sidecar::{
    command::FfmpegCommand,
    ffprobe::ffprobe_path,
};
use rand::{Rng, thread_rng};

use crate::common::Result;

fn duration(path: impl AsRef<Path>) -> Result<f64> {
    let output = dbg!(
        Command::new(ffprobe_path())
            .arg("-v")
            .arg("error")
            .arg("-show_entries")
            .arg("format=duration")
            .arg("-of")
            .arg("default=noprint_wrappers=1")
            .arg(path.as_ref()).output()?
    );

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

    child.wait()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use ffmpeg_sidecar::download::auto_download;

    use super::*;

    #[test]
    fn test_thumbnail() {
        auto_download().unwrap();

        let time = std::time::Instant::now();

        thumbnail("D:/delete/material/store/79dcc1c32242a04081fa8f9f26fc349d4c330e14e4f8f2dac045a34be30cd71a/raw", "1.jpeg")
            .expect("");

        dbg!(time.elapsed());
    }
}