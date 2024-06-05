use std::time::Instant;

use axum::{debug_handler, Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::common::approx_instant;

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
    source: String,
    #[schema(value_type = String)]
    #[serde(with = "approx_instant")]
    create_at: Instant,
}

#[debug_handler]
#[utoipa::path(get,
    path = "/manager/users",
    tag = "manager",
    responses((status = 200, body = [User]))
)]
pub(crate) async fn users() -> Json<Vec<User>> {
    todo!()
}


