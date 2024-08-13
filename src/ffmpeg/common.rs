use crate::{
    ffmpeg::slice::slice,
    ffmpeg::thumbnail::thumbnail,
};
use anyhow::Context;
use ffmpeg_sidecar::{
    download::{check_latest_version, download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg},
    version::ffmpeg_version_with_path
};
use ioc::{bean, BeanSpec, InitContext};
use std::{
    path::{Path, PathBuf},
    process::{
        Command,
        Stdio,
    },
};
use tracing::info;

#[derive(Debug)]
pub(crate) struct FFmpegUtils {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

fn sidecar_path(sidecar_parent: impl AsRef<Path>, name: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let mut path = sidecar_parent
        .as_ref()
        .parent()
        .context("Can't get parent of current_exe")?
        .join(name);
    if cfg!(windows) {
        path.set_extension("exe");
    }
    Ok(path)
}

fn path(sidecar_parent: impl AsRef<Path>, name: &str) -> PathBuf {
    let default = Path::new(name).to_path_buf();
    match sidecar_path(sidecar_parent, name) {
        Ok(sidecar_path) => match sidecar_path.exists() {
            true => sidecar_path,
            false => default,
        },
        Err(_) => default,
    }
}

fn is_installed(sidecar_parent: impl AsRef<Path>, name: impl AsRef<Path>) -> bool {
    Command::new(sidecar_path(sidecar_parent, name).unwrap())
        .arg("-version")
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or_else(|_| false)
}
impl FFmpegUtils {
    fn init(sidecar_parent: PathBuf) -> ioc::Result<Self> {
        if is_installed(&sidecar_parent, "ffmpeg") {
            let ffmpeg_path = path(&sidecar_parent, "ffmpeg");
            let version = ffmpeg_version_with_path(&ffmpeg_path)?;
            info!("FFmpeg({version}) is already installed! 🎉");
            let ffprobe_path = path(&sidecar_parent, "ffprobe");
            info!("ffmpeg_path: {}", ffmpeg_path.display());
            info!("ffprobe_path: {}", ffprobe_path.display());
            Ok(Self {
                ffmpeg_path,
                ffprobe_path,
            })
        } else {
            let version = check_latest_version()?;
            info!("FFmpeg Latest available version: {version}");

            let download_url = ffmpeg_download_url()?;
            let destination = sidecar_parent.as_path();

            info!("Downloading from: {:?}", download_url);
            let archive_path = download_ffmpeg_package(download_url, &destination)?;
            info!("Downloaded package: {:?}", archive_path);

            info!("Extracting to {} ...", destination.display());
            unpack_ffmpeg(&archive_path, &destination)?;

            let ffmpeg_path = path(&sidecar_parent, "ffmpeg");
            let ffprobe_path = path(&sidecar_parent, "ffprobe");

            let version = ffmpeg_version_with_path(&ffmpeg_path)?;
            info!("FFmpeg version: {}", version);

            info!("Done! 🏁");

            info!("ffmpeg_path: {}", ffmpeg_path.display());
            info!("ffprobe_path: {}", ffprobe_path.display());

            Ok(Self {
                ffmpeg_path,
                ffprobe_path,
            })
        }
    }
}

#[bean]
impl BeanSpec for FFmpegUtils {
    type Bean = Self;

    fn build(ctx: &mut impl InitContext) -> ioc::Result<Self::Bean> {
        let sidecar_parent = ctx.get_config::<PathBuf>("ffmpeg.sidecar_parent")?;
        Ok(Self::init(sidecar_parent)?)
    }
}

impl FFmpegUtils {
    pub(crate) fn slice(&self, input: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> crate::common::Result<()> {
        slice(input, output_dir, &self.ffmpeg_path)
    }

    pub(crate) fn thumbnail(&self, path: impl AsRef<Path>, image: impl AsRef<Path>) -> crate::common::Result<()> {
        thumbnail(path, image, self.ffmpeg_path.as_path(), self.ffprobe_path.as_path())
    }
}
