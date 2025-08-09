use lazy_static::lazy_static;
use rocket::tokio::fs;
use std::collections::HashSet;
use std::path::PathBuf;
use std::{ffi::OsStr, path::Path};
use thiserror::Error;

use crate::config::server_config::ServerConfig;

const REPLACEMENT_CHAR: char = '_';
const DIR_SEGMENT_HASH_LENGTH: usize = 8;
lazy_static! {
    static ref LEGAL_CHARS: HashSet<char> = {
        let legal_chars =
            " abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890()_+=-!@#$?';\"<>";
        let mut set = HashSet::new();
        for c in legal_chars.chars() {
            set.insert(c);
        }
        set
    };
    static ref REPLACEMENT_CHAR_STR: String = REPLACEMENT_CHAR.to_string();
}

#[derive(Error, Debug)]
pub enum ValidateDirectoryError {
    #[error("Failed to create directory: {0}")]
    FailedToCreateDir(std::io::Error),
    #[error("File already exists")]
    FileAlreadyExists,
    #[error("Invalid extension")]
    InvalidExtension,
    #[error("there was no file extension")]
    NoFileExtension,
    #[error("failed to read the dir")]
    FailedToReadDir,
}

async fn validate_or_create_directory(
    base_path: &Path,
    new_dir: &String,
) -> Result<PathBuf, ValidateDirectoryError> {
    let path = base_path.join(clean_dir_segment(new_dir));
    if !path.exists() {
        fs::create_dir(&path)
            .await
            .map_err(|e| ValidateDirectoryError::FailedToCreateDir(e))?;
    }
    Ok(path)
}

async fn validate_file_does_not_exist(
    base_path: &Path,
    file_name: &String,
) -> Result<PathBuf, ValidateDirectoryError> {
    let path = base_path.join(clean_file_name(file_name)?);
    match path.exists() {
        true => Err(ValidateDirectoryError::FileAlreadyExists),
        false => Ok(path),
    }
}

fn get_file_stem_extension(file_path: &String) -> Result<(String, String), ValidateDirectoryError> {
    let path = Path::new(file_path);
    let file_stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or(ValidateDirectoryError::NoFileExtension)
        .map(|s| s.to_string())
        .map_err(|e| {
            println!("error getting file stem from {}: {}", file_path, e);
            e
        })?;
    let extension = path
        .extension()
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
    let mut num_chars = 0;
    let mut num_replaced_chars = 0;
    let trimmed = dir_segment.trim();
    let filtered: String = trimmed
        .chars()
        .map(|c| {
            num_chars += 1;
            match LEGAL_CHARS.contains(&c) {
                true => c,
                false => {
                    num_replaced_chars += 1;
                    REPLACEMENT_CHAR
                }
            }
        })
        .collect();
    // if we've replaced at least a third of the title lets add some entropy
    match num_chars / 3 <= num_replaced_chars {
        true => {
            let hash = get_dir_segment_hash(dir_segment);
            format!("{filtered}{hash}")
        }
        false => filtered,
    }
}

fn get_dir_segment_hash(string: &String) -> String {
    let hash = sha256::digest(string.as_bytes());
    // shorten the hash because a sha256 digest is 256 bits, 32 bytes, 64 hex characters.
    // most entire titles will not be that long.
    hash.chars()
        .take(DIR_SEGMENT_HASH_LENGTH)
        .collect::<String>()
}

fn clean_file_name(file_name: &String) -> Result<String, ValidateDirectoryError> {
    let dir_escaped_file_name = file_name.replace('/', &REPLACEMENT_CHAR_STR);
    let (stem, extension) = get_file_stem_extension(&dir_escaped_file_name)?;
    Ok(format!(
        "{}.{}",
        clean_dir_segment(&stem),
        clean_dir_segment(&extension)
    ))
}

fn validate_file_type(
    valid_extensions: &Vec<String>,
    file_name: &String,
) -> Result<(), ValidateDirectoryError> {
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
        let examples = vec!["Jakub Zytecki - Remind Me - 02 Remind Me.mp3"];
        for example in examples {
            assert_eq!(
                example.to_string(),
                clean_file_name(&example.to_string()).unwrap()
            );
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
        let examples = vec!["death’s dynamic shroud"];
        for example in examples {
            assert_ne!(example.to_string(), clean_dir_segment(&example.to_string()));
        }
    }

    #[test]
    fn test_strings_are_stripped() {
        assert_eq!(
            "_test_wav.mp3".to_string(),
            clean_file_name(&"  .test.wav.mp3   ".to_string()).unwrap()
        );
        assert_eq!(
            "thin_gy".to_string(),
            clean_dir_segment(&"   thin*gy    ".to_string())
        )
    }

    #[test]
    fn test_strings_are_escaped() {
        assert_eq!(
            "__urmums_secret files_test_wav.mp3",
            clean_file_name(&"  ~/urmums/secret files/test.wav.mp3   ".to_string()).unwrap()
        );
        assert_eq!("thin_gy", clean_dir_segment(&"   thin*gy    ".to_string()))
    }

    #[test]
    fn test_add_hash_to_high_replace_strings() {
        assert_eq!(
            "artist_album___________ec181730.mp3",
            clean_file_name(&"artist/album/^^^^^^^^^^.mp3".to_string()).unwrap(),
        );
        assert_eq!(
            "artist_album___________3faf76b4.mp3",
            clean_file_name(&"artist/album/&&&&&&&&&&.mp3".to_string()).unwrap(),
        );
    }
}
