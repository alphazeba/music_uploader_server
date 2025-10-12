use std::collections::HashSet;

use rusqlite::{params, Connection, Params, Row};

use crate::time_utils::get_now_timestamp;

pub struct OperationalData {
    conn: Connection,
}

impl OperationalData {
    pub fn new(data_path: &String) -> Self {
        let me = Self {
            conn: Connection::open(data_path).expect("failed to open sqlite file"),
        };
        me.get_conn()
            .execute(
                "create table if not exists uploadDeclaration \
                (key TEXT not null PRIMARY KEY, \
                declaredSize INTEGER not null, \
                partSize INTEGER not null, \
                path TEXT not null, \
                hash TEXT not null, \
                timestamp DATE not null)",
                [],
            )
            .expect("could not create table :(");
        me.get_conn()
            .execute(
                "create table if not exists uploadPart \
                (parentKey TEXT not null, \
                pindex INTEGER not null, \
                partHash TEXT not null, \
                timestamp DATE not null, \
                PRIMARY KEY (parentKey, pindex))",
                [],
            )
            .expect("could not create table :(");
        me.get_conn()
            .execute(
                "create table if not exists lastKnownPlaylist \
                (title TEXT not null PRIMARY KEY, \
                subscriberIds TEXT not null \
                timestamp DATE not null)",
                [],
            )
            .expect("could not create lastKnownState table");
        me.get_conn()
            .execute(
                "create table if not exists lastKnownSong \
                (plTitle TEXT not null, \
                songId TEXT not null, \
                timestamp DATE not null, \
                PRIMARY KEY (plTitle, songId))",
                [],
            )
            .expect("could not create lastKnownSong");
        me
    }

    fn get_conn(&self) -> &Connection {
        &self.conn
    }

