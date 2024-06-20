use std::time::Duration;

use ioc::Bean;
use salvo::{
    logging::Logger,
    prelude::*,
};
use salvo::conn::tcp::TcpAcceptor;
use tracing::info;
use crate::web;

#[derive(Bean)]
pub(crate) struct WebServer {
    #[value("web.addr")]
    addr: String,
}

impl WebServer {
    pub async fn run_server(&self, router: Router) -> anyhow::Result<()> {
        let name = env!("CARGO_PKG_NAME");
        let version = env!("CARGO_PKG_VERSION");

        let doc = OpenApi::new(name, version)
            .merge_router(&router);

        let router = Router::new()
            .push(doc.into_router("/api-doc/openapi.json"))
            .push(SwaggerUi::new("/api-doc/openapi.json")
                .into_router("swagger-ui"))
            .push(router);

        let logger = Logger::new();

        let service = Service::new(router)
            .hoop(logger);

        let acceptor = TcpListener::new(self.addr.as_str())
            .try_bind()
            .await?;

        info!("listening on {}", self.addr.as_str());

        let server = Server::new(acceptor);

        gracefully_shutdown(&server);

        server.try_serve(service)
            .await?;

        info!("server stop.");

        Ok(())
    }

    pub fn run(router: Router) -> anyhow::Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async {
                web::WebServer::try_get()?
                    .run_server(router)
                    .await
            })?;
        Ok(())
    }
}

fn gracefully_shutdown(server: &Server<TcpAcceptor>) {
    let handle = server.handle();

    tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };
        ctrl_c.await;
        handle.stop_graceful(Duration::from_secs(3));
    });
}