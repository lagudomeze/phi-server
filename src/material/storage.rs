use std::{
    fs::remove_file as std_remove_file,
    ops::Deref,
    path::{Path, PathBuf},
};

use base64ct::{Base64, Encoding};
use ioc::Bean;
use poem_openapi::NewType;
use sha2::{Digest, Sha256};
use tokio::{
    fs::{File as TokioFile, remove_file, rename, try_exists},
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
};
use tracing::{error, warn};
use uuid::Uuid;

use crate::common::Result;

#[derive(NewType)]
pub(crate) struct Id(String);

impl Deref for Id {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

pub(crate) trait Storage {
    async fn exists(&self, id: &Id) -> Result<bool>;

    async fn delete(&self, id: &Id) -> Result<()>;

    async fn save(&self, source: impl AsyncRead + Unpin) -> Result<Id>;
}

#[derive(Bean)]
pub(crate) struct LocalStorage {
    #[inject(config = "web.static.mapping.storage.dir")]
    dir: PathBuf,
}

struct TmpFile {
    path: PathBuf,
}

impl TmpFile {
    async fn new(dir: impl AsRef<Path>) -> Result<(Self, TokioFile)> {
        let path = dir.as_ref().join(&format!("{}.tmp", Uuid::new_v4()));
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

    async fn save(&self, mut source: impl AsyncRead + Unpin) -> Result<Id> {
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

        move_guard.move_to(self.path(&id)).await?;
        Ok(id)
    }
}
