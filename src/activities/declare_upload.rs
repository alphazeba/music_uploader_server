use rocket::{http, post, request::{self, FromRequest}, Request, State};

use crate::{authenticated::Authenticated, config::server_config::ServerConfig, data::{metrics::Metrics, operational_data::{self, OperationalData}}, model::{DeclareUploadResponse, HeaderError, MusicUploaderError}, path_utils::{build_and_validate_path, ValidateDirectoryError}, rocket_utils::{get_header_string, get_header_u32}};

#[derive(Debug)]
pub struct DeclareUploadHeaders {
    hash: String,
    file_name: String,
    album: String,
    artist: String,
    declared_size: u32,
    part_size: u32,
}

#[post("/declareupload")]
pub async fn declare_upload(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: DeclareUploadHeaders,
) -> Result<DeclareUploadResponse, MusicUploaderError> {
    declare_upload_inner(auth, server_config, headers).await
}

pub async fn declare_upload_inner(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: DeclareUploadHeaders,
) -> Result<DeclareUploadResponse, MusicUploaderError> {
    let dir = build_and_validate_path(
        server_config, 
        &headers.artist, 
        &headers.album, 
        &headers.file_name
    )
    .await
    .map_err(|e| match e {
        ValidateDirectoryError::FileAlreadyExists => MusicUploaderError::SongAlreadyExists,
        e => MusicUploaderError::ValidateDirectoryError(Box::new(e)),
    })?;
    validate_inputs(&headers)?;
    let dir_str = dir.to_str().unwrap_or("<no dir?>").to_string();
    let username = auth.username;
    println!("new multi part upload from {username} using directory: {dir_str}");
    let operational_data = OperationalData::new(&server_config.server_operational_db_dir);
    let upload_declaration = operational_data
        .declare_or_get_previous_upload(headers.hash, headers.declared_size, headers.part_size, dir_str)
        .ok_or(MusicUploaderError::InternalServerError("Failed to declare upload in db".to_string()))?;
    let expected_num_parts = upload_declaration.get_expected_num_parts();
    let received_parts = get_received_parts(&operational_data, &upload_declaration.key)
        .map_err(|e| MusicUploaderError::InternalServerError(format!("Failed to parse received parts: {e:?}")))?;
    if received_parts.len() as u32 >= expected_num_parts {
        // we should run file finalization
        // then cleanup.
        // then return complete
        todo!();
        return Ok(DeclareUploadResponse::Complete);
    }
    metric(&server_config.server_db_dir, &username);
    Ok(DeclareUploadResponse::Incomplete {
        key: upload_declaration.key,
        declared_size: upload_declaration.declared_size,
        part_size: upload_declaration.part_size,
        received_parts,
    })
}

fn validate_inputs(headers: &DeclareUploadHeaders) -> Result<(), MusicUploaderError> {
    if headers.declared_size < headers.part_size {
        return Err(MusicUploaderError::ConstraintViolation("declared file size is smaller than part size".to_string()));
    }
    if headers.hash.chars().count() != 64 {
        return Err(MusicUploaderError::ConstraintViolation("malformed hash".to_string()));
    }
    Ok(())
}

#[derive(Debug)]
enum GetReceivedPartsError {
    TooLargeOfIndices,
    QueryError,
}

fn get_received_parts(operational_data: &OperationalData, key: &str) -> Result<Vec<u8>, GetReceivedPartsError> {
    operational_data.get_parts(key).ok_or(GetReceivedPartsError::QueryError)?
        .into_iter()
        .map(|item| {
            item.index.try_into().map_err(|e| {
                println!("error turning u32 into u8 as part of get recieved parts: {e}");
                GetReceivedPartsError::TooLargeOfIndices
            })
        }).collect()
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DeclareUploadHeaders {
    type Error = HeaderError;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match Self::from_request_inner(req).await {
            Ok(a) => request::Outcome::Success(a),
            Err(e) => request::Outcome::Error((http::Status::BadRequest, e)),
        }
    }
}

impl<'r> DeclareUploadHeaders {
    async fn from_request_inner(req: &'r Request<'_>) -> Result<Self, HeaderError> {
        let headers = req.headers();
        Ok(Self {
            hash: get_header_string(headers, "hash")?,
            file_name: get_header_string(headers, "file")?,
            album: get_header_string(headers, "album")?,
            artist: get_header_string(headers, "artist")?,
            declared_size: get_header_u32(headers, "declared_size")?,
            part_size: get_header_u32(headers, "part_size")?,
        })
    }
}

fn metric(db_path: &String, user: &String) {
    let metrics = Metrics::new(db_path);
    let _ = metrics.note_route(&"declareupload".to_string(), user);
}