use poem::{Error, http::StatusCode};
use poem_openapi::{
    ApiResponse,
    Object,
    payload::Json,
    registry::{MetaResponses, Registry},
    types::{ParseFromJSON, ToJSON},
    types::Type
};
use serde::Serialize;
use thiserror::Error;
use std::io;

pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("sqlx error: `{0}`")]
    DbSqlxError(#[from] sqlx::Error),
    #[error("io error: `{0}`")]
    IoError(#[from] io::Error),
    #[error("ioc error: `{0}`")]
    IocError(#[from] ioc::IocError),
    #[error("poem error: `{0}`")]
    PoemError(poem::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<poem::Error> for AppError {
    fn from(value: Error) -> Self {
        Self::PoemError(value)
    }
}

impl Into<poem::Error> for AppError {
    fn into(self) -> poem::Error {
        Error::new(self, StatusCode::INTERNAL_SERVER_ERROR)
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

#[derive(Object, Serialize)]
pub struct ResponseBody<T: Type + ParseFromJSON + ToJSON + Serialize> {
    code: i32,
    msg: String,
    data: Option<T>,
}

impl<T: Type + ParseFromJSON + ToJSON + Serialize> ResponseBody<T> {
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
#[oai]
pub enum Response<T: Type + ParseFromJSON + ToJSON + Serialize> {
    #[oai(status = 200)]
    Ok(Json<ResponseBody<T>>),
}


impl<T> Response<T>
where
    T: Type + ParseFromJSON + ToJSON + Serialize,
{
    pub fn ok(data: T) -> Self {
        Self::Ok(Json(ResponseBody::ok(data)))
    }
}