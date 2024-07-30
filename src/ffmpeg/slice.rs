use std::fs;
use std::path::Path;

use ffmpeg_sidecar::command::FfmpegCommand;

use crate::common::Result;

fn slice(input: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Result<()> {
    let status = FfmpegCommand::new()
        .input(&input.as_ref().to_string_lossy())
        .codec_video("libx264")
        .args(&["-filter:v", "scale=1280:-1", "-g", "30"])
        .args(&["-profile:v", "main", "-level", "4.0"])
        .args(&["-b:v", "1500k", "-maxrate", "1500k", "-bufsize", "2250k"])
        .args(&[
            "-start_number",
            "0",
            "-hls_time",
            "1",
            "-hls_list_size",
            "0",
            "-f",
            "hls",
        ])
        .arg(output_dir.as_ref().join("720p").join("slice.m3u8"))
        .codec_video("libx264")
        .args(&["-filter:v", "scale=1280:-1", "-g", "30"])
        .args(&["-profile:v", "main", "-level", "4.2"])
        .args(&["-b:v", "3000k", "-maxrate", "3000k", "-bufsize", "4500k"])
        .args(&[
            "-start_number",
            "0",
            "-hls_time",
            "1",
            "-hls_list_size",
            "0",
            "-f",
            "hls",
        ])
        .arg(output_dir.as_ref().join("1080p").join("slice.m3u8"))
        .spawn()?
        .wait()?;

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

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to get slice"))?
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_thumbnail() {
        let time = std::time::Instant::now();

        slice("./video_01.mp4", "./storage")
            .expect("");

        dbg!(time.elapsed());
    }
}
