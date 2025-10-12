use rusqlite::{params, Connection};

use crate::data::{query_and_map, DbErr};

// will handle queries against plex's db.

pub struct PlexDb {
    conn: Connection,
}

impl PlexDb {
    pub fn new(data_path: &String) -> Self {
        Self {
            conn: Connection::open(data_path).expect("failed to open plex sqlite fiel"),
        }
    }

    fn get_conn(&self) -> &Connection {
        &self.conn
    }

    pub fn get_albums(&self) -> Result<Vec<AlbumResult>, DbErr> {
        let mut query = self
            .get_conn()
            .prepare("select title, id from metadata_items where metadata_type = 9")
            .map_err(|e| DbErr::PrepSqlFailure(e.to_string()))?;
        let result_iter = query
            .query_map([], |row| {
                Ok(AlbumResult {
                    album_title: row.get(0)?,
                    id: row.get(1)?,
                })
            })
            .map_err(|e| DbErr::HandleQueryResultFailure(e.to_string()))?;
        let mut output = Vec::<AlbumResult>::new();
        for item in result_iter {
            match item {
                Ok(album) => output.push(album),
                Err(e) => println!("get album error: {:?}", e),
            }
        }
        match output.len() {
            0 => Err(DbErr::NoResults),
            _ => Ok(output),
        }
    }

    pub fn get_song_files_of_album(&self, album: &AlbumResult) -> Result<Vec<SongResult>, DbErr> {
        let mut query = self.get_conn().prepare(
            "select md_id, title, file \
                    from ( select title, id as md_id from metadata_items where parent_id = ?1) as md \
                    left outer join media_items on \
                        media_items.metadata_item_id = md.md_id \
                    left outer join media_parts on \
                        media_parts.media_item_id=media_items.id")
            .map_err(|e| DbErr::PrepSqlFailure(e.to_string()))?;
        let output = query
            .query_map([album.id], |row| {
                Ok(SongResult {
                    id: row.get(0)?,
                    song_title: row.get(1)?,
                    path: row.get(2)?,
                })
            })
            .map_err(|e| DbErr::HandleQueryResultFailure(e.to_string()))?
            .filter_map(|item| {
                item.inspect_err(|e| println!("get song file error: {e:?}"))
                    .ok()
            })
            .collect::<Vec<_>>();
        match output.len() {
            0 => Err(DbErr::NoResults),
            _ => Ok(output),
        }
    }

    pub fn get_public_user_playlists(&self) -> Result<Vec<PlaylistResult>, DbErr> {
        let mut query = self.get_conn().prepare(
            "select playlistId, ownerId, name, title from ( \
                select id as playlistId, title, json_extract(extra_data, \"$.pv:owner\") as ownerId \
                from metadata_items \
                where metadata_type = 15 and title like \"PUB: %\" \
            ) join accounts on ownerId = id"
        ).map_err(|e| DbErr::PrepSqlFailure(e.to_string()))?;
        let result = query
            .query_map([], |row| {
                Ok(PlaylistResult {
                    id: row.get(0)?,
                    owner_id: row.get(1)?,
                    owner_name: row.get(2)?,
                    title: row.get(3)?,
                })
            })
            .map_err(|e| DbErr::HandleQueryResultFailure(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DbErr::HandleQueryResultFailure(e.to_string()));
        result
    }

    pub fn get_playlist_songs(&self, playlist_id: &str) -> Result<Vec<String>, DbErr> {
        let songs = query_and_map(
            self.get_conn(),
            "get playlist songs",
            "select metadata_item_id from play_queue_generators where playlist_id=?1",
            params![playlist_id],
            |row| {
                let song_id: String = row.get(0)?;
                Ok(song_id)
            },
        );
        songs
    }
}

pub type MetadataId = i32;
pub struct AlbumResult {
    album_title: String,
    id: MetadataId,
}

impl AlbumResult {
    pub fn get_title(&self) -> &String {
        &self.album_title
    }
}

#[allow(unused)]
pub struct SongResult {
    song_title: String,
    path: String,
    id: MetadataId,
}

impl SongResult {
    pub fn get_path(&self) -> &String {
        &self.path
    }
}

// playlistId, ownerId, name, title
pub struct PlaylistResult {
    pub id: String,
    pub owner_id: String,
    #[allow(unused)]
    pub owner_name: String,
    pub title: String,
}
