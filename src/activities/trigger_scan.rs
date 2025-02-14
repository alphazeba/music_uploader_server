use std::path::Path;
use reqwest::Client;
use rocket::{post, State};
use crate::{authenticated::Authenticated, config::server_config::ServerConfig, model::MusicUploaderServerError};

#[post("/triggerscan")]
pub async fn trigger_scan(
    auth: Authenticated,
    server_config: &State<ServerConfig>
) -> Result<String, MusicUploaderServerError> {
    println!("{} is triggering a scan", auth.username);
    let client = Client::new();
    let path: String = Path::new(&server_config.plex_url)
        .join(format!(
            "library/sections/{}/refresh",
            server_config.plex_music_library_id.to_string()))
        .to_str().map(str::to_string)
        .ok_or_else(|| {
            println!("Failed to build url trigger scanning.  This likely means Rocket.toml is bad.");
            MusicUploaderServerError::InternalServerError
        })?;
    match client.get(path).query(&("X-Plex-Token", &server_config.plex_server_token)).send().await {
        Ok(result) => {
            if !result.status().is_success() {
                println!("plex error: {}", result.status().as_u16());
                return Err(MusicUploaderServerError::PlexComplaint(result.status().as_u16()))
            }
            Ok(format!("Success: ({})", result.status().as_str()))
        },
        Err(e) => {
            println!("there was an error reaching out to plex to trigger scan: {}", e);
            Err(MusicUploaderServerError::InternalServerError)
        }
    }
}