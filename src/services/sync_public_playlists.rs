use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use rocket::tokio;

use crate::{
    clients::{plex_client::PlexClient, plex_model::User},
    config::{load_toml, server_config::ServerConfig},
    data::{
        operational_data::{LastKnownPlaylistState, OperationalData},
        plex_db::{PlaylistResult, PlexDb},
    },
};

const ONE_HOUR_IN_SECONDS: u64 = 60 * 60;

pub fn start_sync_public_playlists() {
    tokio::spawn(sync_public_playlists());
}

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct DefaultServerConfig {
    default: ServerConfig,
}

async fn sync_public_playlists() {
    let default_server_config = load_toml::<DefaultServerConfig>("./Rocket.toml");
    let server_config = default_server_config.default;
    let state = Arc::new(State {
        plex_base: "test".to_string(),
        plex_token: server_config.plex_server_token,
        plex_db_path: server_config.plex_db_dir,
        operational_db_path: server_config.server_operational_db_dir,
    });
    loop {
        let job = state.build_job();
        match job.run().await {
            Ok(()) => println!("sync public playlists success"),
            Err(e) => println!("sync public playlists ERROR: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(ONE_HOUR_IN_SECONDS)).await;
    }
}

struct State {
    plex_token: String,
    plex_base: String,
    plex_db_path: String,
    operational_db_path: String,
}

struct PopulatedUserPlaylist {
    playlist: PlaylistResult,
    songs: HashSet<String>,
}

impl PopulatedUserPlaylist {
    pub fn id(&self) -> String {
        self.playlist.id.to_string()
    }
    pub fn owner_id(&self) -> String {
        self.playlist.owner_id.to_string()
    }
}

impl State {
    fn build_job(&self) -> Job {
        let client = PlexClient::new(&self.plex_base, self.plex_token.clone());
        Job {
            client,
            plex_token: self.plex_token.clone(),
            plex_db_path: self.plex_db_path.clone(),
            operational_db_path: self.operational_db_path.clone(),
        }
    }
}

// TODO: instead of storing a reference to dbs. should create and tear down dbs asap.

struct Job {
    client: PlexClient,
    plex_token: String,
    plex_db_path: String,
    operational_db_path: String,
}

impl Job {
    async fn run(&self) -> Result<(), String> {
        let (server_identifier, user_tokens) = self.get_token_data().await?;
        let plex_db = PlexDb::new(&self.plex_db_path);
        let operational_db = OperationalData::new(&self.operational_db_path);
        let public_user_playlists = Self::get_public_user_playlists(&plex_db)?;
        let last_known_playlist_states = Self::get_last_known_playlist_states(&operational_db)?;
        // at the end we can delete last known playlist state for playlists still in this list.
        let mut unused_last_known_playlist_titles = last_known_playlist_states
            .keys()
            .map(String::clone)
            .collect::<HashSet<_>>();

        for (title, user_playlists) in public_user_playlists {
            unused_last_known_playlist_titles.remove(&title);
            let mut populated_user_playlist =
                self.populate_songs_for_user_playlists(&plex_db, user_playlists)?;
            // hydrate user playlists
            // need to get the last known state
            let (canonical_song_list, playlists_to_nuke) =
                match last_known_playlist_states.get(&title) {
                    Some(last_known_playlist_state) => self.handle_update_playlist(
                        &operational_db,
                        &title,
                        populated_user_playlist.as_mut(),
                        last_known_playlist_state,
                        &user_tokens,
                    )?,
                    None => {
                        self.handle_new_playlist(&operational_db, &title, &populated_user_playlist)?
                    }
                };
            for nukable in playlists_to_nuke {
                self.client
                    .clear_playlist(&nukable.playlist_id, &nukable.user_token)
                    .await
                    .map_err(|e| format!("failed to nuke playlist: {e}"))?;
            }
            self.sync_users_to_song_list(
                canonical_song_list,
                &user_tokens,
                &populated_user_playlist,
                &server_identifier,
            )
            .await
            .map_err(|_e| "could not sync users and playlist")?;

            let new_subscriber_ids = populated_user_playlist
                .iter()
                .map(|item| item.owner_id().to_string())
                .collect::<Vec<_>>();
            let _changes = operational_db
                .update_last_known_playlist_subscribers(&title, &new_subscriber_ids)
                .ok_or("could not update playlist subscribers")?;
        }

        for unused_playlist in unused_last_known_playlist_titles {
            println!("deleting last known state for {unused_playlist} because there are no users with this playlist.");
            let _rows_deleted = operational_db
                .delete_last_known_playlist(&unused_playlist)
                .ok_or("Failed to delete unused last know playlist")?;
        }
        Ok(())
    }

    fn get_public_user_playlists(
        plex_db: &PlexDb,
    ) -> Result<HashMap<String, Vec<PlaylistResult>>, String> {
        let public_playlists = plex_db
            .get_public_user_playlists()
            .map_err(|e| e.to_string())?;
        Ok(Self::sort_and_digest_public_user_playlists(
            public_playlists,
        ))
    }

    fn get_last_known_playlist_states(
        operational_db: &OperationalData,
    ) -> Result<HashMap<String, LastKnownPlaylistState>, String> {
        let last_known_playlist_states = operational_db
            .get_last_known_playlist_state()
            .ok_or("failed to get public playlist")?
            .into_iter()
            .map(|item| {
                let key = item.title.clone();
                (key, item)
            })
            .collect::<HashMap<_, _>>();
        Ok(last_known_playlist_states)
    }

    fn handle_new_playlist(
        &self,
        operational_db: &OperationalData,
        title: &String,
        user_playlists: &[PopulatedUserPlaylist],
    ) -> Result<(HashSet<String>, Vec<PlaylistToNuke>), String> {
        let init = HashSet::new();
        let all_songs = user_playlists
            .iter()
            .fold(init, |mut sink, playlist| {
                sink.extend(playlist.songs.iter());
                sink
            })
            .into_iter()
            .map(String::to_string)
            .collect::<HashSet<_>>();
        // create a new known state.
        let user_ids = user_playlists
            .iter()
            .map(|item| item.owner_id().to_string())
            .collect::<Vec<_>>();
        let _rows_changed = operational_db
            .create_last_known_playlist(&title, &user_ids)
            .ok_or("could not create last known state playlist")?;
        let _rows_changed = operational_db.add_last_known_playlist_songs(&title, all_songs.iter());
        // update the user playlists to the canonical song list.
        // there should only be additions.
        Ok((all_songs, Vec::new()))
    }

    fn handle_update_playlist(
        &self,
        operational_db: &OperationalData,
        title: &String,
        user_playlists: &mut [PopulatedUserPlaylist],
        last_known_playlist_state: &LastKnownPlaylistState,
        user_tokens: &HashMap<String, User>,
    ) -> Result<(HashSet<String>, Vec<PlaylistToNuke>), String> {
        // this is an existing playlist.
        // for each user (that is in subscriber list)
        let mut canonical_song_list = last_known_playlist_state.song_ids.clone();
        let mut playlists_to_nuke = Vec::new();
        for user_playlist in user_playlists {
            let user_id = user_playlist.owner_id().to_string();
            let user_token = user_tokens
                .get(&user_id)
                .map(|user| user.access_token.to_string())
                .ok_or(format!("could not get token for user: {user_id}"))?;
            let is_old_subscriber = last_known_playlist_state.subscriber_ids.contains(&user_id);
            match is_old_subscriber {
                true => {
                    // this is an old subscriber.
                    let song_delta =
                        get_song_delta(&last_known_playlist_state.song_ids, &user_playlist.songs);
                    // missing songs should be removed from canonical list.
                    for missing_song in &song_delta.missing_songs {
                        canonical_song_list.remove(missing_song);
                    }
                    let missing_songs = song_delta.missing_songs.into_iter().collect::<Vec<_>>();
                    operational_db.remove_last_known_playlist_songs(&title, &missing_songs);
                    // additional songs should be added to the list.
                    let additional_songs =
                        song_delta.additional_songs.into_iter().collect::<Vec<_>>();
                    operational_db.add_last_known_playlist_songs(&title, additional_songs.iter());
                    canonical_song_list.extend(additional_songs.into_iter());
                }
                false => {
                    // this is a new user.
                    // their playlist should be erased.
                    user_playlist.songs.clear();
                    // the full playlist should be added to their playlist.
                    playlists_to_nuke.push(PlaylistToNuke {
                        user_token,
                        playlist_id: user_playlist.id().to_string(),
                    });
                }
            }
        }
        Ok((canonical_song_list, playlists_to_nuke))
    }

    fn populate_songs_for_user_playlists(
        &self,
        plex_db: &PlexDb,
        user_playlists: Vec<PlaylistResult>,
    ) -> Result<Vec<PopulatedUserPlaylist>, String> {
        user_playlists
            .into_iter()
            .map(|playlist| {
                let all_user_songs = plex_db
                    .get_playlist_songs(&playlist.id.to_string())
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .collect::<HashSet<_>>();
                Ok(PopulatedUserPlaylist {
                    songs: all_user_songs,
                    playlist,
                })
            })
            .collect()
    }

    async fn sync_users_to_song_list(
        &self,
        song_list: HashSet<String>,
        user_tokens: &HashMap<String, User>,
        user_playlists: &[PopulatedUserPlaylist],
        server_identifier: &str,
    ) -> Result<(), ()> {
        for user_playlist in user_playlists {
            let Some(user) = user_tokens.get(&user_playlist.owner_id()) else {
                println!("could not find user: {}", user_playlist.owner_id());
                continue;
            };
            let song_delta = get_song_delta(&song_list, &user_playlist.songs);
            let songs_to_add = song_delta.missing_songs.into_iter().collect::<Vec<_>>();
            let songs_to_remove = song_delta.additional_songs.into_iter().collect::<Vec<_>>();
            self.client
                .add_songs_to_playlist(
                    server_identifier,
                    &user_playlist.id(),
                    &user.access_token,
                    &songs_to_add,
                )
                .await
                .map_err(|e| println!("could not add songs to user: {e}"))?;
            self.client
                .remove_songs_from_playlist(
                    &user_playlist.id(),
                    &user.access_token,
                    &songs_to_remove,
                )
                .await
                .map_err(|e| println!("could not remove songs from user: {e}"))?
        }
        Ok(())
    }

    fn sort_and_digest_public_user_playlists(
        playlists: Vec<PlaylistResult>,
    ) -> HashMap<String, Vec<PlaylistResult>> {
        let mut playlists_by_key: HashMap<String, Vec<PlaylistResult>> = HashMap::new();
        for playlist in playlists {
            let key = &playlist.title;
            let Some(playlist_list) = playlists_by_key.get_mut(key) else {
                playlists_by_key.insert(key.clone(), vec![playlist]);
                continue;
            };
            playlist_list.push(playlist);
        }
        playlists_by_key
    }

    async fn get_token_data(&self) -> Result<(String, HashMap<String, User>), String> {
        let server_identifier = self.get_server_identifier().await?;
        let result = self
            .client
            .get_user_info(&server_identifier)
            .await
            .map_err(|e| e.to_string())?;
        let mut users = result.users;
        // server owner is not returned in this call
        // but their id is always 1 and plex token will act as their token
        // for the api.
        users.push(User {
            username: "server owner".to_string(),
            user_id: "1".to_string(),
            access_token: self.plex_token.clone(),
        });
        let user_tokens = users
            .into_iter()
            .map(|user| {
                let key = user.user_id.clone();
                (key, user)
            })
            .collect::<HashMap<_, _>>();
        Ok((server_identifier, user_tokens))
    }

    async fn get_server_identifier(&self) -> Result<String, String> {
        self.client
            .get_resources()
            .await
            .map_err(|e| e.to_string())?
            .devices
            .into_iter()
            .find(|item| item.product == "Plex Media Server")
            .map(|item| item.client_identifier)
            .ok_or("could not find da anaswer".to_string())
    }
}

struct PlaylistToNuke {
    user_token: String,
    playlist_id: String,
}

struct SongDelta {
    /// missing songs refers to songs that were in the last last state but not in the new state.
    pub missing_songs: HashSet<String>,
    /// additional songs are songs that are in the new state but not in the last state.
    pub additional_songs: HashSet<String>,
}

fn get_song_delta(
    last_state_songs: &HashSet<String>,
    new_state_songs: &HashSet<String>,
) -> SongDelta {
    let missing_songs = last_state_songs
        .difference(new_state_songs)
        .into_iter()
        .map(String::to_string)
        .collect::<HashSet<_>>();
    let additional_songs = new_state_songs
        .difference(last_state_songs)
        .into_iter()
        .map(String::to_string)
        .collect::<HashSet<String>>();
    SongDelta {
        missing_songs,
        additional_songs,
    }
}
