use poem_openapi::{
    payload::Json,
    types::{ParseFromJSON, ToJSON, Type},
    ApiResponse, Object,
};
use serde::Serialize;

#[derive(Object, Serialize)]
pub struct ResponseBody<T: Type + ParseFromJSON + ToJSON + Serialize> {
    code: i32,
    msg: String,
    data: Option<T>,
}

impl<T: Type + ParseFromJSON + ToJSON + Serialize> ResponseBody<T> {
    pub fn ok(data: T) -> Self {
        Self::new(0, "OK".to_string(), data)
    }

    pub fn new(code: i32, msg: String, data: T) -> Self {
        Self {
            code,
            msg,
            data: Some(data),
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: 404,
            msg: "not found".to_string(),
            data: None,
        }
    }
}

#[derive(ApiResponse)]
#[oai]
pub enum Response<T: Type + ParseFromJSON + ToJSON + Serialize> {
    #[oai(status = 200)]
    Ok(Json<ResponseBody<T>>),
    #[oai(status = 404)]
    NotFound(Json<ResponseBody<T>>),
}

impl<T> Response<T>
where
    T: Type + ParseFromJSON + ToJSON + Serialize,
{
    pub fn ok(data: T) -> Self {
        Self::Ok(Json(ResponseBody::ok(data)))
    }
    pub fn not_found() -> Self {
        Self::Ok(Json(ResponseBody::not_found()))
    }
}
