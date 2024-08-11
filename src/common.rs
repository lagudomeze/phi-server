use std::{fmt::Display, io, panic::Location};

use anyhow::Context;
use poem::{error::ResponseError, http::StatusCode, Body, Error, Response as PoemResponse};
use poem_openapi::{
    payload::Json,
    registry::{MetaResponses, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    ApiResponse, Object, Tags,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio::task::JoinError;

use crate::material::storage::Id;

#[derive(Tags)]
pub(crate) enum PhiTags {
    Auth,
}

pub trait LocationContext<T, E>: Context<T, E> {
    fn location<C>(self, context: C, location: &Location) -> anyhow::Result<T>
    where
        C: Display + Send + Sync + 'static,
        Self: Sized,
    {
        self.context(format!("`{context}` at `{location}`"))
    }
}

impl<T, E, C> LocationContext<T, E> for C where C: Context<T, E> {}

pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("sqlx error: `{0}`")]
    DbSqlxError(#[from] sqlx::Error),
    #[error("join error: `{0}`")]
    JoinError(#[from] JoinError),
    #[error("material not found: `{0}`")]
    MaterialNotFound(Id),
    #[error("video upload event send error: `{0}`")]
    SseError(
        #[from]
        #[source]
        SendError<FormatedEvent>,
    ),
    #[error("parse float error: `{0}`")]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error("io error: `{0}`")]
    IoError(#[from] io::Error),
    #[error("ioc error: `{0}`")]
    IocError(#[from] ioc::IocError),
    #[error("poem error: `{0}`")]
    PoemError(poem::Error),
    #[error("http error: `{0}`")]
    HttpError(#[from] http::Error),
    #[error("invalid uri: `{0}`")]
    InvalidUri(#[from] http::uri::InvalidUri),
    #[error("url parse error: `{0}`")]
    UrlParseError(#[from] url::ParseError),
    #[error("reqwest error: `{0:?}`")]
    ReqwestError(#[from] reqwest::Error),
    #[error("jwt error: `{0}`")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    #[error("ring error: `{0}`")]
    UnspecifiedRingError(#[from] ring::error::Unspecified),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<poem::Error> for AppError {
    fn from(value: Error) -> Self {
        Self::PoemError(value)
    }
}

impl ResponseError for AppError {
    fn status(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn as_response(&self) -> PoemResponse {
        let body = Body::from_json(serde_json::json!({
            "code": 500,
            "msg": format!("{self}"),
        }))
        .unwrap();
        PoemResponse::builder().status(self.status()).body(body)
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

#[derive(Object, Serialize, Deserialize)]
pub(crate) struct FormatedEvent {
    pub(crate) id: Id,
    pub(crate) progress: i16,
    pub(crate) state: String,
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

    pub fn new(code: i32, msg: String, data: T) -> Self {
        Self {
            code,
            msg,
            data: Some(data),
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: 404,
            msg: "not found".to_string(),
            data: None,
        }
    }
}

#[derive(ApiResponse)]
#[oai]
pub enum Response<T: Type + ParseFromJSON + ToJSON + Serialize> {
    #[oai(status = 200)]
    Ok(Json<ResponseBody<T>>),
    #[oai(status = 404)]
    NotFound(Json<ResponseBody<T>>),
}

impl<T> Response<T>
where
    T: Type + ParseFromJSON + ToJSON + Serialize,
{
    pub fn ok(data: T) -> Self {
        Self::Ok(Json(ResponseBody::ok(data)))
    }
    pub fn not_found() -> Self {
        Self::Ok(Json(ResponseBody::not_found()))
    }
}
