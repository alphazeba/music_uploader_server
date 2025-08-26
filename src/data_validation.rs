use rocket::{data::ByteUnit, Data};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use crate::model::MusicUploaderError;

pub fn check_hash(expected_hash: &String, data: &[u8]) -> Result<(), MusicUploaderError> {
    let computed_hash = sha256::digest(data);
    if expected_hash == &computed_hash {
        Ok(())
    } else {
        Err(MusicUploaderError::ConstraintViolation(
            "Hash check failed".to_string(),
        ))
    }
}

pub async fn read_in_complete_data(
    data: Data<'_>,
    max_bytes: ByteUnit,
) -> Result<Vec<u8>, MusicUploaderError> {
    let incoming_data = data.open(max_bytes);
    let bytes = incoming_data
        .into_bytes()
        .await
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    if !bytes.is_complete() {
        return Err(MusicUploaderError::ConstraintViolation(
            "File uploaded is too large".to_string(),
        ));
    }
    Ok(bytes.value)
}

pub fn write_bytes_to_new_file(file_path: PathBuf, bytes: &[u8]) -> Result<(), MusicUploaderError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()))?;
    let _ = file
        .write_all(&bytes)
        .map_err(|e| MusicUploaderError::InternalServerError(e.to_string()));
    Ok(())
}

pub fn read_bytes_from_file(
    file_path: PathBuf,
    expected_size: usize,
) -> Result<Vec<u8>, MusicUploaderError> {
    let mut file = File::open(file_path).map_err(|e| {
        MusicUploaderError::InternalServerError(format!("Failed to read file part: {e}"))
    })?;
    let mut bytes: Vec<u8> = Vec::with_capacity(expected_size);
    let _ = file.read_to_end(&mut bytes).map_err(|e| {
        MusicUploaderError::InternalServerError(format!("failed to read bytes of parts file: {e}"))
    })?;
    Ok(bytes)
}
