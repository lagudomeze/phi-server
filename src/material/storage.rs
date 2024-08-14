use anyhow::anyhow;
use base64ct::{Base64, Encoding};
use ioc::Bean;
use poem_openapi::NewType;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};
use std::{
    fs::{create_dir_all, remove_file as std_remove_file},
    ops::Deref,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{remove_file, rename, try_exists, File as TokioFile},
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
};
use tracing::{error, warn};
use url::Url;
use uuid::Uuid;

use crate::common::{AppError, Result};
use crate::util::poem::BaseUrl;

#[derive(NewType, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Id(pub(crate) String);

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
    Existed(Id),
    New(Id),
}

impl From<SavedId> for Id {
    fn from(value: SavedId) -> Self {
        match value {
            SavedId::Existed(id) => id,
            SavedId::New(id) => id,
        }
    }
}

pub(crate) trait Storage {
    async fn exists(&self, id: &Id) -> Result<bool>;

    async fn delete(&self, id: &Id) -> Result<()>;

    async fn raw_file(&self, id: &Id) -> Result<PathBuf>;

    async fn assert_file(&self, id: &Id, path: impl AsRef<Path>) -> Result<PathBuf>;

    async fn save(&self, source: impl AsyncRead + Unpin) -> Result<SavedId>;

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
    path: PathBuf,
}

impl TmpFile {
    async fn new(dir: impl AsRef<Path>) -> Result<(Self, TokioFile)> {
        let path = dir.as_ref().join(format!("{}.tmp", Uuid::new_v4()));
        let file = TokioFile::create(&path).await?;
        Ok((Self { path }, file))
    }

    async fn move_to(self, target: impl AsRef<Path>) -> Result<()> {
        rename(&self.path, target).await?;
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
            Ok(remove_file(&path).await?)
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

    async fn save(&self, mut source: impl AsyncRead + Unpin) -> Result<SavedId> {
        // todo maybe use cache in heap?
        let mut cache = [0; 512];

        let (move_guard, mut target) = TmpFile::new(&self.dir).await?;

        let mut hasher = Sha256::new();
        loop {
            match source.read(&mut cache).await? {
                0 => break,
                n => {
                    hasher.update(&cache[..n]);
                    target.write_all(&cache[..n]).await?;
                }
            };
        }

        let array = hasher.finalize();
        let id = Id(Base64::encode_string(&array));

        let mut target = self.path(&id);
        target.push("raw");

        if try_exists(&target).await? {
            Ok(SavedId::Existed(id))
        } else {
            if let Some(parent) = target.parent() {
                create_dir_all(parent)?;
            }
            move_guard.move_to(target).await?;
            Ok(SavedId::New(id))
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
