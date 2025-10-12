use std::{fs::File, io::Read};

use serde::de::DeserializeOwned;

pub mod secrets_config;
pub mod server_config;

pub fn load_toml<T: DeserializeOwned>(path: &str) -> T {
    let mut f = File::open(path).expect(&format!("failed to open {}", path));
    let mut file_text = String::new();
    f.read_to_string(&mut file_text)
        .expect("Failed reading the file");
    toml::from_str::<T>(&file_text).expect("failed parsing the toml")
}
