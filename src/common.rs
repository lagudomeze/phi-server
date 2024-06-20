use salvo::{
    oapi::{self, Components, Operation},
    prelude::*,
};
use serde::Serialize;
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("sqlx error: `{0}`")]
    DbSqlxError(#[from] sqlx::Error),
    #[error("ioc error: `{0}`")]
    IocError(#[from] ioc::IocError),
    #[error("clap error: `{0:?}`")]
    ClapError(#[from] clap::Error),
    #[error(transparent)]
    SalvoError(#[from] salvo::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Serialize, ToSchema, ToResponse)]
pub struct ErrorResponse {
    code: u16,
    message: String,
}

impl Scribe for AppError {
    fn render(self, res: &mut Response) {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.render(Json(ErrorResponse {
            code: 500,
            message: self.to_string(),
        }));
    }
}

impl EndpointOutRegister for AppError {
    fn register(components: &mut Components, operation: &mut Operation) {
        operation.responses.insert(
            StatusCode::INTERNAL_SERVER_ERROR.as_str(),
            oapi::Response::new("Internal server error")
                .add_content("application/json", ErrorResponse::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::NOT_FOUND.as_str(),
            oapi::Response::new("Not found")
                .add_content("application/json", ErrorResponse::to_schema(components)),
        );
        operation.responses.insert(
            StatusCode::BAD_REQUEST.as_str(),
            oapi::Response::new("Bad request")
                .add_content("application/json", ErrorResponse::to_schema(components)),
        );
    }
}