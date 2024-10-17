use anyhow::anyhow;
use ioc::Bean;
use poem_openapi::NewType;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    fs::{create_dir_all, remove_file as std_remove_file},
    ops::Deref,
    path::{
        Path,
        PathBuf,
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{
    fs::{remove_file, try_exists, File as TokioFile},
    io::AsyncRead,
};
use tokio::fs::remove_dir_all;
use tracing::{error, warn};
use url::Url;
use uuid::Uuid;

use crate::common::{AppError, Result};
use crate::util::poem::BaseUrl;

#[derive(NewType, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Id(pub(crate) String);

impl Id {
    pub(crate) fn new_uuid() -> Self {
        Self(Uuid::new_v4().as_simple().to_string())
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Deref for Id {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

pub(crate) enum SavedId {
    Existed,
    New,
}

pub(crate) trait Storage {
    async fn exists(&self, id: &Id) -> Result<bool>;

    async fn delete(&self, id: &Id) -> Result<()>;

    async fn raw_file(&self, id: &Id) -> Result<PathBuf>;

    async fn assert_file(&self, id: &Id, path: impl AsRef<Path>) -> Result<PathBuf>;

    async fn save(&self, id: &Id, source: impl AsyncRead + Unpin) -> Result<SavedId>;

    fn url(&self, base_url: &BaseUrl, id: &Id, path: impl AsRef<str>) -> Result<Url>;
}

#[derive(Bean)]
pub(crate) struct LocalStorage {
    #[inject(config = "web.static.mapping.storage.dir")]
    dir: PathBuf,
    #[inject(config = "web.static.mapping.storage.path")]
    uri_path: String,
}

struct TmpFile {
    target: TokioFile,
    path: PathBuf,
}

impl TmpFile {
    async fn new(path: PathBuf) -> Result<Self> {
        let target = TokioFile::create(&path).await?;
        Ok(Self { target, path })
    }

    async fn copy_from(mut self, mut source: impl AsyncRead + Unpin) -> Result<()> {
        let mut cache = [0; 512];

        loop {
            match source.read(&mut cache).await? {
                0 => break,
                n => {
                    self.target.write_all(&cache[..n]).await?;
                }
            };
        }

        // skip file clean in `Drop::drop`
        std::mem::forget(self);
        Ok(())
    }
}

impl Drop for TmpFile {
    fn drop(&mut self) {
        let path = &self.path;
        if let Err(e) = std_remove_file(path) {
            error!("remove tmp file {} failed: {e:?}", path.display());
        }
    }
}

impl LocalStorage {
    fn path(&self, id: &Id) -> PathBuf {
        self.dir.join(&id.0)
    }
}

impl Storage for LocalStorage {
    async fn exists(&self, id: &Id) -> Result<bool> {
        let path = self.path(id);
        Ok(try_exists(path).await?)
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        let path = self.path(id);
        if try_exists(&path).await? {
            if path.is_dir() {
                Ok(remove_dir_all(&path).await?)
            } else {
                Ok(remove_file(&path).await?)
            }
        } else {
            warn!("file {} is not exist! skip delete op!", path.display());
            Ok(())
        }
    }

    async fn raw_file(&self, id: &Id) -> Result<PathBuf> {
        if !self.exists(id).await? {
            return Err(AppError::MaterialNotFound(id.to_string()));
        }

        let mut buf = self.path(id);
        buf.push("raw");
        Ok(buf)
    }

    async fn assert_file(&self, id: &Id, path: impl AsRef<Path>) -> Result<PathBuf> {
        if !self.exists(id).await? {
            return Err(AppError::MaterialNotFound(id.to_string()));
        }

        let mut buf = self.path(id);
        buf.push(path);
        Ok(buf)
    }

    async fn save(&self, id: &Id, source: impl AsyncRead + Unpin) -> Result<SavedId> {
        let mut target = self.path(&id);
        target.push("raw");

        if try_exists(&target).await? {
            Ok(SavedId::Existed)
        } else {
            if let Some(parent) = target.parent() {
                create_dir_all(parent)?;
            }
            TmpFile::new(target)
                .await?
                .copy_from(source)
                .await?;
            Ok(SavedId::New)
        }
    }

    fn url(&self, base_url: &BaseUrl, id: &Id, path: impl AsRef<str>) -> Result<Url> {
        let mut url = base_url.join(self.uri_path.as_str())?;
        url.path_segments_mut()
            .map_err(|_| AppError::Other(anyhow!("invalid base url:{}", self.uri_path)))?
            .push(id.as_ref())
            .push(path.as_ref());
        Ok(url)
    }
}
