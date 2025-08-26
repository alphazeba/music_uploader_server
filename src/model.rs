use crate::path_utils::ValidateDirectoryError;
use rocket::{
    http::{ContentType, Status},
    response::Responder,
    Response,
};
use serde::{Deserialize, Serialize};
use serde_json::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HeaderError {
    #[error("could not parse headers")]
    ParsingIssue,
}

#[derive(Error, Debug)]
pub enum MusicUploaderError {
    // user issue
    #[error("issue parsing the directory: {0}")]
    ValidateDirectoryError(Box<ValidateDirectoryError>),
    #[error("Song already exists")]
    SongAlreadyExists,
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
    // not user issue
    #[error("serde issue: {0}")]
    SerdeIssue(Box<Error>),
    #[error("plex is complaining with status: ({0})")]
    PlexComplaint(u16),
    #[error("There was an internal server that was not a customer issue. Reason: {0}")]
    InternalServerError(String),
    #[error("Could not find value in uploader db")]
    UploaderDataIncomplete,
}

#[derive(Serialize, Deserialize)]
pub struct AlbumSearchResponse {
    pub album: String,
    pub uploader: String,
}

#[derive(Serialize, Deserialize)]
pub enum DeclareUploadResponse {
    Complete,
    Incomplete {
        key: String,
        declared_size: u32,
        part_size: u32,
        received_parts: Vec<u8>,
    },
}

pub fn to_json(obj: &impl Serialize) -> Result<String, MusicUploaderError> {
    serde_json::to_string(obj).map_err(|e| MusicUploaderError::SerdeIssue(Box::new(e)))
}

pub fn from_json<'a, T: Deserialize<'a>>(json: &'a str) -> Result<T, MusicUploaderError> {
    serde_json::from_str::<'a, T>(json).map_err(|e| MusicUploaderError::SerdeIssue(Box::new(e)))
}

impl<'r> Responder<'r, 'static> for AlbumSearchResponse {
    // make this generic if there are more return types
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let response = to_json(&self).unwrap();
        Response::build_from(response.respond_to(request)?)
            .header(ContentType::new("application", "json"))
            .status(Status::Ok)
            .ok()
    }
}

impl<'r> Responder<'r, 'static> for DeclareUploadResponse {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let response = to_json(&self).unwrap();
        Response::build_from(response.respond_to(request)?)
            .header(ContentType::new("application", "json"))
            .status(Status::Ok)
            .ok()
    }
}

impl<'r> Responder<'r, 'static> for MusicUploaderError {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let response = self.to_string();
        Response::build_from(response.respond_to(request)?)
            .header(ContentType::new("application", "json"))
            .status(Status::InternalServerError)
            .ok()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_error_prints_boxed_error() {
        let err = MusicUploaderError::ValidateDirectoryError(Box::new(
            ValidateDirectoryError::FileAlreadyExists,
        ));
        assert_eq!(
            "issue parsing the directory: File already exists".to_string(),
            err.to_string()
        );
    }
}
