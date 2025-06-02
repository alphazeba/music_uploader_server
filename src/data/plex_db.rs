use rusqlite::Connection;
use thiserror::Error;

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

    pub fn get_albums(&self) -> Result<Vec<AlbumResult>, PlexDbErr> {
        let mut query = self
            .get_conn()
            .prepare("select title, id from metadata_items where metadata_type = 9")
            .map_err(|e| PlexDbErr::PrepSqlFailure(e.to_string()))?;
        let result_iter = query
            .query_map([], |row| {
                Ok(AlbumResult {
                    album_title: row.get(0)?,
                    id: row.get(1)?,
                })
            })
            .map_err(|e| PlexDbErr::HandleQueryResultFailure(e.to_string()))?;
        let mut output = Vec::<AlbumResult>::new();
        for item in result_iter {
            match item {
                Ok(album) => output.push(album),
                Err(e) => println!("get album error: {:?}", e),
            }
        }
        match output.len() {
            0 => Err(PlexDbErr::NoResults),
            _ => Ok(output),
        }
    }

    pub fn get_song_files_of_album(
        &self,
        album: &AlbumResult,
    ) -> Result<Vec<SongResult>, PlexDbErr> {
        let mut query = self.get_conn().prepare(
            "select md_id, title, file \
                    from ( select title, id as md_id from metadata_items where parent_id = ?1) as md \
                    left outer join media_items on \
                        media_items.metadata_item_id = md.md_id \
                    left outer join media_parts on \
                        media_parts.media_item_id=media_items.id")
            .map_err(|e| PlexDbErr::PrepSqlFailure(e.to_string()))?;
        let result_iter = query
            .query_map([album.id], |row| {
                Ok(SongResult {
                    id: row.get(0)?,
                    song_title: row.get(1)?,
                    path: row.get(2)?,
                })
            })
            .map_err(|e| PlexDbErr::HandleQueryResultFailure(e.to_string()))?;
        let mut output = Vec::<SongResult>::new();
        for item in result_iter {
            match item {
                Ok(song) => output.push(song),
                Err(e) => println!("get song file error: {:?}", e),
            }
        }
        match output.len() {
            0 => Err(PlexDbErr::NoResults),
            _ => Ok(output),
        }
    }
}

#[derive(Error, Debug)]
pub enum PlexDbErr {
    #[error("failed to prepare sql statement: {0}")]
    PrepSqlFailure(String),
    #[error("Failed to handle results: {0}")]
    HandleQueryResultFailure(String),
    #[error("no results")]
    NoResults,
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
