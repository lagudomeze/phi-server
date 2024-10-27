use crate::common::{AppError, LocationContext};
use crate::{
    client::HttpClient,
    common::Result,
    project::biz::{
        Project,
        ProjectPage,
        ProjectsRepo,
    },
};
use ioc::Bean;
use serde::{Deserialize, Serialize};
use std::panic::Location;
use std::time::Duration;
use poem_openapi::Object;
use url::Url;

#[derive(Bean)]
pub(crate) struct DeployService {
    #[inject(config = "project.deploy-url")]
    deploy_url: String,

    #[inject(config = "project.timeout")]
    timeout: Duration,

    #[inject(bean)]
    client: &'static HttpClient,

    #[inject(bean)]
    repo: &'static ProjectsRepo,
}

#[derive(Debug, Deserialize, Serialize, Object)]
pub(crate) struct DeployProject {
    #[serde(flatten)]
    #[oai(flatten = true)]
    pub(crate) project: Project,
    pub(crate) pages: Vec<ProjectPage>,
}

impl DeployService {
    pub(crate) async fn deploy(&self, id: &str) -> Result<String> {
        if let Some(project) = self.repo.find_by_id(id).await? {
            let deploy = DeployProject {
                project,
                pages: self.repo.list_pages(id).await?,
            };

            let client = self.client.rest();

            let mut url = Url::parse(&self.deploy_url)?;
            url
                .query_pairs_mut()
                .append_pair("project_id", id);

            let response = client
                .post(url)
                .json(&deploy)
                .timeout(self.timeout)
                .send()
                .await
                .location("call deploy failed", Location::caller())?;

            if response.status().is_success() {
                let text = response
                    .text()
                    .await
                    .location("get access_token parse json failed", Location::caller())?;

                Ok(text)
            } else {
                let text = response
                    .text()
                    .await
                    .location("get access_token parse json failed", Location::caller())?;
                Err(AppError::DeployError(format!("failed to deploy service: {}", text)))
            }
        } else {
            Err(AppError::DeployError(format!("deploy not found: {}", id)))
        }
    }
}