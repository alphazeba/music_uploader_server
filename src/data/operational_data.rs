use rusqlite::{params, Connection};

use crate::time_utils::get_now_timestamp;

pub struct OperationalData {
    conn: Connection,
}

impl OperationalData {
    pub fn new(data_path: &String) -> Self {
        let me = Self {
            conn: Connection::open(data_path).expect("failed to open sqlite file"),
        };
        me
            .get_conn()
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
        me
            .get_conn()
            .execute(
                "create table if not exists uploadPart \
            (parentKey TEXT not null, \
                index INTEGER not null, \
                partHash TEXT not null, \
                timestamp DATE not null, \
                PRIMARY KEY (parentKey, index))",
                [],
            )
            .expect("could not create table :(");
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
        declared_size: u32,
        part_size: u32,
        path: String,
    ) -> Option<UploadDeclarationItem>{
        let key = Self::build_key(&hash);
        // check if the upload is new.
        if let Some(previous_item) = self.get_upload_declaration(&key) {
            return Some(previous_item);
        };
        let timestamp = get_now_timestamp();
        match self.get_conn().execute("insert into uploadDeclaration \
            (key, hash, declaredSize, partSize, path, timestamp) \
            values (?1, ?2, ?3, ?4, ?5, ?6)", 
            params![key, hash, declared_size, part_size, path, timestamp],
        ) {
            Ok(n) if n == 1 => Some(UploadDeclarationItem { 
                key,
                hash,
                declared_size,
                part_size,
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
            },
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
                }
            )
            .ok()
    }

    pub fn add_part(&self,
        parent_key: &str,
        index: u32,
        part_hash: &str,
    ) -> bool {
        match self.get_conn().execute("insert into uploadPart \
            (parentKey, index, partHash, timestamp) \
            values (?1, ?2, ?3, ?4)",
            params![parent_key, index, part_hash, get_now_timestamp()],
        ) {
            Ok(n) if n == 1 => true,
            Ok(n) => {
                println!("error creating upload part: did not get expected 1 row, created {n} rows");
                false
            }
            Err(e) => {
                println!("error creating upload part: {e}");
                false
            },
        }
    }

    pub fn get_num_parts(&self, key: &str) -> Option<usize> {
        self.get_conn().query_row("select count(*) from uploadPart where parentKey=?1", params![key], |row| {
            Ok(row.get(0)?)
        }).ok()
    }

    pub fn get_parts(&self, key: &str) -> Option<Vec<UploadPartItem>> {
        let mut query = self.get_conn().prepare("select parentKey, index, partHash, timestamp \
            from uploadPart where parentKey=?1"
        ).inspect_err(|e| println!("error preparing get_parts query: {e}")).ok()?;
        let result = query.query_map(params![key], |row| {
            Ok(UploadPartItem {
                parent_key: row.get(0)?,
                index: row.get(1)?,
                part_hash: row.get(2)?,
                timestamp: row.get(3)?,
            })
        }).inspect_err(|e| println!("Error making get parts query on {key}: {e}"))
        .ok()?
        .map(|item| item
            .inspect_err(|e| println!("error processing get_parts query result: {e}"))
            .ok())
        .collect::<Option<Vec<_>>>();
        result
    }

    pub fn cleanup_upload(&self, key: &str) -> usize {
        match self.get_conn().execute("delete from uploadDeclaration where key=?1", params![key]) {
            Ok(n) if n == 1 => (),
            Ok(n) => {
                println!("Unexpected {n} updates when deleting {key} from uploadDeclaration")
            }
            Err(e) => {
                println!("Error deleting {key} from uploadDeclaration: {e}");
            }
        };
        match self.get_conn().execute("delete from uploadPart where parentKey=?1", params![key]) {
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
        (f_declared_size/f_part_size).ceil() as u32
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

#[allow(unused)]
pub struct UploadPartItem {
    pub parent_key: String,
    pub index: u32,
    pub part_hash: String,
    pub timestamp: i64,
}
