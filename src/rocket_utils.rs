use rocket::http::HeaderMap;

use crate::model::HeaderError;

pub fn get_header_string(headers: &HeaderMap, key: &str) -> Result<String, HeaderError> {
    Ok(headers
        .get_one(key)
        .ok_or(HeaderError::ParsingIssue)?
        .to_string())
}
