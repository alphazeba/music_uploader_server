use rocket::http::HeaderMap;

use crate::model::HeaderError;

pub fn get_header_string(headers: &HeaderMap, key: &str) -> Result<String, HeaderError> {
    Ok(headers
        .get_one(key)
        .ok_or(HeaderError::ParsingIssue)?
        .to_string())
}

pub fn get_header_u32(headers: &HeaderMap, key: &str) -> Result<u32, HeaderError> {
    Ok(headers.get_one(key)
        .ok_or(HeaderError::ParsingIssue)?
        .parse::<u32>()
        .map_err(|e| {
            println!("error: {e}");
            HeaderError::ParsingIssue
        })?
    )
}