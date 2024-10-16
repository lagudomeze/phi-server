use crate::common::AppError::WrongMaterialType;
use crate::ffmpeg::slice::SliceEvent;
use crate::material::mvc::{ImagesUploadPayload, MaterialPatchRequest};
use crate::{
    auth::jwt::Claims,
    common::{AppError, FormatedEvent, PageResult, Result},
    db::Db,
    ffmpeg::common::FFmpegUtils,
    material::{
        mvc::{SearchCondition, UploadPayload},
        storage::{Id, LocalStorage, SavedId, Storage},
        STATE_OK, TYPE_IMAGE, TYPE_VIDEO,
    },
    util::poem::BaseUrl,
};
use chrono::{NaiveDateTime, Utc};
use ioc::Bean;
use poem_openapi::{Object, Union};
use serde::{Deserialize, Serialize};
use sqlx::{query_as_with, query_scalar_with, Arguments, QueryBuilder, SqlitePool};
use std::{borrow::Cow, ops::Deref};
use tokio::{sync::mpsc::Sender, task::spawn_blocking};
use tracing::{debug, info, warn};

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
pub(crate) struct MaterialVideoDetail {
    #[serde(flatten)]
    #[oai(flatten)]
    video: MaterialVideo,
    #[serde(flatten)]
    #[oai(flatten)]
    slices: VideoSlices,
}

#[derive(Serialize, Deserialize, Debug, Union)]
#[oai(discriminator_name = "type")]
pub(crate) enum MaterialDetail {
    Video(MaterialVideoDetail),
    Image(MaterialImage),
}

#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct MaterialImage {
    id: Id,
    name: String,
    raw: String,
    description: String,
}

impl MaterialsService {
    pub(crate) async fn search(
        &self,
        condition: SearchCondition,
        base_url: BaseUrl,
        claims: Claims,
    ) -> Result<PageResult<MaterialDetail>> {
        let result = self.repo.search(&condition, &claims.id).await?;
        result.transfer(|material| self.transfer(&base_url, material))
    }

    pub(crate) async fn upload(
        &self,
        upload: UploadPayload,
        tx: Sender<FormatedEvent>,
        claims: Claims,
    ) -> Result<()> {
        let file_name = upload.file.file_name().unwrap_or("no_name").to_string();
        let raw_file = upload.file.into_file();

        let id = Id::new_uuid();

        match self.storage.save(&id, raw_file).await? {
            SavedId::Existed => {
                warn!("file {file_name} is existed! return id {id}!");
                tx.send(VideoUploadEvent::existed(&id).into()).await?;
            }
            SavedId::New => {
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
                let mut rx = ffmpeg.slice2(&slice_raw, slice_raw.parent().expect("not here")).await?;

                let mut progress = 26;

                while let Some(event) = rx.recv().await {
                    match event {
                        SliceEvent::Ok => {
                            info!("save slice: {file_name} with id {id}");
                            tx.send(VideoUploadEvent::wip(&id, 75).into()).await?;
                        }
                        SliceEvent::Wip(e) => {
                            debug!("ffmpeg {e:?}");
                            tx.send(VideoUploadEvent::wip(&id, progress).into()).await?;
                            if progress < 75 {
                                progress += 1;
                            }
                        }
                        SliceEvent::Err(error) => {
                            return Err(error.into());
                        }
                    }
                }

                let materials =
                    Material::new_video(id.to_string(), file_name, upload.desc, claims.id);
                let tags = upload.tags.as_ref().map(|tags| tags.as_slice());
                self.repo.save(&materials, tags).await?;

                tx.send(VideoUploadEvent::ok(&id).into()).await?;
            }
        }

        Ok(())
    }

