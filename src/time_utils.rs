use time::OffsetDateTime;

pub fn get_now_timestamp() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
