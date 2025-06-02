use crate::{
    authenticated::Authenticated,
    config::server_config::ServerConfig,
    data::{
        metrics::Metrics,
        plex_db::{AlbumResult, PlexDb, SongResult},
    },
    model::{AlbumSearchResponse, MusicUploaderError},
    path_utils,
};
use rocket::get;
use rocket::State;
use rust_fuzzy_search::fuzzy_search_best_n;

#[get("/albumsearch/<album>")]
pub async fn album_search(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    album: &str,
) -> Result<AlbumSearchResponse, MusicUploaderError> {
    println!("{} is searching for {}", auth.username, album);
    let plex_db = PlexDb::new(&server_config.plex_db_dir);
    let albums = plex_db.get_albums().map_err(|e| {
        println!("internal error with search");
        MusicUploaderError::InternalServerError(e.to_string())
    })?;
    let found_album = find_searched_album_result(albums, album).map_err(|e| {
        MusicUploaderError::ConstraintViolation("could not find the searched for album".to_string())
    })?;
    let album_songs = plex_db
        .get_song_files_of_album(&found_album)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    let metric_db = Metrics::new(&server_config.server_db_dir);
    let upload_result = metric_db
        .get_upload(
            album_songs
                .get(0)
                .ok_or(MusicUploaderError::InternalServerError(
                    "found album had no songs".to_string(),
                ))?
                .get_path(),
        )
        .ok_or(MusicUploaderError::UploaderDataIncomplete)?;
    let response = Ok(AlbumSearchResponse {
        album: found_album.get_title().to_string(),
        uploader: upload_result.user,
    });
    metric(&metric_db, &auth.username, &"albumsearch".to_string());
    response
}

fn find_searched_album_result(albums: Vec<AlbumResult>, album: &str) -> Result<AlbumResult, ()> {
    let fuzzy_found_album_str = {
        let album_str_list = &albums
            .iter()
            .map(|x| x.get_title().as_str())
            .collect::<Vec<&str>>();
        let fuzzy_found_album_list = fuzzy_search_best_n(album, &album_str_list, 1);
        fuzzy_found_album_list
            .get(0)
            .map(|(title, _certainty)| *title) // consider checking if the certainty is too low?
            .ok_or(())?
            .to_string()
    };
    for album in albums {
        if *album.get_title() == fuzzy_found_album_str {
            return Ok(album);
        }
    }
    Err(())
}

fn metric(metric_db: &Metrics, user: &String, route: &String) {
    let _ = metric_db.note_route(route, user);
}
