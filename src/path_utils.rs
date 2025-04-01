use lazy_static::lazy_static;
use rocket::tokio::fs;
use std::path::PathBuf;
use std::{ffi::OsStr, path::Path};
use std::collections::HashSet;
use thiserror::Error;

use crate::config::server_config::ServerConfig;

lazy_static! {
    static ref LEGAL_CHARS: HashSet<char> = {
        let legal_chars = " abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890()_+=-!@#$?';\"<>";
        let mut set = HashSet::new();
        for c in legal_chars.chars() {
            set.insert(c);
        }
        set
    };
}
const REPLACE_CHAR: char = '_';

#[derive(Error, Debug)]
pub enum ValidateDirectoryError {
    #[error("Failed to create directory: {0}")]
    FailedToCreateDir(std::io::Error),
    #[error("File already exists")]
    FileAlreadyExists,
    #[error("Invalid extension")]
    InvalidExtension,
    #[error("there was no file extnesion")]
    NoFileExtension,
    #[error("failed to read the dir")]
    FailedToReadDir,
}

async fn validate_or_create_directory(base_path: &Path, new_dir: &String) -> Result<PathBuf, ValidateDirectoryError> {
    let path = base_path.join(clean_dir_segment(new_dir));
    if !path.exists() {
        fs::create_dir(&path)
            .await
            .map_err(|e| ValidateDirectoryError::FailedToCreateDir(e))?;
    }
    Ok(path)
}

async fn validate_file_does_not_exist(base_path: &Path, file_name: &String) -> Result<PathBuf, ValidateDirectoryError> {
    let path = base_path.join(
        clean_file_name(file_name)?
    );
    match path.exists() {
        true => Err(ValidateDirectoryError::FileAlreadyExists),
        false => Ok(path),
    }
}

fn get_file_stem_extension(file_path: &String) -> Result<(String, String), ValidateDirectoryError> {
    let path = Path::new(file_path);
    let file_stem = path.file_stem()
        .and_then(OsStr::to_str)
        .ok_or(ValidateDirectoryError::NoFileExtension)
        .map(|s| s.to_string())
        .map_err(|e| {
            println!("error getting file stem from {}: {}", file_path, e);
            e
        })?;
    let extension = path.extension()
        .and_then(OsStr::to_str)
        .ok_or(ValidateDirectoryError::NoFileExtension)
        .map(|s| s.to_string())
        .map_err(|e| {
            println!("error getting file extension from {}: {}", file_path, e);
            e
        })?;
    Ok((file_stem, extension))
}

fn clean_dir_segment(dir_segment: &String) -> String {
    let trimmed = dir_segment.trim();
    let filtered: String = trimmed.chars()
        .map(|c| match LEGAL_CHARS.contains(&c) {
            true => c,
            false => REPLACE_CHAR,
        }).collect();
    filtered
}

fn clean_file_name(file_name: &String) -> Result<String, ValidateDirectoryError> {
    let dir_escaped_file_name = file_name.replace('/', &REPLACE_CHAR.to_string());
    let (stem, extension) = get_file_stem_extension(&dir_escaped_file_name)?;
    Ok(format!("{}.{}", clean_dir_segment(&stem), clean_dir_segment(&extension)))
}

fn validate_file_type(valid_extensions: &Vec<String>, file_name: &String) -> Result<(), ValidateDirectoryError> {
    let (_, extension) = get_file_stem_extension(file_name)?;
    match valid_extensions.contains(&extension) {
        true => Ok(()),
        false => Err(ValidateDirectoryError::InvalidExtension),
    }
}

pub async fn build_and_validate_path(
    server_config: &ServerConfig,
    artist: &String,
    album: &String,
    filename: &String,
) -> Result<PathBuf, ValidateDirectoryError> {
    validate_file_type(&server_config.valid_extensions, filename)?;
    let base_path = Path::new(&server_config.upload_dir);
    let artist_path = validate_or_create_directory(base_path, artist).await?;
    let album_path = validate_or_create_directory(&artist_path, album).await?;
    let song_path = validate_file_does_not_exist(&album_path, filename).await?;
    Ok(song_path)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_file_names_that_should_not_change() {
        let examples = vec![
            "Jakub Zytecki - Remind Me - 02 Remind Me.mp3"
        ];
        for example in examples {
            assert_eq!(example.to_string(), clean_file_name(&example.to_string()).unwrap());
        }
    }

    #[test]
    fn test_dir_names_that_should_not_change() {
        let examples = vec![
            "Fearless (Taylor's Version)",
            "Charli XCX",
            "Death's Dynamic Shroud",
        ];
        for example in examples {
            assert_eq!(example.to_string(), clean_dir_segment(&example.to_string()));
        }
    }

    #[test]
    fn test_dir_names_that_should_change() {
        let examples = vec![
            "death’s dynamic shroud",
        ];
        for example in examples {
            assert_ne!(example.to_string(), clean_dir_segment(&example.to_string()));
        }
    }

    #[test]
    fn test_strings_are_stripped() {
        assert_eq!("_test_wav.mp3".to_string(), clean_file_name(&"  .test.wav.mp3   ".to_string()).unwrap());
        assert_eq!("thin_gy".to_string(), clean_dir_segment(&"   thin*gy    ".to_string()))
    }

    #[test]
    fn test_strings_are_escaped() {
        assert_eq!("__urmums_secret files_test_wav.mp3".to_string(), clean_file_name(&"  ~/urmums/secret files/test.wav.mp3   ".to_string()).unwrap());
        assert_eq!("thin_gy".to_string(), clean_dir_segment(&"   thin*gy    ".to_string()))
    }
}