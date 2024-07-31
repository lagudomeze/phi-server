use crate::common::Result;
use crate::material::storage::{Id, LocalStorage, Storage};
use ioc::{mvc, Bean};
use poem::web::Field;
use poem_openapi::param::Path;
use poem_openapi::{
    payload::Json,
    types::multipart::Upload,
    types::{ParseFromMultipartField, ParseResult},
    Multipart, NewType,
};

#[derive(NewType, Debug)]
#[oai(to_header = false, from_multipart = false)]
pub(crate) struct Tags(Vec<String>);

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
    file: Upload,
    tags: Option<Tags>,
    desc: Option<String>,
}

#[derive(Bean)]
pub(crate) struct UploadMvc {
    #[inject(bean)]
    storage: &'static LocalStorage,
}

#[mvc]
impl UploadMvc {
    /// Upload file
    #[oai(path = "/materials/video", method = "post")]
    async fn upload(&self, upload: UploadPayload) -> Result<Json<Id>> {
        let mut file = upload.file.into_file();

        let id = self.storage.save(&mut file).await?;

        Ok(Json(id))
    }

    #[oai(path = "/materials/:id", method = "head")]
    async fn exists(&self, id: Path<Id>) -> Result<Json<bool>> {
        let existed = self.storage.exists(&id).await?;
        Ok(Json(existed))
    }
}
