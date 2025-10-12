use crate::{
    authenticated::Authenticated, clients::plex_client::PlexClient,
    config::server_config::ServerConfig, data::metrics::Metrics, model::MusicUploaderError,
};
use rocket::{post, State};

#[post("/triggerscan")]
pub async fn trigger_scan(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
) -> Result<String, MusicUploaderError> {
    println!("{} is triggering a scan", auth.username);
    let plex_client = PlexClient::new(
        &server_config.plex_url,
        server_config.plex_server_token.clone(),
    );
    let result = plex_client
        .trigger_scan(server_config.plex_music_library_id)
        .await
        .map(|_| {
            println!("plex successfully scanned :)");
            "successful scan".to_string()
        })
        .map_err(|e| {
            println!(
                "there was an error reaching out to plex to trigger scan: {}",
                e.to_string()
            );
            MusicUploaderError::InternalServerError(e.to_string())
        });
    metric(&server_config.server_db_dir, &auth.username);
    result
}

fn metric(db_path: &String, user: &String) {
    let metrics = Metrics::new(db_path);
    let _ = metrics.note_route(&"triggerScan".to_string(), user);
}
