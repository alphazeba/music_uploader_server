use std::path::Path;

use rocket::data::{Data, ToByteUnit};
use rocket::{
    http, post,
    request::{self, FromRequest},
    Request, State,
};

use crate::{
    authenticated::Authenticated,
    config::server_config::ServerConfig,
    data::operational_data::OperationalData,
    data_validation::{check_hash, read_in_complete_data, write_bytes_to_new_file},
    model::{HeaderError, MusicUploaderError},
    rocket_utils::get_header_value,
};

#[derive(Debug)]
pub struct UploadPartHeaders {
    key: String,
    part_hash: String,
    index: u8,
}

#[post("/uploadpart", data = "<data>")]
pub async fn upload_part(
    auth: Authenticated,
    server_config: &State<ServerConfig>,
    headers: UploadPartHeaders,
    data: Data<'_>,
) -> Result<(), MusicUploaderError> {
    println!(
        "\n{} is trying to upload part {:?}",
        &auth.username, headers
    );
    upload_part_inner(server_config, headers, data).await
}

async fn upload_part_inner(
    server_config: &State<ServerConfig>,
    headers: UploadPartHeaders,
    data: Data<'_>,
) -> Result<(), MusicUploaderError> {
    // step1, validate quick parameters
    // does key exist?
    let operational_data = OperationalData::new(&server_config.server_operational_db_dir);
    let upload_declaration = operational_data
        .get_upload_declaration(&headers.key)
        .ok_or(MusicUploaderError::ConstraintViolation(
            "No upload declaration for file part".to_string(),
        ))?;
    // is the index within the expected range?
    if upload_declaration.get_expected_num_parts() <= headers.index as u32 {
        return Err(MusicUploaderError::ConstraintViolation(
            "invalid part index".to_string(),
        ));
    }
    // is the index novel?
    if !operational_data.is_part_new(&headers.key, headers.index as u32) {
        return Err(MusicUploaderError::ConstraintViolation(
            "part has already been uploaded".to_string(),
        ));
    }
    // step 2 load the data, validate that it is an acceptable size.  It should be "expected size length".
    let bytes = read_in_complete_data(data, server_config.max_mb.megabytes()).await?;
    if upload_declaration.get_expected_index_size(headers.index as u32) != bytes.len() as u32 {
        return Err(MusicUploaderError::ConstraintViolation(
            "uploaded part is not expected size".to_string(),
        ));
    }
    // step 3 validate the hash
    check_hash(&headers.part_hash, &bytes)?;
    // step 4: update the database.
    let part = operational_data
        .add_part(&headers.key, headers.index as u32, &headers.part_hash)
        .ok_or(MusicUploaderError::InternalServerError(
            "Failed to add part to db".to_string(),
        ))?;
    // write the data to a file
    let file_path = Path::new(&server_config.temp_file_dir).join(part.part_file_name());
    write_bytes_to_new_file(file_path, &bytes)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UploadPartHeaders {
    type Error = HeaderError;
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match Self::from_request_inner(req).await {
            Ok(a) => request::Outcome::Success(a),
            Err(e) => request::Outcome::Error((http::Status::BadRequest, e)),
        }
    }
}

impl<'r> UploadPartHeaders {
    async fn from_request_inner(req: &'r Request<'_>) -> Result<Self, HeaderError> {
        let headers = req.headers();
        Ok(Self {
            key: get_header_value(headers, "key")?,
            part_hash: get_header_value(headers, "hash")?,
            index: get_header_value(headers, "index")?,
        })
    }
}
