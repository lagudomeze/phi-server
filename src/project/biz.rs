use crate::{
    common::{AppError, PageResult},
    db::Db,
    project::{
        mvc::ProjectSearchCondition,
        new_id,
    },
};
use chrono::{NaiveDateTime, Utc};
use ioc::Bean;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{query_as_with, query_scalar_with, Arguments, Executor, QueryBuilder, Sqlite, SqlitePool};
use std::ops::Deref;
use tracing::warn;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Object)]
pub(crate) struct Project {
    id: String,

    repo_url: String,
    repo_branch: String,

    public_dir: String,
    store_dir: String,

    creator: String,
    created_at: NaiveDateTime,
}
#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Object)]
pub(crate) struct ProjectPage {
    project_id: String,
    id: String,
    name: String,

    category: String,
    content_type: String,
    content: String,

    creator: String,
    created_at: NaiveDateTime,
}
#[derive(Bean)]
pub struct ProjectsRepo {
    #[inject(bean = Db)]
    db: &'static SqlitePool,
}
#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct PageBo {
    name: String,
    category: String,
    content_type: String,
    content: String,
}
#[derive(Serialize, Deserialize, Debug, Object)]
pub(crate) struct ProjectBo {
    repo_url: String,
    repo_branch: String,

    public_dir: String,
    store_dir: String,
}
impl ProjectsRepo {
    pub(crate) async fn search(&self, condition: &ProjectSearchCondition, creator: impl AsRef<str>) -> crate::common::Result<PageResult<Project>> {
        let mut sql_select_args = sqlx::sqlite::SqliteArguments::default();
        let mut sql_count_args = sqlx::sqlite::SqliteArguments::default();

        let sql_select =
            "SELECT id, repo_url, repo_branch, public_dir, store_dir, creator, created_at FROM projects";

        let sql_count = "SELECT COUNT(*) FROM projects";

        let sql_where = " WHERE creator = ?".to_string();

        sql_select_args.add(creator.as_ref())?;
        sql_count_args.add(creator.as_ref())?;

        let total: u64 = query_scalar_with(&format!("{sql_count}{sql_where}"), sql_count_args)
            .fetch_one(self.db)
            .await?;

        let sql_order = " ORDER BY created_at DESC";

        let sql_limit = " LIMIT ? OFFSET ?";

        sql_select_args.add(condition.page.limit())?;
        sql_select_args.add(condition.page.offset())?;

        let records: Vec<Project> = query_as_with(
            &format!("{sql_select}{sql_where}{sql_order}{sql_limit}"),
            sql_select_args,
        )
            .fetch_all(self.db)
            .await?;

        Ok(PageResult::new(&condition.page, total, records))
    }
    pub(crate) async fn list_pages(&self, project_id: &str) -> crate::common::Result<Vec<ProjectPage>> {
        let records = sqlx::query_as("SELECT project_id, id, name, category, content_type, content, creator, created_at FROM project_pages WHERE project_id = ?")
            .bind(project_id)
            .fetch_all(self.db)
            .await?;

        Ok(records)
    }
    pub(crate) async fn new_project(&self, project: &ProjectBo, pages: &Option<Vec<PageBo>>, creator: impl AsRef<str>) -> crate::common::Result<String> {
        let mut tx = self.db.begin().await?;

        let project_id = new_id();
        let project_id_str = project_id.as_str();
        let created_at = Utc::now().naive_utc();
        let creator_id = creator.as_ref();

        let result = sqlx::query!(
            r#"
            INSERT INTO projects (id, repo_url, repo_branch, public_dir, store_dir, creator, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            project_id_str,
            project.repo_url,
            project.repo_branch,
            project.public_dir,
            project.store_dir,
            creator_id,
            created_at
        )
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::DbError("save project failed".to_string()));
        }

        if let Some(pages) = pages {
            ProjectsRepo::raw_new_page(&mut *tx, &project_id, pages, creator_id)
                .await?
        }

        tx.commit().await?;

        Ok(project_id)
    }
    pub(crate) async fn new_pages(&self, project_id: &str, pages: &[PageBo], creator: &str) -> crate::common::Result<()> {
        ProjectsRepo::raw_new_page(self.db, project_id, pages, creator).await
    }
    async fn raw_new_page<'a, E: Executor<'a, Database=Sqlite>>(db: E, project_id: &str, pages: &[PageBo], creator: &str) -> crate::common::Result<()> {
        let created_at = Utc::now().naive_utc();

        if pages.is_empty() {
            warn!("No pages for project[id={project_id}]")
        } else {
            QueryBuilder::new("INSERT INTO project_pages (project_id, id, name, category, content_type, content, creator, created_at) ")
                .push_values(pages.iter(), |mut b, page| {
                    let page_id = new_id();
                    b.push_bind(project_id);
                    b.push_bind(page_id);
                    b.push_bind(&page.name);
                    b.push_bind(&page.category);
                    b.push_bind(&page.content_type);
                    b.push_bind(&page.content);
                    b.push_bind(creator);
                    b.push_bind(created_at);
                })
                .build()
                .execute(db)
                .await?;
        }

        Ok(())
    }
    pub(crate) async fn update_project(&self, id: &str, project: &ProjectBo) -> crate::common::Result<()> {
        sqlx::query!(
            r#"
            UPDATE projects
            SET
                repo_url = ? ,
                repo_branch = ? ,
                public_dir = ? ,
                store_dir = ?
            WHERE id = ?
            "#,
            project.repo_url,
            project.repo_branch,
            project.public_dir,
            project.store_dir,
            id
        ).execute(self.db).await?;

        Ok(())
    }
    pub(crate) async fn update_single_page(&self, project_id: &str, id: &str, page: &PageBo) -> crate::common::Result<()> {

        sqlx::query!(
            r#"
            UPDATE project_pages
            SET
                name = ? ,
                category = ? ,
                content_type = ? ,
                content = ?
            WHERE id = ? AND project_id = ?
            "#,
            page.name,
            page.category,
            page.content_type,
            page.content,
            id,
            project_id
        ).execute(self.db).await?;

        Ok(())
    }
    pub(crate) async fn delete_project(&self, id: &str) -> crate::common::Result<()> {
        let mut tx = self.db.begin().await?;

        sqlx::query!(
            r#"
            DELETE FROM project_pages WHERE project_id = ?
            "#,
            id
        ).execute(&mut *tx)
            .await?;

        sqlx::query!(
            r#"
            DELETE FROM projects WHERE id = ?
            "#,
            id
        ).execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
    pub(crate) async fn delete_pages(&self, ids: &[String]) -> crate::common::Result<()> {
        let mut tx = self.db.begin().await?;

        QueryBuilder::new("DELETE FROM project_pages WHERE id IN")
            .push_tuples(ids.iter(), |mut b, id| {
                b.push_bind(id.deref());
            })
            .build()
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}