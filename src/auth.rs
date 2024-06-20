use ioc::Bean;
use salvo::{
    prelude::*,
};
use crate::common::Result;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::NaiveDateTime;

use crate::db::Db;

pub(crate) fn router() -> Router {
    Router::new()
        .push(Router::with_path("/manager/users").get(users))
        .push(Router::with_path("/welcome").get(welcome))
}

#[endpoint]
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

#[endpoint(status_codes(200, 500))]
pub(crate) async fn users() -> Result<Json<Vec<User>>> {
    let conn = Db::get();
    let result: Vec<User> = sqlx::query_as("SELECT id, name, source, created_at FROM users")
        .fetch_all(conn)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(Json(result))
}