    pub(crate) async fn upload_image(
        &self,
        upload: ImagesUploadPayload,
        base_url: BaseUrl,
        claims: Claims,
    ) -> Result<Vec<MaterialImage>> {
        let mut details = Vec::with_capacity(upload.files.capacity());

        for file in upload.files {
            let file_name = file.file_name().unwrap_or("no_name").to_string();
            let raw_file = file.into_file();
            let id = Id::new_uuid();

            let detail = match self.storage.save(&id, raw_file).await? {
                SavedId::Existed => {
                    warn!("file {file_name} is existed! return id {id}!");
                    let material = self.repo.get(&id).await?;
                    self.transfer_image(&base_url, material)
                }
                SavedId::New => {
                    info!("new image file {file_name} with id {id}");
                    let material = Material::new_image(id.to_string(), file_name, upload.desc.clone(), claims.id.clone());
                    let tags = upload.tags.as_ref().map(|tags| tags.as_slice());
                    self.repo.save(&material, tags).await?;
                    self.transfer_image(&base_url, material)
                }
            }?;
            details.push(detail);
        }

        Ok(details)
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

        let material = self.repo.get(&id).await?;

        self.transfer(&base_url, material)
    }

    fn transfer_video(&self, base_url: &BaseUrl, material: Material) -> Result<MaterialVideoDetail> {
        let id = Id(material.id);
        let slices = VideoSlices {
            slice: self.storage.url(base_url, &id, "slice.m3u8")?.to_string(),
            slice720p: self
                .storage
                .url(base_url, &id, "720p/slice.m3u8")?
                .to_string(),
            slice1080p: self
                .storage
                .url(base_url, &id, "1080p/slice.m3u8")?
                .to_string(),
        };

        let raw = self.storage.url(base_url, &id, "raw")?.to_string();

        let thumbnail = self
            .storage
            .url(base_url, &id, "thumbnail.jpeg")?
            .to_string();

        let video = MaterialVideo {
            id,
            name: material.name.unwrap_or("".to_string()),
            raw,
            thumbnail,
            description: material.description.unwrap_or("".to_string()),
        };

        let detail = MaterialVideoDetail { slices, video };
        Ok(detail)
    }

    fn transfer_image(&self, base_url: &BaseUrl, material: Material) -> Result<MaterialImage> {
        let id = Id(material.id);
        let raw = self.storage.url(base_url, &id, "raw")?.to_string();

        let detail = MaterialImage {
            id,
            name: material.name.unwrap_or("".to_string()),
            raw,
            description: material.description.unwrap_or("".to_string()),
        };
        Ok(detail)
    }

    fn transfer(&self, base_url: &BaseUrl, material: Material) -> Result<MaterialDetail> {
        let detail = match material.r#type as u16 {
            TYPE_VIDEO => {
                let detail = self.transfer_video(base_url, material)?;
                MaterialDetail::Video(detail)
            }
            TYPE_IMAGE => {
                let detail = self.transfer_image(base_url, material)?;
                MaterialDetail::Image(detail)
            }
            unexpected => {
                return Err(WrongMaterialType(unexpected))
            }
        };

        Ok(detail)
    }

    pub(crate) async fn update(&self, id: Id, _claims: Claims, request: MaterialPatchRequest) -> Result<()> {
        self.repo.update_name(&id, &request.name).await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, id: Id, _claims: Claims) -> Result<()> {
        if !self.storage.exists(&id).await? {
            return Err(AppError::MaterialNotFound(id.to_string()));
        }
        self.storage.delete(&id).await?;

        self.repo.delete(&id).await?;
        Ok(())
    }

    pub(crate) async fn batch_delete(&self, ids: Vec<Id>, _claims: Claims) -> Result<()> {
        for id in ids.iter() {
            if self.storage.exists(id).await? {
                self.storage.delete(id).await?;
            }
        }
        self.repo.delete_by_ids(&ids).await?;
        Ok(())
    }
}

