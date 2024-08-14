use chrono::{NaiveDateTime, Utc};
use ioc::Bean;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{borrow::Cow, ops::Deref};
use tokio::{sync::mpsc::Sender, task::spawn_blocking};
use tracing::{info, warn};

use crate::auth::jwt::Claims;
use crate::common::{AppError, FormatedEvent};
use crate::ffmpeg::common::FFmpegUtils;
use crate::material::{STATE_OK, TYPE_VIDEO};
use crate::util::poem::BaseUrl;
use crate::{
    common::Result,
    db::Db,
    material::storage::SavedId,
    material::{
        storage::{Id, LocalStorage, Storage},
        mvc::UploadPayload,
    },
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "state")]
pub(crate) enum VideoUploadEvent<'a> {
    #[serde(rename(serialize = "already_existed"))]
    Existed { id: Cow<'a, Id> },
    #[serde(rename(serialize = "wip"))]
    Progress { id: Cow<'a, Id>, progress: u16 },
    #[serde(rename(serialize = "ok"))]
    Ok { id: Cow<'a, Id> },
}

impl<'a> VideoUploadEvent<'a> {
    pub(crate) fn existed(id: &'a Id) -> Self {
        Self::Existed {
            id: Cow::Borrowed(id),
        }
    }

    pub(crate) fn wip(id: &'a Id, progress: u16) -> Self {
        Self::Progress {
            id: Cow::Borrowed(id),
            progress,
        }
    }

    pub(crate) fn ok(id: &'a Id) -> Self {
        Self::Ok {
            id: Cow::Borrowed(id),
        }
    }
}


impl From<VideoUploadEvent<'_>> for FormatedEvent {
    fn from(value: VideoUploadEvent<'_>) -> Self {
        match value {
            VideoUploadEvent::Existed { id } => Self {
                id: id.to_string(),
                progress: -1,
                state: "already_existed".to_string(),
            },
            VideoUploadEvent::Progress { id, progress } => Self {
                id: id.to_string(),
                progress: progress as i16,
                state: "wip".to_string(),
            },
            VideoUploadEvent::Ok { id } => Self {
                id: id.to_string(),
                progress: 100,
                state: "ok".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use crate::common::FormatedEvent;
    use crate::material::biz::VideoUploadEvent;
    use crate::material::storage::Id;
    use serde_json::{json, Value};

    #[test]
    fn test_already_existed_event() -> anyhow::Result<()> {
        let test: FormatedEvent = VideoUploadEvent::existed(&Id("test".to_string())).into();

        let string = serde_json::to_string(&test)?;
        let json: Value = serde_json::from_str(&string)?;

        assert_eq!(
            json,
            json!(
                {
                    "id": "test",
                    "progress": -1,
                    "state": "already_existed"
                }
            )
        );

        Ok(())
    }

    #[test]
    fn test_already_progress() -> anyhow::Result<()> {
        let test: FormatedEvent = VideoUploadEvent::wip(&Id("test".to_string()), 16).into();

        let string = serde_json::to_string(&test)?;
        let json: Value = serde_json::from_str(&string)?;

        assert_eq!(
            json,
            json!(
                {
                    "id": "test",
                    "progress": 16,
                    "state": "wip"
                }
            )
        );

        Ok(())
    }

    #[test]
    fn test_ok() -> anyhow::Result<()> {
        let test: FormatedEvent = VideoUploadEvent::ok(&Id("test".to_string())).into();

        let string = serde_json::to_string(&test)?;
        let json: Value = serde_json::from_str(&string)?;

        assert_eq!(
            json,
            json!(
                {
                    "id": "test",
                    "progress": 100,
                    "state": "ok"
                }
            )
        );

        Ok(())
    }
}

#[derive(Bean)]
pub(crate) struct MaterialsService {
    #[inject(bean)]
    storage: &'static LocalStorage,
    #[inject(bean)]
    repo: &'static MaterialsRepo,
    #[inject(bean)]
    ffmpeg: &'static FFmpegUtils,
}

#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct MaterialVideo {
    id: Id,
    name: String,
    raw: String,
    thumbnail: String,
    description: String,
}

#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct VideoSlices {
    slice: String,
    slice720p: String,
    slice1080p: String,
}

#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct MaterialDetail {
    video: MaterialVideo,
    #[serde(flatten)]
    slices: VideoSlices,
}

impl MaterialsService {
    pub(crate) async fn upload(
        &self,
        upload: UploadPayload,
        tx: Sender<FormatedEvent>,
        claims: Claims,
    ) -> Result<()> {
        let file_name = upload.file.file_name().unwrap_or("no_name").to_string();
        let raw_file = upload.file.into_file();

        match self.storage.save(raw_file).await? {
            SavedId::Existed(id) => {
                warn!("file {file_name} is existed! return id {id}!");
                tx.send(VideoUploadEvent::existed(&id).into()).await?;
            }
            SavedId::New(id) => {
                info!("new file {file_name} with id {id}");
                tx.send(VideoUploadEvent::wip(&id, 15).into()).await?;

                let raw = self.storage.raw_file(&id).await?;
                let thumbnail_assert = self.storage.assert_file(&id, "thumbnail.jpeg").await?;

                let raw_thumbnail = raw.clone();
                let ffmpeg = self.ffmpeg;
                spawn_blocking(move || -> Result<()> {
                    ffmpeg.thumbnail(&raw_thumbnail, &thumbnail_assert)?;
                    Ok(())
                })
                .await??;

                info!("save thumbnail: {file_name} with id {id}");
                tx.send(VideoUploadEvent::wip(&id, 25).into()).await?;

                let slice_raw = raw.clone();
                spawn_blocking(move || -> Result<()> {
                    //todo
                    ffmpeg.slice(&slice_raw, slice_raw.parent().expect("not here"))?;
                    Ok(())
                })
                .await??;

                info!("save slice: {file_name} with id {id}");
                tx.send(VideoUploadEvent::wip(&id, 75).into()).await?;

                let materials =
                    Materials::new_video(id.to_string(), file_name, upload.desc, claims.id);
                let tags = upload.tags.as_ref().map(|tags| tags.as_slice());
                self.repo.save(&materials, tags).await?;

                tx.send(VideoUploadEvent::ok(&id).into()).await?;
            }
        }

        Ok(())
    }

    pub(crate) async fn detail(
        &self,
        id: Id,
        base_url: BaseUrl,
        _claims: Claims,
    ) -> Result<MaterialDetail> {
        if !self.storage.exists(&id).await? {
            return Err(AppError::MaterialNotFound(id.to_string()));
        }

        let materials = self.repo.get(&id).await?;

        let slices = VideoSlices {
            slice: self.storage.url(&base_url, &id, "slice.m3u8")?.to_string(),
            slice720p: self
                .storage
                .url(&base_url, &id, "720p/slice.m3u8")?
                .to_string(),
            slice1080p: self
                .storage
                .url(&base_url, &id, "1080p/slice.m3u8")?
                .to_string(),
        };

        let video = MaterialVideo {
            id: Id(materials.id),
            name: materials.name.unwrap_or("".to_string()),
            raw: self.storage.url(&base_url, &id, "raw")?.to_string(),
            thumbnail: self
                .storage
                .url(&base_url, &id, "thumbnail.jpeg")?
                .to_string(),
            description: materials.description.unwrap_or("".to_string()),
        };

        let detail = MaterialDetail { slices, video };

        Ok(detail)
    }

    pub(crate) async fn delete(&self, id: Id, _claims: Claims) -> Result<()> {
        if !self.storage.exists(&id).await? {
            return Err(AppError::MaterialNotFound(id.to_string()));
        }
        self.storage.delete(&id).await?;

        self.repo.delete(&id).await?;
        Ok(())
    }
}

#[derive(Bean)]
pub struct MaterialsRepo {
    #[inject(bean = Db)]
    db: &'static SqlitePool,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Object)]
pub(crate) struct Materials {
    id: String,
    name: Option<String>,
    description: Option<String>,
    creator: String,
    state: i64,
    r#type: i64,
    created_at: NaiveDateTime,
}

impl Materials {
    pub(crate) fn new_video(
        id: String,
        name: String,
        description: Option<String>,
        creator: String,
    ) -> Self {
        Self {
            id,
            name: Some(name),
            description,
            creator,
            state: STATE_OK as i64,
            r#type: TYPE_VIDEO as i64,
            created_at: Utc::now().naive_utc(),
        }
    }
}

impl MaterialsRepo {
    async fn save(&self, materials: &Materials, tags: Option<&[String]>) -> Result<()> {
        let mut tx = self.db.begin().await?;

        let result = sqlx::query!(
            r#"
            INSERT INTO materials (id, name, description, creator, state, type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            materials.id,
            materials.name,
            materials.description,
            materials.creator,
            materials.state,
            materials.r#type,
            materials.created_at
        )
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::DbError("save materials failed".to_string()));
        }

        if let Some(tags) = tags {
            for tag in tags {
                sqlx::query!(
                    r#"
                    INSERT INTO material_tags (material_id, tag, created_at)
                    VALUES (?, ?, ?)
                    "#,
                    materials.id,
                    tag,
                    materials.created_at
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        Ok(())
    }

    async fn get(&self, id: &Id) -> Result<Materials> {
        let materials = sqlx::query_as(
            r#"
            SELECT id, name, description, creator, state, type, created_at
            FROM materials
            WHERE id = ?
            "#,
        )
        .bind(id.as_ref())
        .fetch_one(self.db)
        .await?;

        Ok(materials)
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        let mut tx = self.db.begin().await?;

        let id_str = id.deref();

        sqlx::query!(
            r#"
            DELETE FROM materials WHERE id = ?
            "#,
            id_str
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            DELETE FROM material_tags WHERE material_id = ?
            "#,
            id_str
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}
