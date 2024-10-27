use crate::{
    auth::{
        apikey::JwtAuth,
        jwt::Claims,
    },
    common::{
        Page,
        PageResult,
        Response,
        Result,
    },
    project::biz::{
        PageBo,
        Project,
        ProjectBo,
        ProjectPage,
        ProjectsRepo,
    },
};
use ioc::{mvc, Bean, OpenApi};
use poem_openapi::{
    param::Path,
    payload::Json,
    Object,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::common::PhiTags;
use crate::project::deploy::{DeployProject, DeployService};

#[derive(Debug, Deserialize, Serialize, Object)]
pub(crate) struct ProjectSearchCondition {
    #[serde(flatten)]
    #[oai(flatten = true)]
    pub(crate) page: Page,
    /// not work
    pub(crate) query: Option<String>,
}

#[derive(Bean)]
pub(crate) struct ProjectMvc {
    #[inject(bean)]
    repo: &'static ProjectsRepo,
    #[inject(bean)]
    deploy: &'static DeployService,
}

#[derive(Debug, Deserialize, Serialize, Object)]
pub(crate) struct PageBatchDeleteRequest {
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Object)]
pub(crate) struct NewProjectAndPages {
    #[serde(flatten)]
    #[oai(flatten = true)]
    pub(crate) project: ProjectBo,
    pub(crate) pages: Option<Vec<PageBo>>,
}

#[mvc]
#[OpenApi(prefix_path = "/api/v1", tag = PhiTags::Project )]
impl ProjectMvc {
    /// 搜索projects 目前只有分页参数有效 query无效
    #[oai(path = "/projects/search", method = "post")]
    async fn search(&self, condition: Json<ProjectSearchCondition>, auth: JwtAuth) -> Result<Response<PageResult<Project>>> {
        info!("{:?}", condition);

        let auth: Claims = auth.into();
        let result = self
            .repo
            .search(&condition.0, &auth.id)
            .await?;

        Ok(Response::ok(result))
    }

    /// 列举指定的project下面的pages
    #[oai(path = "/projects/:project_id/pages", method = "get")]
    async fn list_pages(&self, project_id: Path<String>, auth: JwtAuth) -> Result<Response<Vec<ProjectPage>>> {
        let _auth: Claims = auth.into();

        let result = self
            .repo
            .list_pages(&project_id)
            .await?;

        Ok(Response::ok(result))
    }

    /// 部署指定的project下面的pages
    #[oai(path = "/projects/:project_id/deploy", method = "post")]
    async fn deploy(&self, project_id: Path<String>, auth: JwtAuth) -> Result<Response<String>> {
        let _auth: Claims = auth.into();

        let result = self
            .deploy.deploy(&project_id)
            .await?;

        Ok(Response::ok(result))
    }

    /// 新建project, 此时允许附带pages, 可以为空
    #[oai(path = "/projects", method = "post")]
    async fn new_project(
        &self,
        request: Json<NewProjectAndPages>,
        auth: JwtAuth,
    ) -> Result<Response<String>> {
        let auth: Claims = auth.into();

        let result = self
            .repo
            .new_project(&request.project, &request.pages, &auth.id)
            .await?;

        Ok(Response::ok(result))
    }


    /// 给指定的project追加pages
    #[oai(path = "/projects/:project_id/pages", method = "post")]
    async fn new_pages(
        &self,
        project_id: Path<String>,
        request: Json<Vec<PageBo>>,
        auth: JwtAuth,
    ) -> Result<Response<String>> {
        let auth: Claims = auth.into();

        self.repo
            .new_pages(&project_id, &request.0, &auth.id)
            .await?;

        Ok(Response::ok("ok".to_string()))
    }

    /// 修改projects自身的配置，不包含pages
    #[oai(path = "/projects/:id", method = "put")]
    async fn update_project(
        &self,
        id: Path<String>,
        request: Json<ProjectBo>,
        auth: JwtAuth,
    ) -> Result<Response<String>> {
        let _auth: Claims = auth.into();

        self.repo
            .update_project(&id, &request.0)
            .await?;

        Ok(Response::ok("ok".to_string()))
    }

    /// 修改单个page的配置
    #[oai(path = "/projects/:project_id/pages/:id", method = "put")]
    async fn update_single_page(
        &self,
        project_id: Path<String>,
        id: Path<String>,
        request: Json<PageBo>,
        auth: JwtAuth,
    ) -> Result<Response<String>> {
        let _auth: Claims = auth.into();

        self.repo
            .update_single_page(&project_id, &id, &request.0)
            .await?;

        Ok(Response::ok(id.0))
    }

    /// 删除project，关联的page也会删除
    #[oai(path = "/projects/:id", method = "delete")]
    async fn delete_project(&self, id: Path<String>, _auth: JwtAuth) -> Result<Response<String>> {
        self.repo.delete_project(&id).await?;
        Ok(Response::ok("ok".to_string()))
    }

    /// 删除指定的pages
    #[oai(path = "/projects/pages/batch_delete", method = "post")]
    async fn batch_delete(&self, request: Json<PageBatchDeleteRequest>, _auth: JwtAuth) -> Result<Response<String>> {
        self.repo.delete_pages(request.ids.as_ref()).await?;
        Ok(Response::ok("ok".to_string()))
    }
}

#[derive(Bean)]
pub(crate) struct DeployTest;

#[mvc]
#[OpenApi]
impl DeployTest {
    /// mock的部署测试url
    #[oai(path = "/deploy/test", method = "post")]
    async fn test(&self, request: Json<DeployProject>) -> Result<Response<DeployProject>> {
        info!("{:?}", request);
        Ok(Response::ok(request.0))
    }

}