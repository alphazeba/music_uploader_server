use std::fs;
use std::io::Write;

use rocket::http::HeaderMap;
use rocket::{http, post, request, Request, State};
use rocket::data::{Data, ToByteUnit};
use rocket::request::FromRequest;

use crate::model::{HeaderError, MusicUploaderError};
use crate::path_utils::build_and_validate_path;
use crate::config::server_config::ServerConfig;
use crate::authenticated::Authenticated;

pub struct UploadHeaders {
    hash: String,
    file_name: String,
    album: String,
    artist: String,
}

#[post("/upload", data = "<data>")]
pub async fn upload(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: UploadHeaders,
    data: Data<'_>,
) -> Result<String, MusicUploaderError> {
    println!("\n{} is trying to upload {}", auth.username, headers.file_name);
    match upload_inner(server_config, headers, data).await {
        Ok(x) => {
            println!("success :3");
            Ok(x)
        }
        Err(e) => {
            println!("error: {}", e.to_string());
            Err(e)
        }
    }
}

async fn upload_inner(
    server_config: &State<ServerConfig>,
    headers: UploadHeaders,
    data: Data<'_>,
) -> Result<String, MusicUploaderError> {
    let dir = build_and_validate_path(
        server_config,
        &headers.artist,
        &headers.album,
        &headers.file_name,
    ).await.map_err(|e| MusicUploaderError::ValidateDirectoryError(Box::new(e)))?;
    println!("using directory: {}", dir);
    let incoming_data = data.open(server_config.max_mb.megabytes());
    let bytes = incoming_data.into_bytes().await
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    if !bytes.is_complete() {
        return Err(MusicUploaderError::ConstraintViolation("File uploaded is too large".to_string()));
    }
    if !check_hash(headers.hash, &bytes.value) {
        return Err(MusicUploaderError::ConstraintViolation("Hash check failed".to_string()));
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir).map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    let _ = file.write_all(&bytes)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    Ok(format!("uploaded file: {}", headers.file_name))
}

fn check_hash(sent_hash: String, file: &Vec<u8>) -> bool {
    let computed_hash = sha256::digest(file);
    sent_hash == computed_hash
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UploadHeaders {
    type Error = HeaderError;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match Self::from_request_inner(req).await {
            Ok(a) => request::Outcome::Success(a),
            Err(e) => request::Outcome::Error((http::Status::Unauthorized, e)),
        }
    }
}

impl<'r> UploadHeaders {
    async fn from_request_inner(req: &'r Request<'_>) -> Result<UploadHeaders, HeaderError> {
        let headers = req.headers();
        Ok(UploadHeaders {
            hash: Self::get_header_string(headers, "hash")?,
            file_name: Self::get_header_string(headers, "file")?,
            album: Self::get_header_string(headers, "album")?,
            artist: Self::get_header_string(headers, "artist")?,
        })
    }

    fn get_header_string(headers: &HeaderMap, key: &str) -> Result<String, HeaderError> {
        Ok(headers.get_one(key)
            .ok_or(HeaderError::ParsingIssue)?
            .to_string())
    }
}