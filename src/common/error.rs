use crate::common::FormatedEvent;
use anyhow::Context;
use http::StatusCode;
use poem::{error::ResponseError, Body, Error, Response as PoemResponse};
use poem_openapi::{
    registry::{MetaResponses, Registry},
    ApiResponse,
};
use serde::de::StdError;
use std::fmt::Display;
use std::io;
use std::panic::Location;
use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("sqlx error: `{0}`")]
    DbSqlxError(#[from] sqlx::Error),
    #[error("sqlx error: `{0}`")]
    BoxDynErrorError(#[from] Box<dyn StdError + 'static + Send + Sync>),
    #[error("join error: `{0}`")]
    JoinError(#[from] JoinError),
    #[error("material not found: `{0}`")]
    MaterialNotFound(String),
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
    #[error("wrong material type: `{0}`")]
    WrongMaterialType(u16),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("`{0}`")]
    DbError(String),
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
