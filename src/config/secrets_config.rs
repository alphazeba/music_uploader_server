use std::{fs::File, io::Read};

use rocket::serde::Deserialize;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Users {
    pub users: Vec<User>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct User {
    pub username: String,
    pub password: String,
}

pub fn load_users(path: &String) -> Users {
    let mut f = File::open(path).expect(&format!("failed to open {}", path));
    let mut file_text = String::new();
    f.read_to_string(&mut file_text).expect("Failed reading the file");
    let config = toml::from_str::<Users>(&file_text).expect("failed parsing the toml");
    println!("succesfully loaded users");
    config
}