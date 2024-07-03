use std::time::Duration;

use ioc::Bean;
use poem::{
    EndpointExt,
    get,
    handler,
    listener::TcpListener,
    middleware::Tracing,
    Response,
    Route,
    Server
};
use tracing::info;
#[derive(Bean)]
pub(crate) struct WebServer {
    #[value("web.addr")]
    addr: String,
    #[value("web.graceful_shutdown_timeout")]
    shutdown_timeout: Duration,
    #[value("web.tracing")]
    tracing: bool,
}

#[handler]
async fn favicon() -> Response {
    static ICO: &[u8] = include_bytes!("../asserts/favicon.ico");
    Response::builder().content_type("image/x-icon").body(ICO)
}

impl WebServer {
    pub async fn run_server(&self) -> anyhow::Result<()> {
        let name = env!("CARGO_PKG_NAME");
        let version = env!("CARGO_PKG_VERSION");

        let api_service = crate::open_api_service(name, version);

        let ui = api_service.swagger_ui();

        let app = Route::new()
            .nest("/", api_service)
            .nest("/ui", ui)
            .at("/favicon.ico", get(favicon))
            .with_if(self.tracing, Tracing::default());

        let listener = TcpListener::bind(self.addr.as_str());

        let server = Server::new(listener)
            .run_with_graceful_shutdown(
                app,
                gracefully_shutdown(),
                Some(self.shutdown_timeout));

        server.await?;

        Ok(())
    }

    pub fn run() -> anyhow::Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let metrics = runtime.metrics();
        info!("workers: {}", metrics.num_workers());
        runtime.block_on(async {
                WebServer::try_get()?.run_server().await
            })?;
        Ok(())
    }
}

async fn gracefully_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}