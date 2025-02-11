use rocket::serde;

#[derive(serde::Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ServerConfig {
    pub upload_dir: String,
    pub valid_extensions: Vec<String>,
    pub max_mb: u32,
}