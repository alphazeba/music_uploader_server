use std::str::FromStr;

use rocket::http::HeaderMap;

use crate::model::HeaderError;

fn get_header_str<'a>(headers: &'a HeaderMap, key: &str) -> Result<&'a str, HeaderError> {
    headers.get_one(key).ok_or_else(|| {
        println!("could not find key: {key}");
        HeaderError::ParsingIssue
    })
}

pub fn get_header_value<T>(headers: &HeaderMap, key: &str) -> Result<T, HeaderError>
where
    T: FromStr,
{
    let value = get_header_str(headers, key)?;
    Ok(value.parse::<T>().map_err(|_| {
        println!("error parsing value with key: {key}, value: {value}");
        HeaderError::ParsingIssue
    })?)
}
