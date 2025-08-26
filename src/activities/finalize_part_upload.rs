use std::{fs, path::Path};

use rocket::State;

use crate::{
    config::server_config::ServerConfig,
    data::operational_data::{OperationalData, UploadDeclarationItem},
    data_validation::{check_hash, read_bytes_from_file, write_bytes_to_new_file},
    model::MusicUploaderError,
};

pub async fn finalize_part_upload(
    upload_declaration: UploadDeclarationItem,
    server_config: &State<ServerConfig>,
    operational_data: OperationalData,
) -> Result<(), MusicUploaderError> {
    let mut parts = operational_data.get_parts(&upload_declaration.key).ok_or(
        MusicUploaderError::InternalServerError(format!(
            "failed to get the parts for upload: {}",
            upload_declaration.key
        )),
    )?;
    parts.sort();
    let base_path = Path::new(&server_config.temp_file_dir);
    let bytes = parts
        .iter()
        .flat_map(|part| {
            let expected_size = upload_declaration.get_expected_index_size(part.index) as usize;
            let part_path = base_path.join(part.part_file_name());
            read_bytes_from_file(part_path, expected_size)
        })
        .flat_map(|bytes| bytes)
        .collect::<Vec<_>>();
    if upload_declaration.declared_size != bytes.len() as u32 {
        return Err(MusicUploaderError::ConstraintViolation(
            "total file is not the expected size".to_string(),
        ));
    }
    check_hash(&upload_declaration.hash, &bytes)?;
    // now we have verified everything. write to disk.
    write_bytes_to_new_file(upload_declaration.path.into(), &bytes)?;
    operational_data.cleanup_upload(&upload_declaration.key);
    // delete the temp files.
    parts.iter().for_each(|part| {
        let part_path = base_path.join(part.part_file_name());
        let _ = fs::remove_file(part_path)
            .inspect_err(|e| println!("failed to delete a temp file: {e}"));
    });
    Ok(())
}
