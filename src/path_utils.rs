use rocket::{futures::TryFutureExt, tokio::fs};
use std::{ffi::OsStr, fs::read_dir, path::Path};
use thiserror::Error;

use crate::server_config::ServerConfig;

#[derive(Error, Debug)]
pub enum ValidateDirectoryError {
    #[error("Failed to create directory")]
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

async fn validate_or_create_directory(directory: &String) -> Result<(), ValidateDirectoryError> {
    if !Path::new(directory).exists() {
        fs::create_dir(directory)
            .map_err(|e| ValidateDirectoryError::FailedToCreateDir(e))
            .await?;
    }
    Ok(())
}

fn get_extension(file_path: &String) -> Result<String, ValidateDirectoryError> {
    Path::new(file_path)
        .extension()
        .and_then(OsStr::to_str)
        .ok_or(ValidateDirectoryError::NoFileExtension)
        .map(|s| s.to_string())
}

async fn validate_file_does_not_exist(file_path: &String) -> Result<(), ValidateDirectoryError> {
    let path = Path::new(file_path);
    match path.exists() {
        true => Err(ValidateDirectoryError::FileAlreadyExists),
        false => Ok(()),
    }
}

fn validate_file_type(valid_extensions: &Vec<String>, file_name: &String) -> Result<(), ValidateDirectoryError> {
    let extension = get_extension(file_name)?;
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
) -> Result<String, ValidateDirectoryError> {
    validate_file_type(&server_config.valid_extensions, filename)?;
    let artist_dir = format!("{}/{}", server_config.upload_dir, artist);
    validate_or_create_directory(&artist_dir).await?;
    let album_dir = format!("{}/{}", artist_dir, album);
    validate_or_create_directory(&album_dir).await?;
    let full_dir = format!("{}/{}", album_dir, filename);
    validate_file_does_not_exist(&full_dir).await?;
    Ok(full_dir)
}

pub fn get_album_names(music_dir: &String) -> Result<Vec<String>, ValidateDirectoryError> {
    let path = Path::new(music_dir);
    let dirs = read_dir(path).map_err(|e| {
        println!("{:?}", e);
        ValidateDirectoryError::FailedToReadDir
    })?;
    let dirs = dirs.into_iter()
        .filter_map(|x| x
            .map_err(|e| println!("could not see file: {}", e)).ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return path.to_str().map(|s| s.to_string());
            } else {
                return None;
            }
        }).collect();
    Ok(dirs)
}