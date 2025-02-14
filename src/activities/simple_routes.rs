use rocket::{get, State};
use rust_fuzzy_search::fuzzy_search_best_n;
use serde::{Deserialize, Serialize};
use crate::{authenticated::Authenticated, config::server_config::ServerConfig, model::MusicUploaderError, path_utils};


#[get("/auth")]
pub fn check_auth(_auth: Authenticated) -> &'static str {
    "hello"
}

#[get("/conn")]
pub fn check_conn() -> &'static str {
    "hello"
}

#[derive(Serialize, Deserialize)]
pub struct AlbumSearchResponse {
    pub albums: Vec<String>,
}

#[get("/albumsearch/<album>")]
pub async fn album_search(
    server_config: &State<ServerConfig>,
    album: &str,
) -> Result<AlbumSearchResponse, MusicUploaderError> {
    let albums = path_utils::get_album_names(&server_config.upload_dir)
        .map_err(|e| {
            println!("error: {}", e);
            MusicUploaderError::ValidateDirectoryError(Box::new(e))
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