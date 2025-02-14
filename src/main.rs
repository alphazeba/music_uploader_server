#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    music_uploader_server::build_rocket()
}
