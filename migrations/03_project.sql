CREATE TABLE IF NOT EXISTS projects
(
    id          VARCHAR(36) NOT NULL PRIMARY KEY,
    repo_url    VARCHAR(255) NOT NULL,
    repo_branch VARCHAR(64) NOT NULL,
    public_dir  VARCHAR(255) NOT NULL,
    store_dir   VARCHAR(255) NOT NULL,
    creator     VARCHAR(64) NOT NULL,
    created_at  INTEGER     NOT NULL
);

CREATE TABLE IF NOT EXISTS project_pages
(
    project_id   VARCHAR(36) NOT NULL,

    id           VARCHAR(36) NOT NULL PRIMARY KEY,
    name         VARCHAR(64) NOT NULL,

    category     VARCHAR(16) NOT NULL,
    content_type VARCHAR(16) NOT NULL,
    content      TEXT        NOT NULL,
    creator     VARCHAR(64) NOT NULL,
    created_at   INTEGER     NOT NULL
);

CREATE INDEX project_pages_project_id_index ON project_pages (project_id);