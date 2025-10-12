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
