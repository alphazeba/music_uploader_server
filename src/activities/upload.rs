use std::io::Write;
use std::{fmt, fs};

use rocket::data::{Data, ToByteUnit};
use rocket::request::FromRequest;
use rocket::{http, post, request, Request, State};

use crate::authenticated::Authenticated;
use crate::config::server_config::ServerConfig;
use crate::data::metrics::Metrics;
use crate::model::{HeaderError, MusicUploaderError};
use crate::path_utils::{build_and_validate_path, ValidateDirectoryError};
use crate::rocket_utils::get_header_string;

pub struct UploadHeaders {
    hash: String,
    file_name: String,
    album: String,
    artist: String,
}

impl fmt::Debug for UploadHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadHeaders")
            .field("file_name", &self.file_name)
            .field("album", &self.album)
            .field("artist", &self.artist)
            .finish()
    }
}

#[post("/upload", data = "<data>")]
pub async fn upload(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: UploadHeaders,
    data: Data<'_>,
) -> Result<String, MusicUploaderError> {
    println!("\n{} is trying to upload {:?}", &auth.username, headers);
    match upload_inner(server_config, headers, data, &auth.username).await {
        Ok(x) => {
            println!("success :3");
            Ok(x)
        }
        Err(MusicUploaderError::SongAlreadyExists) => {
            println!("Assuming success since song already existed");
            Ok(MusicUploaderError::SongAlreadyExists.to_string())
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
    username: &String,
) -> Result<String, MusicUploaderError> {
    let dir = build_and_validate_path(
        server_config,
        &headers.artist,
        &headers.album,
        &headers.file_name,
    )
    .await
    .map_err(|e| match e {
        ValidateDirectoryError::FileAlreadyExists => MusicUploaderError::SongAlreadyExists,
        e => MusicUploaderError::ValidateDirectoryError(Box::new(e)),
    })?;
    let dir_str = dir.to_str().unwrap_or("<no dir?>").to_string();
    println!("using directory: {}", &dir_str);
    let incoming_data = data.open(server_config.max_mb.megabytes());
    let bytes = incoming_data
        .into_bytes()
        .await
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    if !bytes.is_complete() {
        return Err(MusicUploaderError::ConstraintViolation(
            "File uploaded is too large".to_string(),
        ));
    }
    if !check_hash(headers.hash, &bytes.value) {
        return Err(MusicUploaderError::ConstraintViolation(
            "Hash check failed".to_string(),
        ));
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    let _ = file
        .write_all(&bytes)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    metric(&server_config.server_db_dir, &dir_str, username);
    Ok(format!("uploaded file: {}", headers.file_name))
}

fn metric(db_path: &String, song_path: &String, user: &String) {
    let metrics = Metrics::new(db_path);
    let _ = metrics.note_route(&"upload".to_string(), user);
    let _ = metrics.note_upload(song_path, user);
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
    async fn from_request_inner(req: &'r Request<'_>) -> Result<Self, HeaderError> {
        let headers = req.headers();
        Ok(UploadHeaders {
            hash: get_header_string(headers, "hash")?,
            file_name: get_header_string(headers, "file")?,
            album: get_header_string(headers, "album")?,
            artist: get_header_string(headers, "artist")?,
        })
    }
}
