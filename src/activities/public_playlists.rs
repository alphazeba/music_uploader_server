use rocket::{get, State};

use crate::{authenticated::Authenticated, config::server_config::ServerConfig, data::operational_data::{OperationalData}, model::{ListedPublicPlaylist, MusicUploaderError, PublicPlaylistResponse}};



#[get("/publicplaylists")]
pub async fn public_playlists(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
) -> Result<PublicPlaylistResponse, MusicUploaderError> {
    println!("{} is looking up public playlists", auth.username);
    let operational_data = OperationalData::new(&server_config.server_operational_db_dir);
    let playlists = operational_data.get_last_known_playlist_state()
        .ok_or(MusicUploaderError::InternalServerError("Failed to get public playlists".to_string()))?
        .into_iter()
        .map(|item| {
            ListedPublicPlaylist {
                title: item.title,
                num_subscribers: item.subscriber_ids.len() as u32,
                num_songs: item.song_ids.len() as u32,
            }
        })
        .collect::<Vec<_>>();
    Ok(PublicPlaylistResponse { playlists })
}
