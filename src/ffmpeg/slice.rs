use ffmpeg_sidecar::command::FfmpegCommand;
use std::ffi::OsStr;
use std::{fs, path::Path};
use tracing::debug;

use crate::common::Result;

pub(crate) fn slice(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    ffmpeg_path: impl AsRef<OsStr>,
) -> Result<()> {
    let slice_720p = output_dir.as_ref().join("720p");
    fs::create_dir_all(&slice_720p)?;

    let slice_1080p = output_dir.as_ref().join("1080p");
    fs::create_dir_all(&slice_1080p)?;

    let mut child = FfmpegCommand::new_with_path(ffmpeg_path)
        .input(input.as_ref().to_string_lossy())
        .codec_video("libx264")
        .args(["-filter:v", "scale=1280:-1", "-g", "30"])
        .args(["-profile:v", "main", "-level", "4.0"])
        .args(["-b:v", "1500k", "-maxrate", "1500k", "-bufsize", "2250k"])
        .args([
            "-start_number",
            "0",
            "-hls_time",
            "1",
            "-hls_list_size",
            "0",
            "-f",
            "hls",
        ])
        .arg(slice_720p.join("slice.m3u8"))
        .codec_video("libx264")
        .args(["-filter:v", "scale=1280:-1", "-g", "30"])
        .args(["-profile:v", "main", "-level", "4.2"])
        .args(["-b:v", "3000k", "-maxrate", "3000k", "-bufsize", "4500k"])
        .args([
            "-start_number",
            "0",
            "-hls_time",
            "1",
            "-hls_list_size",
            "0",
            "-f",
            "hls",
        ])
        .arg(slice_1080p.join("slice.m3u8"))
        .spawn()?;

    for e in child.iter()? {
        debug!("{e:#?}");
    }

    let status = child.wait()?;

    if status.success() {
        fs::write(
            output_dir.as_ref().join("slice.m3u8"),
            "
                    #EXTM3U
                    #EXT-X-STREAM-INF:BANDWIDTH=1500000,RESOLUTION=1280x720
                    720p/slice.m3u8
                    #EXT-X-STREAM-INF:BANDWIDTH=3000000,RESOLUTION=1920x1080
                    1080p/slice.m3u8
                    ",
        )?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to get slice {}", status))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffmpeg_sidecar::{
        download::{auto_download, ffmpeg_download_url},
        paths::ffmpeg_path,
    };

    #[test]
    fn test_slice() {
        println!("{}", ffmpeg_download_url().unwrap());

        auto_download().unwrap();

        let time = std::time::Instant::now();

        let ffmpeg = ffmpeg_path();

        slice("./video_01.mp4", "./storage", &ffmpeg).expect("");

        dbg!(time.elapsed());
    }
}
