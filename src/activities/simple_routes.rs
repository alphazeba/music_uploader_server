use crate::authenticated::Authenticated;
use rocket::get;

#[get("/auth")]
pub fn check_auth(_auth: Authenticated) -> &'static str {
    println!("{} checked auth", _auth.username);
    "hello"
}

#[get("/conn")]
pub fn check_conn() -> &'static str {
    "hello"
}
