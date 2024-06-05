
use axum::{
    debug_handler,
    Router,
    routing::get,
    response::Result
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::{
    common::{Array},
    db::db,
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

#[derive(Serialize, Deserialize, Debug, ToSchema, ToResponse)]
pub(crate) struct User {
    id: String,
    name: String,
    source: Option<String>,
    created_at: i64,
}

#[debug_handler]
#[utoipa::path(get,
    path = "/manager/users",
    tag = "manager",
    responses((status = 200, body = [User]))
)]
pub(crate) async fn users() -> Result<Array<User>> {
    let conn = db();
    let x = sqlx::query_as!(User, "select * from users")
        .fetch_all(conn)
        .await;
    Ok(x.unwrap().into())
}


