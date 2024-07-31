use ioc::{log::LogPatcher, mvc, Bean};
use poem_openapi::{payload::Json, Object};
use serde::Deserialize;
use tracing::{debug, info, trace};

use crate::common::{Response, Result};

#[derive(Deserialize, Object)]
struct LogDirective {
    /// log format like my_crate::module=trace,debug
    value: String,
}

#[derive(Bean)]
pub struct Logger {
    #[inject(bean)]
    patcher: &'static LogPatcher,
}

#[mvc]
impl Logger {
    #[oai(path = "/loggers", method = "get")]
    async fn index(&self) -> Result<Response<String>> {
        info!("get logger");
        debug!("debug get logger: {:?}", self.patcher.to_string());
        trace!("debug get logger: {:?}", self.patcher.to_string());
        Ok(Response::ok(self.patcher.to_string()?))
    }

    #[oai(path = "/loggers", method = "post")]
    async fn set_logger(&self, body: Json<LogDirective>) -> Result<Response<String>> {
        let split = body.value.split(',');
        self.patcher.reload(split)?;
        Ok(Response::ok("ok".to_string()))
    }
}
