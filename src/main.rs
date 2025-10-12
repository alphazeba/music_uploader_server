#[macro_use]
extern crate rocket;
use music_uploader_server::services::sync_public_playlists::start_sync_public_playlists;

#[launch]
async fn rocket() -> _ {
    start_sync_public_playlists();
    music_uploader_server::build_rocket()
}
