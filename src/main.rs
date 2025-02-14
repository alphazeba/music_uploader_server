#[macro_use]
extern crate rocket;

// #[post("/scanplease")]
// async fn trigger_scan(
//     server_config: &State<ServerConfig>,
// ) -> 




#[launch]
fn rocket() -> _ {
    music_uploader_server::build_rocket()
}