    /// takes ownership of the passed items to help make it more obvious that you should use results of this call
    /// instead of the previously assumed declared size and such.
    pub fn declare_or_get_previous_upload(
        &self,
        hash: String,
        declared_size_bytes: u32,
        part_size_bytes: u32,
        path: String,
    ) -> Option<UploadDeclarationItem> {
        let key = Self::build_key(&hash);
        // check if the upload is new.
        if let Some(previous_item) = self.get_upload_declaration(&key) {
            return Some(previous_item);
        };
        let timestamp = get_now_timestamp();
        match self.get_conn().execute(
            "insert into uploadDeclaration \
            (key, hash, declaredSize, partSize, path, timestamp) \
            values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                key,
                hash,
                declared_size_bytes,
                part_size_bytes,
                path,
                timestamp
            ],
        ) {
            Ok(n) if n == 1 => Some(UploadDeclarationItem {
                key,
                hash,
                declared_size: declared_size_bytes,
                part_size: part_size_bytes,
                path,
                timestamp,
            }),
            Ok(n) => {
                println!("error creating upload declaration: did not get expected 1 row, created {n} rows");
                None
            }
            Err(e) => {
                println!("error creating upload declaration: {e}");
                None
            }
        }
    }

    fn build_key(hash: &str) -> String {
        hash.chars().take(30).collect()
    }

    pub fn get_upload_declaration(&self, key: &str) -> Option<UploadDeclarationItem> {
        self.get_conn()
            .query_row(
                "select key, hash, declaredSize, partSize, path, timestamp \
                    from uploadDeclaration where key=?1",
                params![key],
                |row| {
                    Ok(UploadDeclarationItem {
                        key: row.get(0)?,
                        hash: row.get(1)?,
                        declared_size: row.get(2)?,
                        part_size: row.get(3)?,
                        path: row.get(4)?,
                        timestamp: row.get(5)?,
                    })
                },
            )
            .ok()
    }

    pub fn is_part_present(&self, parent_key: &str, index: u32) -> bool {
        match self
            .get_conn()
            .query_row(
                "select count(*) from uploadPart where parentKey=?1 and pindex=?2",
                params![parent_key, index],
                |row| Ok(row.get::<usize, usize>(0)?),
            )
            .ok()
        {
            Some(items) => items > 0,
            None => false,
        }
    }

    pub fn add_part(
        &self,
        parent_key: &str,
        index: u32,
        part_hash: &str,
    ) -> Option<UploadPartItem> {
        let timestamp = get_now_timestamp();
        match self.get_conn().execute(
            "insert into uploadPart \
            (parentKey, pindex, partHash, timestamp) \
            values (?1, ?2, ?3, ?4)",
            params![parent_key, index, part_hash, get_now_timestamp()],
        ) {
            Ok(n) if n == 1 => Some(UploadPartItem {
                parent_key: parent_key.to_string(),
                index,
                part_hash: part_hash.to_string(),
                timestamp,
            }),
            Ok(n) => {
                println!(
                    "error creating upload part: did not get expected 1 row, created {n} rows"
                );
                None
            }
            Err(e) => {
                println!("error creating upload part: {e}");
                None
            }
        }
    }

    pub fn get_parts(&self, key: &str) -> Option<Vec<UploadPartItem>> {
        self.query_and_map(
            "get_parts",
            "select parentKey, pindex, partHash, timestamp \
                from uploadPart where parentKey=?1",
            params![key],
            |row| {
                Ok(UploadPartItem {
                    parent_key: row.get(0)?,
                    index: row.get(1)?,
                    part_hash: row.get(2)?,
                    timestamp: row.get(3)?,
                })
            },
        )
    }

    pub fn cleanup_upload(&self, key: &str) -> usize {
        match self
            .get_conn()
            .execute("delete from uploadDeclaration where key=?1", params![key])
        {
            Ok(n) if n == 1 => (),
            Ok(n) => {
                println!("Unexpected {n} updates when deleting {key} from uploadDeclaration")
            }
            Err(e) => {
                println!("Error deleting {key} from uploadDeclaration: {e}");
            }
        };
        match self
            .get_conn()
            .execute("delete from uploadPart where parentKey=?1", params![key])
        {
            Ok(n) => {
                println!("Deleted {n} rows for {key} from upload parts");
                n
            }
            Err(e) => {
                println!("Error deleting {key} from upload parts: {e}");
                0
            }
        }
    }

    pub fn get_last_known_playlist_state(&self) -> Option<Vec<LastKnownPlaylistState>> {
        let results = self.query_and_map(
            "lastKnownPlaylist",
            "select title, subscriberIds from lastKnownPlaylist",
            [],
            |row| {
                Ok(TitleSubscriber {
                    title: row.get(0)?,
                    unparsed_subscriber_ids: row.get(1)?,
                })
            },
        )?;
        let playlists = results
            .into_iter()
            .map(|item| {
                let song_ids = self
                    .get_playlist_songs(&item.title)?
                    .into_iter()
                    .collect::<HashSet<_>>();
                let subscriber_ids = Self::parse_subscriber_ids(item.unparsed_subscriber_ids);
                Some(LastKnownPlaylistState {
                    title: item.title,
                    subscriber_ids,
                    song_ids,
                })
            })
            .collect::<Option<Vec<_>>>();
        playlists
    }

    fn parse_subscriber_ids(subscriber_ids: String) -> Vec<String> {
        subscriber_ids.split(",").map(str::to_string).collect()
    }

    fn build_subscriber_ids(subscriber_ids: &[String]) -> String {
        subscriber_ids.join(",")
    }

    fn get_playlist_songs(&self, title: &str) -> Option<Vec<String>> {
        self.query_and_map(
            "get_playlist_songs",
            "select plTitle, songId from lastKnownSong where plTitle=?1",
            params![title],
            |row| {
                let song_id: String = row.get(1)?;
                Ok(song_id)
            },
        )
    }

    pub fn delete_last_known_playlist(&self, title: &str) -> Option<usize> {
        let conn = self.get_conn();
        let song_updates = conn
            .execute("delete from lastKnownSong where plTitle=?1", params![title])
            .inspect_err(|e| println!("failed to delete songs from deleted playlsit {title}: {e}"))
            .ok()?;
        let playlist_updates = conn
            .execute(
                "delete from lastKnownPlaylist where title=?1",
                params![title],
            )
            .inspect_err(|e| println!("faield to delete playlist {title}: {e}"))
            .ok()?;
        Some(song_updates + playlist_updates)
    }

    pub fn create_last_known_playlist(
        &self,
        title: &str,
        initial_subscriber_ids: &[String],
    ) -> Option<usize> {
        let formatted_subscriber_ids = Self::build_subscriber_ids(initial_subscriber_ids);
        self.get_conn()
            .execute(
                "insert into lastKnownPlaylist \
                (title, subscriberIds, timestamp) \
                values (?1, ?2, ?3)",
                params![title, formatted_subscriber_ids, get_now_timestamp()],
            )
            .inspect_err(|e| println!("failed to execute create_last_known_playlist: {e}"))
            .ok()
    }

    pub fn add_last_known_playlist_songs<'a, T>(&self, title: &str, song_ids: T) -> Option<usize>
    where
        T: Iterator<Item = &'a String>,
    {
        let sql = "insert into lastKnownSong \
            (plTitle, songId, timestamp) \
            values (?1, ?2, ?3)";
        let mut statement = self
            .get_conn()
            .prepare(sql)
            .inspect_err(|e| println!("Faield to preapre add_last_known_playlist_songs: {e}"))
            .ok()?;
        let changes = song_ids
            .filter_map(|song_id| {
                let params = params![title, song_id, get_now_timestamp()];
                statement
                    .execute(params)
                    .inspect_err(|e| {
                        println!(
                            "failed executing add_last_known_playlist_songs for {song_id}: {e}"
                        )
                    })
                    .ok()
            })
            .sum::<usize>();
        Some(changes)
    }

    pub fn remove_last_known_playlist_songs(
        &self,
        title: &str,
        song_ids: &[String],
    ) -> Option<usize> {
        let sql = "delete from lastKnownSong where plTitle=?1 and songId=?2";
        let mut statement = self
            .get_conn()
            .prepare(sql)
            .inspect_err(|e| println!("Faield to preapre remove_last_known_playlist_songs: {e}"))
            .ok()?;
        let changes = song_ids
            .iter()
            .filter_map(|song_id| {
                let params = params![title, song_id];
                statement
                    .execute(params)
                    .inspect_err(|e| {
                        println!(
                            "failed executing remove_last_known_playlist_songs for {song_id}: {e}"
                        )
                    })
                    .ok()
            })
            .sum::<usize>();
        Some(changes)
    }

    pub fn update_last_known_playlist_subscribers(
        &self,
        title: &str,
        subscriber_ids: &[String],
    ) -> Option<usize> {
        let formatted_subscriber_ids = Self::build_subscriber_ids(subscriber_ids);
        let sql = "update lastKnownPlaylist \
            SET subscriberIds=?1 \
            WHERE title=?2";
        self.get_conn()
            .execute(sql, params![formatted_subscriber_ids, title])
            .inspect_err(|e| {
                println!("failed to execute update_last_known_playlist_subscribers: {e}")
            })
            .ok()
    }

    fn query_and_map<T, P, F>(
        &self,
        title: &str,
        sql: &str,
        params: P,
        row_map_fn: F,
    ) -> Option<Vec<T>>
    where
        P: Params,
        F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
    {
        let mut query = self
            .get_conn()
            .prepare(sql)
            .inspect_err(|e| println!("error preparing query for {title}: {e}"))
            .ok()?;
        let results = query
            .query_map(params, row_map_fn)
            .inspect_err(|e| println!("error getting results for {title}: {e}"))
            .ok()?
            .collect::<Result<Vec<_>, _>>()
            .inspect_err(|e| println!("error mapping results for {title}: {e}"))
            .ok();
        results
    }
}

