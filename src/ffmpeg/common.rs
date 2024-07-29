use ffmpeg_sidecar::{
    command::ffmpeg_is_installed,
    download::{check_latest_version, download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg},
    paths::sidecar_dir,
    version::ffmpeg_version,
};
use ioc::{bean, Bean, BeanSpec, InitContext};
use tracing::{debug, info};

#[derive(Debug)]
pub(crate) struct FFmpegUtils;

pub(crate) fn init() -> ioc::Result<()> {
    if ffmpeg_is_installed() {
        let version = ffmpeg_version()?;
        info!("FFmpeg({version}) is already installed! 🎉");
    } else {
        let version = check_latest_version()?;
        info!("FFmpeg Latest available version: {version}");

        let download_url = ffmpeg_download_url()?;
        let destination = sidecar_dir()?;

        info!("Downloading from: {:?}", download_url);
        let archive_path = download_ffmpeg_package(download_url, &destination)?;
        info!("Downloaded package: {:?}", archive_path);

        info!("Extracting...");
        unpack_ffmpeg(&archive_path, &destination)?;

        let version = ffmpeg_version()?;
        info!("FFmpeg version: {}", version);

        info!("Done! 🏁");
    }
    Ok(())
}

#[bean]
impl BeanSpec for FFmpegUtils {
    type Bean = Self;

    fn build(_: &mut impl InitContext) -> ioc::Result<Self::Bean> {
        init()?;

        Ok(Self)
    }
}
