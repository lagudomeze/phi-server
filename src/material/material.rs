use chrono::{NaiveDateTime, Utc};
use ioc::Bean;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::{sync::mpsc::Sender, task::spawn_blocking};
use tracing::{info, warn};

use crate::common::{AppError, FormatedEvent};
use crate::material::{STATE_OK, TYPE_VIDEO};
use crate::{
    common::Result,
    db::Db,
    ffmpeg::slice::slice,
    ffmpeg::thumbnail::thumbnail,
    material::storage::SavedId,
    material::{
        storage::{Id, LocalStorage, Storage},
        upload::UploadPayload,
    },
};
use crate::auth::jwt::Claims;

#[derive(Serialize, Deserialize)]
#[serde(tag = "state")]
pub(crate) enum VideoUploadEvent {
    #[serde(rename(serialize = "already_existed"))]
    Existed { id: Id},
    #[serde(rename(serialize = "wip"))]
    Progress { id: Id, progress: u16 },
    #[serde(rename(serialize = "ok"))]
    Ok { id: Id },
}

impl VideoUploadEvent {
    pub(crate) fn existed(id: Id) -> Self {
        Self::Existed { id }
    }

    pub(crate) fn wip(id: Id, progress: u16) -> Self {
        Self::Progress { id, progress }
    }

    pub(crate) fn ok(id: Id) -> Self {
        Self::Ok { id}
    }
}

impl Into<FormatedEvent> for VideoUploadEvent {
    fn into(self) -> FormatedEvent {
        match self {
            VideoUploadEvent::Existed { id } => {
                FormatedEvent {
                    id,
                    progress: -1,
                    state: "already_existed".to_string()
                }
            }
            VideoUploadEvent::Progress { id, progress } => {
                FormatedEvent {
                    id,
                    progress: progress as i16,
                    state: "wip".to_string()
                }
            }
            VideoUploadEvent::Ok { id } => {
                FormatedEvent {
                    id,
                    progress: 100,
                    state: "ok".to_string()
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::common::FormatedEvent;
    use crate::material::material::VideoUploadEvent;
    use crate::material::storage::Id;
    use serde_json::{json, Value};

    #[test]
    fn test_already_existed_event() -> anyhow::Result<()> {
        let test : FormatedEvent = VideoUploadEvent::existed(Id("test".to_string())).into();

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
        let test : FormatedEvent = VideoUploadEvent::wip(Id("test".to_string()), 16).into();

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
        let test : FormatedEvent = VideoUploadEvent::ok(Id("test".to_string())).into();

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
                tx.send(VideoUploadEvent::existed(id).into()).await?;
            }
            SavedId::New(id) => {
                info!("new file {file_name} with id {id}");
                tx.send(VideoUploadEvent::wip(id.clone(), 15).into()).await?;

                let raw = self.storage.raw_file(&id).await?;
                let thumbnail_assert = self.storage.assert_file(&id, "thumbnail.jpeg").await?;

                let raw_thumbnail = raw.clone();
                spawn_blocking(move || -> Result<()> {
                    thumbnail(&raw_thumbnail, &thumbnail_assert)?;
                    Ok(())
                })
                .await??;

                info!("save thumbnail: {file_name} with id {id}");
                tx.send(VideoUploadEvent::wip(id.clone(), 25).into()).await?;

                let slice_raw = raw.clone();
                spawn_blocking(move || -> Result<()> {
                    //todo
                    slice(&slice_raw, slice_raw.parent().expect("not here"))?;
                    Ok(())
                })
                .await??;

                info!("save slice: {file_name} with id {id}");
                tx.send(VideoUploadEvent::wip(id.clone(), 75).into()).await?;

                let materials = Materials::new_video(
                    id.to_string(),
                    file_name,
                    upload.desc.unwrap_or("".to_string()),
                    claims.id,
                );
                let tags = upload.tags
                    .as_ref()
                    .map(|tags| tags.as_slice());
                self.repo.save(&materials, tags).await?;

                tx.send(VideoUploadEvent::ok(id.clone()).into()).await?;
            }
        }

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
    name: String,
    description: String,
    creator: String,
    state: u16,
    r#type: u16,
    created_at: NaiveDateTime,
}

impl Materials {
    pub(crate) fn new_video(
        id: String,
        name: String,
        description: String,
        creator: String,
    ) -> Self {
        Self {
            id,
            name,
            description,
            creator,
            state: STATE_OK,
            r#type: TYPE_VIDEO,
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
        ).execute(&mut *tx)
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
                ).execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;

        Ok(())
    }
}
