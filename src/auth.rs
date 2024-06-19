use axum::{
    debug_handler,
    response::Result,
    Router,
    routing::get,
};
use ioc::Bean;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;
use utoipa::{ToResponse, ToSchema};

use crate::{
    common::Array,
    db::Db,
};

pub(crate) fn router() -> Router {
    Router::new()
        .route("/welcome", get(welcome))
        .route("/manager/users", get(users))
}

#[debug_handler]
#[utoipa::path(get,
    path = "/welcome",
    responses((status = 200, body = String))
)]
pub(crate) async fn welcome() -> String {
    // todo
    let email = "";
    format!("Welcome, {email}!")
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, ToSchema, ToResponse)]
pub(crate) struct User {
    id: String,
    name: String,
    source: Option<String>,
    created_at: NaiveDateTime,
}

#[debug_handler]
#[utoipa::path(get,
    path = "/manager/users",
    tag = "manager",
    responses((status = 200, body = [User]))
)]
pub(crate) async fn users() -> Result<Array<User>> {
    let conn = Db::get();
    let x = sqlx::query_as("SELECT id, name, source, created_at FROM users")
        .fetch_all(conn)
        .await;
    Ok(x.unwrap().into())
}


