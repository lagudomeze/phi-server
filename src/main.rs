use std::net::SocketAddr;

use anyhow::Result as AnyResult;
use axum::{
    Router,
};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipauto::utoipauto;

mod auth;
mod material;
mod common;

#[utoipauto]
#[derive(OpenApi, Debug)]
#[openapi(
    tags(
        (name = "todo", description = "Todo management endpoints.")
    ),
)]
struct ApiDoc;

fn swagger() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .merge(auth::router())
        .merge(swagger());

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
