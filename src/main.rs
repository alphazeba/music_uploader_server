use rocket::{
    data::ToByteUnit, fairing::AdHoc, http::{self, HeaderMap}, request::{self, FromRequest}, tokio::{fs, io::AsyncWriteExt}, Data, Request, State
};
use rust_fuzzy_search::fuzzy_search_best_n;
use thiserror::Error;
use music_uploader_server::{authenticated::{Authenticated, Authenticator}, path_utils};
use music_uploader_server::path_utils::build_and_validate_path;
use music_uploader_server::model::{AlbumSearchResponse, MusicUploaderServerError};
use music_uploader_server::server_config::ServerConfig;

#[macro_use]
extern crate rocket;

#[get("/auth")]
fn check_auth(_auth: Authenticated) -> &'static str {
    "hello"
}

#[get("/conn")]
fn check_conn() -> &'static str {
    "hello"
}

#[derive(Error, Debug)]
pub enum HeaderError {
    #[error("could not parse headers")]
    ParsingIssue,
}

struct UploadHeaders {
    hash: String,
    file_name: String,
    album: String,
    artist: String,
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

#[post("/upload", data = "<data>")]
async fn upload(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: UploadHeaders,
    data: Data<'_>,
) -> Result<String, String> {
    println!("\n{} is trying to upload {}", auth.username, headers.file_name);
    match upload_inner(server_config, headers, data).await {
        Ok(x) => {
            println!("success :3");
            Ok(x)
        }
        Err(e) => {
            println!("error: {:?}", e);
            Err(e)
        }
    }
}

async fn upload_inner(
    server_config: &State<ServerConfig>,
    headers: UploadHeaders,
    data: Data<'_>,
) -> Result<String, String> {
    let dir = build_and_validate_path(
        server_config,
        &headers.artist,
        &headers.album,
        &headers.file_name,
    ).await.map_err(|e| e.to_string())?;
    println!("using directory: {}", dir);
    let incoming_data = data.open(server_config.max_mb.megabytes());
    let bytes = incoming_data.into_bytes().await
        .map_err(|e| e.to_string() )?;
    if !bytes.is_complete() {
        return Err("File uploaded is too large".to_string());
    }
    if !check_hash(headers.hash, &bytes.value) {
        return Err("hash failure".to_string());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir).await.map_err(|e| e.to_string())?;
    let _ = file.write_all(&bytes).await
        .map_err(|e| e.to_string())?;
    Ok(format!("uploaded file: {}", headers.file_name))
}

#[get("/albumsearch/<album>")]
async fn album_search(
    server_config: &State<ServerConfig>,
    album: &str,
) -> Result<AlbumSearchResponse, MusicUploaderServerError> {
    let albums = path_utils::get_album_names(&server_config.upload_dir)
        .map_err(|e| {
            println!("error: {}", e);
            MusicUploaderServerError::ValidateDirectoryError(Box::new(e))
        })?;
    Ok(AlbumSearchResponse {
        albums: fuzzy_search_best_n(
            &album,
            &albums.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            5
        ).iter().map(|(matched_album, score)| {
            println!("album: {}, score: {}", matched_album, score);
            matched_album.to_string()
        }).collect()
    })
}

fn check_hash(sent_hash: String, file: &Vec<u8>) -> bool {
    let computed_hash = sha256::digest(file);
    sent_hash == computed_hash
}

#[catch(401)]
fn unauthorized() -> String {
    "request is not authorized".to_string()
}

#[launch]
fn rocket() -> _ {
    let authenticator= Authenticator::new()
        .expect("cannot run server without authenticator must look into issues");
    rocket::build()
        .register("/", catchers![unauthorized])
        .mount("/", routes![check_conn, check_auth, upload, album_search])
        .attach(AdHoc::config::<ServerConfig>())
        .manage(authenticator)
}