#[derive(Bean)]
pub struct MaterialsRepo {
    #[inject(bean = Db)]
    db: &'static SqlitePool,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Object)]
pub(crate) struct Material {
    id: String,
    name: Option<String>,
    raw_name: Option<String>,
    description: Option<String>,
    creator: String,
    state: i64,
    r#type: i64,
    created_at: NaiveDateTime,
}

impl Material {
    pub(crate) fn new_video(
        id: String,
        name: String,
        description: Option<String>,
        creator: String,
    ) -> Self {
        Self {
            id,
            name: Some(name.clone()),
            raw_name: Some(name),
            description,
            creator,
            state: STATE_OK as i64,
            r#type: TYPE_VIDEO as i64,
            created_at: Utc::now().naive_utc(),
        }
    }
    pub(crate) fn new_image(
        id: String,
        name: String,
        description: Option<String>,
        creator: String,
    ) -> Self {
        Self {
            id,
            name: Some(name.clone()),
            raw_name: Some(name),
            description,
            creator,
            state: STATE_OK as i64,
            r#type: TYPE_IMAGE as i64,
            created_at: Utc::now().naive_utc(),
        }
    }
}

impl MaterialsRepo {
    async fn search(
        &self,
        condition: &SearchCondition,
        creator: impl AsRef<str>,
    ) -> Result<PageResult<Material>> {
        let mut sql_select_args = sqlx::sqlite::SqliteArguments::default();
        let mut sql_count_args = sqlx::sqlite::SqliteArguments::default();

        let sql_select =
            "SELECT id, name, raw_name, description, creator, state, type, created_at FROM materials";

        let sql_count = "SELECT COUNT(*) FROM materials";

        let mut sql_where = " WHERE creator = ?".to_string();

        sql_select_args.add(creator.as_ref())?;
        sql_count_args.add(creator.as_ref())?;

        if let Some(ref tags) = condition.tags {
            if !tags.is_empty() {
                sql_where
                    .push_str(" AND id IN (SELECT material_id FROM material_tags WHERE tag IN (");
                for tag in tags {
                    sql_select_args.add(tag)?;
                    sql_count_args.add(tag)?;
                    sql_where.push_str("?,");
                }
                sql_where.pop();
                sql_where.push_str("))");
            }
        }

        if let Some(ref query) = condition.query {
            sql_where.push_str(" AND (name LIKE ? or description LIKE ?)");
            sql_select_args.add(format!("%{query}%"))?;
            sql_select_args.add(format!("%{query}%"))?;
            sql_count_args.add(format!("%{query}%"))?;
            sql_count_args.add(format!("%{query}%"))?;
        }

        if let Some(ref material_type) = condition.r#type {
            sql_where.push_str(" AND type = ?");
            sql_select_args.add(material_type.value())?;
            sql_count_args.add(material_type.value())?;
        }

        let total: u64 = query_scalar_with(&format!("{sql_count}{sql_where}"), sql_count_args)
            .fetch_one(self.db)
            .await?;

        let sql_order = " ORDER BY created_at DESC";

        let sql_limit = " LIMIT ? OFFSET ?";

        sql_select_args.add(condition.page.limit())?;
        sql_select_args.add(condition.page.offset())?;

        let records: Vec<Material> = query_as_with(
            &format!("{sql_select}{sql_where}{sql_order}{sql_limit}"),
            sql_select_args,
        )
            .fetch_all(self.db)
            .await?;

        Ok(PageResult::new(&condition.page, total, records))
    }

    async fn save(&self, materials: &Material, tags: Option<&[String]>) -> Result<()> {
        let mut tx = self.db.begin().await?;

        let result = sqlx::query!(
            r#"
            INSERT INTO materials (id, name, raw_name, description, creator, state, type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            materials.id,
            materials.name,
            materials.raw_name,
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

    async fn get(&self, id: &Id) -> Result<Material> {
        let materials = sqlx::query_as(
            r#"
            SELECT id, name, raw_name, description, creator, state, type, created_at
            FROM materials
            WHERE id = ?
            "#,
        )
            .bind(id.as_ref())
            .fetch_one(self.db)
            .await?;

        Ok(materials)
    }

    async fn update_name(&self, id: &Id, name: &str) -> Result<()> {
        let id_str = id.deref();

        sqlx::query!(
            r#"
            UPDATE materials SET name = ? WHERE id = ?
            "#,
            name,
            id_str
        ).execute(self.db).await?;

        Ok(())
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

    async fn delete_by_ids(&self, ids: &[Id]) -> Result<()> {
        let mut tx = self.db.begin().await?;

        QueryBuilder::new("DELETE FROM materials WHERE id IN")
            .push_tuples(ids.iter(), |mut b, id| {
                b.push_bind(id.deref());
            })
            .build()
            .execute(&mut *tx)
            .await?;

        QueryBuilder::new("DELETE FROM material_tags WHERE material_id IN")
            .push_tuples(ids.iter(), |mut b, id| {
                b.push_bind(id.deref());
            })
            .build()
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}
