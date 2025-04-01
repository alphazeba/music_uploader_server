use std::path::Path;
use reqwest::Client;
use rocket::{post, State};
use crate::{authenticated::Authenticated, config::server_config::ServerConfig, data::metrics::Metrics, model::MusicUploaderError};

#[post("/triggerscan")]
pub async fn trigger_scan(
    auth: Authenticated,
    server_config: &State<ServerConfig>
) -> Result<String, MusicUploaderError> {
    println!("{} is triggering a scan", auth.username);
    let client = Client::new();
    let path: String = Path::new(&server_config.plex_url)
        .join(format!(
            "library/sections/{}/refresh",
            server_config.plex_music_library_id.to_string()))
        .to_str().map(str::to_string)
        .ok_or_else(|| {
            let message= "Failed to build url trigger scanning.  This likely means Rocket.toml is bad.";
            println!("{}", message);
            MusicUploaderError::InternalServerError(message.to_string())
        })?;
    metric(&server_config.server_db_dir, &auth.username);
    match client.get(path)
        .query(&[("X-Plex-Token", &server_config.plex_server_token)])
        .send()
        .await {
            Ok(response) => {
                if !response.status().is_success() {
                    println!("plex error: {}", response.status().as_u16());
                    return Err(MusicUploaderError::PlexComplaint(response.status().as_u16()))
                } else {
                    println!("plex successfully scanned :)");
                    return Ok("successful scan".to_string())
                }
            }
            Err(e) => {
                println!("there was an error reaching out to plex to trigger scan: {}", e.to_string());
                Err(MusicUploaderError::InternalServerError(e.to_string()))
            }
        }
}

fn metric(db_path: &String, user: &String) {
    let metrics = Metrics::new(db_path);
    let _ = metrics.note_route(&"triggerScan".to_string(), user);
}