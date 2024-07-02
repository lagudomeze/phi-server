use poem::{
    http::StatusCode,
    web::Json
};
use poem_openapi::{
    ApiResponse, Object,
    registry::{MetaResponses, Registry},
    ResponseContent,
    types::{ParseFromJSON, ToJSON},
};
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("sqlx error: `{0}`")]
    DbSqlxError(#[from] sqlx::Error),
    #[error("ioc error: `{0}`")]
    IocError(#[from] ioc::IocError),
    #[error("poem error: `{0}`")]
    PoemError(poem::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Into<poem::Error> for AppError {
    fn into(self) -> poem::Error {
        poem::Error::new(self, StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl ApiResponse for AppError {
    fn meta() -> MetaResponses {
        MetaResponses {
            responses: Vec::new(),
        }
    }

    fn register(_: &mut Registry) {}
}

#[derive(Object)]
pub struct ResponseBody<T: ParseFromJSON + ToJSON + Send + Sync> {
    code: i32,
    msg: String,
    data: Option<T>,
}

impl<T: ParseFromJSON + ToJSON + Send + Sync> ResponseBody<T> {
    pub fn ok(data: T) -> Self {
        Self::new(0, "OK".to_string(), data)
    }

    pub fn new(code: i32,
               msg: String,
               data: T) -> Self {
        Self {
            code,
            msg,
            data: Some(data),
        }
    }
}

#[derive(ApiResponse)]
pub enum Response<T: ParseFromJSON + ToJSON + Send + Sync> {
    #[oai(status = 200)]
    Ok(Json<ResponseBody<T>>),
}

impl<T> Response<T>
where
    T: ParseFromJSON + ToJSON + Send + Sync,
{
    pub fn ok(data: T) -> Self {
        Self::Ok(Json(ResponseBody::ok(data)))
    }
}