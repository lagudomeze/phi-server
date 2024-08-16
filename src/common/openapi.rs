use poem_openapi::types::{ParseFromJSON, ToJSON, Type};
use poem_openapi::{Object, Tags};
use serde::{Deserialize, Serialize};

#[derive(Tags)]
pub(crate) enum PhiTags {
    Auth,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Object)]
pub(crate) struct Page {
    #[serde(alias = "pageNo")]
    pub(crate) page: u32,
    #[serde(alias = "pageSize")]
    pub(crate) size: u32,
}

impl Page {
    pub(crate) fn offset(&self) -> i64 {
        (self.page as i64 - 1) * self.size as i64
    }

    pub(crate) fn limit(&self) -> i64 {
        self.size as i64
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Object)]
pub(crate) struct PageResult<T: Type + ParseFromJSON + ToJSON + Serialize> {
    #[serde(alias = "pageNo")]
    pub(crate) page: u32,
    #[serde(alias = "pageSize")]
    pub(crate) size: u32,
    #[serde(alias = "totalRecords")]
    pub(crate) total: u64,
    #[serde(default)]
    pub(crate) records: Vec<T>,
}

impl<T> From<Page> for PageResult<T>
where
    T: Type + ParseFromJSON + ToJSON + Serialize,
{
    fn from(page: Page) -> Self {
        Self {
            page: page.page,
            size: page.size,
            total: 0,
            records: Vec::new(),
        }
    }
}

impl<T> PageResult<T>
where
    T: Type + ParseFromJSON + ToJSON + Serialize,
{
    pub(crate) fn new(page: &Page, total: u64, records: Vec<T>) -> Self {
        Self {
            page: page.page,
            size: page.size,
            total,
            records,
        }
    }

    pub(crate) fn transfer<U, F>(self, method: F) -> super::Result<PageResult<U>>
    where
        U: Type + ParseFromJSON + ToJSON + Serialize,
        F: Fn(T) -> super::Result<U>,
    {
        let mut records = Vec::new();
        for record in self.records {
            records.push(method(record)?);
        }
        Ok(PageResult {
            page: self.page,
            size: self.size,
            total: self.total,
            records,
        })
    }
}
