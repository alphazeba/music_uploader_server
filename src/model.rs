use rocket::{http::{ContentType, Status}, response::Responder, Response};
use serde::{Deserialize, Serialize};
use serde_json::error::Error;
use thiserror::Error;
use crate::path_utils::ValidateDirectoryError;

#[derive(Serialize, Deserialize)]
pub struct AlbumSearchResponse {
    pub albums: Vec<String>,
}

#[derive(Error, Debug)]
pub enum MusicUploaderServerError {
    #[error("issue parsing serailizing the item")]
    SerdeIssue(Box<Error>),
    #[error("issue parsing the directory")]
    ValidateDirectoryError(Box<ValidateDirectoryError>),
}

pub fn to_json(obj: &impl Serialize) -> Result<String, MusicUploaderServerError>  {
    serde_json::to_string(obj).map_err(|e| MusicUploaderServerError::SerdeIssue(Box::new(e)))
}

pub fn from_json<'a,T: Deserialize<'a>>(json: &'a str) -> Result<T, MusicUploaderServerError> {
    serde_json::from_str::<'a, T>(json).map_err(|e| MusicUploaderServerError::SerdeIssue(Box::new(e)))
}

impl<'r> Responder<'r, 'static> for AlbumSearchResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let response = to_json(&self).unwrap();
        Response::build_from(response.respond_to(request)?)
            .header(ContentType::new("application", "json"))
            .status(Status::Ok)
            .ok()
    }
}

impl<'r> Responder<'r, 'static> for MusicUploaderServerError {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let response = self.to_string();
        Response::build_from(response.respond_to(request)?)
            .header(ContentType::new("application", "json"))
            .status(Status::InternalServerError)
            .ok()
    }
}