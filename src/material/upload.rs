use std::ops::Deref;
use ioc::{mvc, Bean, OpenApi};
use poem::web::Field;
use poem_openapi::{
    param::Path,
    payload::EventStream,
    payload::Json,
    types::{multipart::Upload, ParseFromMultipartField, ParseResult},
    Multipart, NewType,
};
use tokio::{sync::mpsc::channel, task::spawn};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    common::FormatedEvent,
    common::{Response, Result},
    material::{
        material::MaterialsService,
        storage::{Id, LocalStorage, Storage},
    },
};
use crate::auth::apikey::JwtAuth;

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

#[derive(Bean)]
pub(crate) struct UploadMvc {
    #[inject(bean)]
    storage: &'static LocalStorage,
    #[inject(bean)]
    materials_svc: &'static MaterialsService,
}

#[mvc]
#[OpenApi(prefix_path = "/api/v1")]
impl UploadMvc {
    //todo
    #[oai(path = "/materials:search", method = "post")]
    async fn search(&self, upload: UploadPayload, _auth: JwtAuth) -> Result<Json<Id>> {
        let mut file = upload.file.into_file();

        let id = self.storage.save(&mut file).await?;

        Ok(Json(id.into()))
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
    async fn detail(&self, id: Path<Id>, _auth: JwtAuth) -> Result<Json<Id>> {
        Ok(Json(id.0))
    }

    /// Upload file
    #[oai(path = "/materials/video", method = "post")]
    async fn upload(&self, upload: UploadPayload, auth: JwtAuth) -> EventStream<ReceiverStream<FormatedEvent>> {
        let (tx, rx) = channel(32);

        let _detached = spawn(self.materials_svc.upload(upload, tx, auth.into()));

        EventStream::new(ReceiverStream::new(rx))
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
