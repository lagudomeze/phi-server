use std::net::SocketAddr;

use anyhow::Result as AnyResult;
use axum::{
    debug_handler,
    extract::Path,
    http::StatusCode,
    Json,
    response::{
        IntoResponse,
        Result
    },
    Router,
    routing::{
        get, post
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

#[tokio::main]
async fn main() -> AnyResult<()> {

    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/", get(root))
        .route("/hello/:name/:test", get(json_hello))
        .route("/user", post(create_user));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    info!("listening on {}", addr);

    let tcp = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(tcp, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("server stop.");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    tokio::select! {
        _ = ctrl_c => {},
    }
}

#[derive(Deserialize, Debug)]
struct Info {
    name: String,
    test: String,
}

#[debug_handler]
async fn json_hello(Path(info): Path<Info>) -> Result<Json<Value>> {
    info!("haha {info:?}");
    let Info { name, test } = info;

    let message = format!("Hello {name} with {test}");

    let value = json!({
        "message":message
    });
    Ok(Json(value))
}

#[debug_handler]
async fn root() -> &'static str {
    "Hello, World!"
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, Hash, PartialEq)]
struct User {
    id: u64,
    username: String,
}

#[debug_handler]
async fn create_user(Json(payload): Json<CreateUser>) -> impl IntoResponse {
    let user = User {
        id: 1337,
        username: payload.username,
    };

    (StatusCode::CREATED, Json(user))
}