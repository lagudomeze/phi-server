use std::time::Duration;

use ioc::{Bean, LogPatcher};
use salvo::{
    conn::tcp::TcpAcceptor,
    oapi::extract::*,
    prelude::*
};
use serde::Deserialize;
use tracing::{debug, info, trace};

use crate::common;

#[derive(Bean)]
pub(crate) struct WebServer {
    #[value("web.addr")]
    addr: String,
    #[value("web.graceful_shutdown_timeout")]
    shutdown_timeout: Duration,
}

#[handler]
fn favicon(res: &mut Response) -> salvo::Result<()> {
    static ICO: &[u8] = include_bytes!("../asserts/favicon.ico");
    res.write_body(ICO)
}

#[derive(Deserialize, ToParameters, ToSchema)]
struct LogDirective {
    /// log format like my_crate::module=trace,debug
    value: String,
}

#[endpoint]
fn set_logger(body: FormBody<LogDirective>) -> common::Result<&'static str> {
    let patcher = LogPatcher::try_get()?;
    let split = body.value.split(',');
    patcher.reload(split)?;
    Ok("ok")
}

#[endpoint]
fn get_logger() -> common::Result<String> {
    info!("get logger");
    let patcher = LogPatcher::try_get()?;
    debug!("debug get logger: {:?}", patcher.to_string());
    trace!("debug get logger: {:?}", patcher.to_string());
    Ok(patcher.to_string()?)
}

impl WebServer {
    pub async fn run_server(&self, router: Router) -> anyhow::Result<()> {
        let name = env!("CARGO_PKG_NAME");
        let version = env!("CARGO_PKG_VERSION");

        let logger = Router::with_path("loggers")
            .get(get_logger)
            .post(set_logger);

        let doc = OpenApi::new(name, version)
            .merge_router(&router)
            .merge_router(&logger);

        let router = Router::new()
            .push(doc.into_router("/api-doc/openapi.json"))
            .push(SwaggerUi::new("/api-doc/openapi.json").into_router("swagger-ui"))
            .push(Router::with_path("favicon.ico").get(favicon))
            .push(router)
            .push(logger);

        let logger = Logger::new();

        let service = Service::new(router)
            .hoop(logger);

        let acceptor = TcpListener::new(self.addr.as_str())
            .try_bind()
            .await?;

        info!("listening on {}", self.addr.as_str());

        let server = Server::new(acceptor);

        gracefully_shutdown(&server, self.shutdown_timeout);

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
                WebServer::try_get()?
                    .run_server(router)
                    .await
            })?;
        Ok(())
    }
}

fn gracefully_shutdown(server: &Server<TcpAcceptor>, duration: Duration) {
    let handle = server.handle();

    tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };
        ctrl_c.await;
        handle.stop_graceful(duration);
    });
}