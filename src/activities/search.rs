use crate::{
    authenticated::Authenticated,
    config::server_config::ServerConfig,
    data::{
        metrics::Metrics,
        plex_db::{AlbumResult, PlexDb},
    },
    model::{AlbumSearchResponse, HeaderError, MusicUploaderError},
    rocket_utils::get_header_value,
};
use rocket::State;
use rocket::{
    get, http,
    request::{self, FromRequest},
    Request,
};
use rust_fuzzy_search::fuzzy_search_best_n;

pub struct AlbumSearchHeaders {
    album: String,
}

#[get("/albumsearch")]
pub async fn album_search(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: AlbumSearchHeaders,
) -> Result<AlbumSearchResponse, MusicUploaderError> {
    println!("{} is searching for {}", auth.username, headers.album);
    let plex_db = PlexDb::new(&server_config.plex_db_dir);
    let albums = plex_db.get_albums().map_err(|e| {
        println!("internal error with search");
        MusicUploaderError::InternalServerError(e.to_string())
    })?;
    let found_album = find_searched_album_result(albums, &headers.album).map_err(|_e| {
        MusicUploaderError::ConstraintViolation("could not find the searched for album".to_string())
    })?;
    let album_songs = plex_db
        .get_song_files_of_album(&found_album)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    let metric_db = Metrics::new(&server_config.server_db_dir);
    let upload_result = metric_db.get_upload(
        album_songs
            .get(0)
            .ok_or(MusicUploaderError::InternalServerError(
                "found album had no songs".to_string(),
            ))?
            .get_path(),
    );
    let response = AlbumSearchResponse {
        album: found_album.get_title().to_string(),
        uploader: match upload_result {
            Some(upload_result) => upload_result.user,
            None => {
                println!(
                    "there is no music upload data for ({})",
                    found_album.get_title()
                );
                "Unknown".to_string()
            }
        },
    };
    metric(&metric_db, &auth.username, &"albumsearch".to_string());
    println!(
        "found ({}) uploaded by {}",
        response.album, response.uploader
    );
    Ok(response)
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AlbumSearchHeaders {
    type Error = HeaderError;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match Self::from_request_inner(req).await {
            Ok(a) => request::Outcome::Success(a),
            Err(e) => request::Outcome::Error((http::Status::Unauthorized, e)),
        }
    }
}

impl<'r> AlbumSearchHeaders {
    async fn from_request_inner(req: &'r Request<'_>) -> Result<Self, HeaderError> {
        let headers = req.headers();
        Ok(AlbumSearchHeaders {
            album: get_header_value(headers, "album")?,
        })
    }
}
