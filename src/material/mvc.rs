use crate::{
    auth::apikey::JwtAuth,
    common::{FormatedEvent, Page, PageResult, Response, Result},
    material::{
        MaterialType,
        biz::{
            MaterialDetail,
            MaterialsService
        },
        storage::{Id, LocalStorage, Storage}
    },
    util::poem::BaseUrl,
};
use ioc::{mvc, Bean, OpenApi};
use poem::web::Field;
use poem_openapi::{
    param::Path,
    payload::EventStream,
    payload::Json,
    types::{multipart::Upload, ParseFromMultipartField, ParseResult},
    Multipart, NewType, Object,
};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tokio::{sync::mpsc::channel, task::spawn};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;
use crate::material::biz::MaterialImage;

#[derive(NewType, Debug)]
#[oai(to_header = false, from_multipart = false)]
pub(crate) struct Tags(Vec<String>);

impl Deref for Tags {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ParseFromMultipartField for Tags {
    async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
        if let Some(field) = field {
            let tags = field
                .text()
                .await?
                .split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>();
            Ok(Tags(tags))
        } else {
            Ok(Tags(Vec::new()))
        }
    }
}

#[derive(Debug, Multipart)]
pub(crate) struct UploadPayload {
    pub(crate) file: Upload,
    pub(crate) tags: Option<Tags>,
    pub(crate) desc: Option<String>,
}

#[derive(Debug, Multipart)]
pub(crate) struct ImagesUploadPayload {
    pub(crate) files: Vec<Upload>,
    pub(crate) tags: Option<Tags>,
    pub(crate) desc: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Object)]
pub(crate) struct SearchCondition {
    pub(crate) tags: Option<Vec<String>>,
    #[serde(flatten)]
    #[oai(flatten = true)]
    pub(crate) page: Page,
    pub(crate) query: Option<String>,
    pub(crate) r#type: Option<MaterialType>,
}

#[derive(Bean)]
pub(crate) struct MaterialMvc {
    #[inject(bean)]
    storage: &'static LocalStorage,
    #[inject(bean)]
    materials_svc: &'static MaterialsService,
}

#[mvc]
#[OpenApi(prefix_path = "/api/v1")]
impl MaterialMvc {
    #[oai(path = "/materials:search", method = "post")]
    async fn search(
        &self,
        condition: Json<SearchCondition>,
        base_url: BaseUrl,
        auth: JwtAuth,
    ) -> Result<Response<PageResult<MaterialDetail>>> {
        info!("{:?}", condition);

        let result = self
            .materials_svc
            .search(condition.0, base_url, auth.into())
            .await?;

        Ok(Response::ok(result))
    }

    #[oai(path = "/materials/:id", method = "head")]
    async fn exists(&self, id: Path<Id>, _auth: JwtAuth) -> Result<Response<bool>> {
        if self.storage.exists(&id).await? {
            Ok(Response::ok(true))
        } else {
            Ok(Response::not_found())
        }
    }

    #[oai(path = "/materials/:id", method = "get")]
    async fn detail(
        &self,
        id: Path<Id>,
        base_url: BaseUrl,
        auth: JwtAuth,
    ) -> Result<Response<MaterialDetail>> {
        let detail = self
            .materials_svc
            .detail(id.0, base_url, auth.into())
            .await?;
        Ok(Response::ok(detail))
    }

    #[oai(path = "/materials/:id", method = "delete")]
    async fn delete(&self, id: Path<Id>, auth: JwtAuth) -> Result<Response<String>> {
        self.materials_svc.delete(id.0, auth.into()).await?;
        Ok(Response::ok("ok".to_string()))
    }

    /// Upload  video file
    #[oai(path = "/materials/video", method = "post")]
    async fn upload(
        &self,
        upload: UploadPayload,
        auth: JwtAuth,
    ) -> EventStream<ReceiverStream<FormatedEvent>> {
        let (tx, rx) = channel(32);

        let _detached = spawn(self.materials_svc.upload(upload, tx, auth.into()));

        EventStream::new(ReceiverStream::new(rx))
    }

    /// Upload image file
    #[oai(path = "/materials/image", method = "post")]
    async fn upload_image(
        &self,
        upload: ImagesUploadPayload,
        base_url: BaseUrl,
        auth: JwtAuth,
    ) -> Result<Response<Vec<MaterialImage>>> {
        let detail = self.materials_svc.upload_image(upload, base_url, auth.into()).await?;
        Ok(Response::ok(detail))
    }
}

#[cfg(test)]
mod test {
    use poem_openapi::payload::EventStream;
    use tokio::spawn;
    use tokio::sync::mpsc::channel;
    use tokio_stream::wrappers::ReceiverStream;

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let (tx, rx) = channel::<u64>(32);

        let _sse = EventStream::new(ReceiverStream::new(rx));

        let a: tokio::task::JoinHandle<Result<(), anyhow::Error>> = spawn(async move {
            tx.send(1).await?;
            tx.send(2).await?;
            tx.send(3).await?;
            tx.send(4).await?;
            Ok(())
        });

        println!("{:?}", a.await??);

        Ok(())
    }
}
