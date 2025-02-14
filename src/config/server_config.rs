use rocket::serde;

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfig {
    pub upload_dir: String,
    pub valid_extensions: Vec<String>,
    pub max_mb: u32,
    pub plex_server_token: String,
    pub plex_url: String,
    pub plex_music_library_id: u16,
}