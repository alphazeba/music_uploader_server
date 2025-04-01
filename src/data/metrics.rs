use rusqlite::{params, Connection};
use time::OffsetDateTime;

pub struct Metrics {
    conn: Connection,
}

impl Metrics {
    pub fn new(data_path: &String) -> Self {
        let metrics = Self {
            conn: Connection::open(data_path).expect("failed to open sqlite file")
        };
        metrics.get_conn().execute("create table if not exists songUploads \
            (path TEXT not null PRIMARY KEY, user TEXT not null, timestamp DATE not null)",
            []).expect("could not create table :(");
        metrics.get_conn().execute("create table if not exists routeMetrics \
            (route TEXT not null, user TEXT not null, timestamp DATE not null)",
            []).expect("could not create table :(");
        metrics
    }

    fn get_conn(&self) -> &Connection {
        &self.conn
    }

    pub fn note_upload(&self, song_path: &String, user: &String) -> bool {
        match self.get_conn().execute("insert into songUploads \
            (user, path, timestamp) \
            values (?1, ?2, ?3)",
            params![user, song_path, get_now_timestamp()]
        ){
            Ok(_) => true,
            Err(e) => {
                println!("Failed to note upload: {:?}", e);
                false
            }
        }
    }

    pub fn note_route(&self, route: &String, user: &String) -> bool {
        match self.get_conn().execute("insert into routeMetrics \
                (route, user, timestamp) \
                values (?1, ?2, ?3)",
            params![route, user, get_now_timestamp()]
        ) {
            Ok(_) => true,
            Err(e) => {
                println!("Failed to note route: {:?}", e);
                false
            }
        }
    }
}

fn get_now_timestamp() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_note_upload() {
        let path = "./testDb.db".to_string();
        let db = Metrics::new(&path);
        let unique_song_name=  format!("fake song {}", OffsetDateTime::now_utc().to_string());
        let result = db.note_upload(&unique_song_name, &"fake user".to_string());
        assert!(result)
    }

    #[test]
    fn test_note_route() {
        let path = "./testDb.db".to_string();
        let db = Metrics::new(&path);
        let result = db.note_route(&"test route".to_string(), &"no one".to_string());
        assert!(result);
    }
}