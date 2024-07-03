use std::time::Duration;

use ioc::Bean;
use poem::{
    EndpointExt,
    get,
    handler,
    listener::TcpListener,
    Response,
    Route,
    Server,
    middleware::Tracing
};
use tracing::info;

#[derive(Bean)]
pub(crate) struct WebServer {
    #[value("web.addr")]
    addr: String,
    #[value("web.graceful_shutdown_timeout")]
    shutdown_timeout: Duration,
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
            .with(Tracing::default());


        let listener = TcpListener::bind(self.addr.as_str());

        let server = Server::new(listener)
            .run_with_graceful_shutdown(
                app,
                gracefully_shutdown(),
                Some(self.shutdown_timeout));

        info!("listening on {}", self.addr.as_str());

        server.await?;

        info!("server stop.");

        Ok(())
    }

    pub fn run() -> anyhow::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async {
                WebServer::try_get()?.run_server().await
            })?;
        Ok(())
    }
}

async fn gracefully_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}