struct TitleSubscriber {
    title: String,
    unparsed_subscriber_ids: String,
}

pub struct LastKnownPlaylistState {
    pub title: String,
    pub subscriber_ids: Vec<String>,
    pub song_ids: HashSet<String>,
}

#[allow(unused)]
pub struct UploadDeclarationItem {
    pub key: String,
    pub hash: String,
    pub declared_size: u32,
    pub part_size: u32,
    pub path: String,
    pub timestamp: i64,
}

impl UploadDeclarationItem {
    pub fn get_expected_num_parts(&self) -> u32 {
        // am using f64s here so that the conversion is gauranteed.
        // could make these smaller if declared size and part size were made smaller.
        let f_declared_size: f64 = self.declared_size.into();
        let f_part_size: f64 = self.part_size.into();
        (f_declared_size / f_part_size).ceil() as u32
    }

    pub fn get_expected_index_size(&self, index: u32) -> u32 {
        let start = index * self.part_size;
        let end = u32::min((index + 1) * self.part_size, self.declared_size);
        match start < end {
            true => end - start,
            false => 0,
        }
    }
}

#[derive(PartialEq, PartialOrd, Eq)]
#[allow(unused)]
pub struct UploadPartItem {
    pub parent_key: String,
    pub index: u32,
    pub part_hash: String,
    pub timestamp: i64,
}

impl UploadPartItem {
    pub fn part_file_name(&self) -> String {
        format!("{}-{}", self.parent_key, self.index)
    }
}

impl Ord for UploadPartItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.index, &other.index)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn build_dummy_part(key: String, index: u32) -> UploadPartItem {
        let fake_hash = "fake hash".to_string();
        UploadPartItem {
            parent_key: key,
            index,
            part_hash: fake_hash,
            timestamp: get_now_timestamp(),
        }
    }

    #[test]
    fn test_sort_works() {
        let key = "dummy key";
        let indices: Vec<u32> = vec![1, 6, 5, 3, 2, 0, 4];
        let mut thing = indices
            .into_iter()
            .map(|index| build_dummy_part(key.to_string(), index))
            .collect::<Vec<_>>();
        thing.sort();
        let sorted_indices = thing.into_iter().map(|item| item.index).collect::<Vec<_>>();
        assert_eq!(sorted_indices, vec![0, 1, 2, 3, 4, 5, 6]);
    }
}
