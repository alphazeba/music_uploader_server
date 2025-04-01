use activities::{simple_routes::{check_auth, check_conn}, trigger_scan::trigger_scan, upload::upload};
use authenticated::Authenticator;
use config::server_config::ServerConfig;
use rocket::{catch, catchers, fairing::AdHoc, routes, Build, Rocket};
use std::env;

pub mod model;
mod path_utils;
mod authenticated;
mod config;
mod activities;
mod data;

#[catch(401)]
fn unauthorized() -> String {
    "request is not authorized".to_string()
}

pub fn build_rocket() -> Rocket<Build> {
    println!("starting musicuploader server, version: {}", env!("CARGO_PKG_VERSION"));
    config_env_or_panic();
    let authenticator= Authenticator::new()
        .expect("cannot run server without authenticator must look into issues");
    rocket::build()
        .register("/api", catchers![unauthorized])
        .mount("/api", routes![
            check_conn, 
            check_auth, 
            upload,
            trigger_scan,
        ])
        .attach(AdHoc::config::<ServerConfig>())
        .manage(authenticator)
}

pub fn config_env_or_panic() {
    let music_env = env::var("MUSIC_ENV").expect("MUSIC_ENV must be set");
    let _ = env::set_current_dir(music_env.clone()).expect(&format!("Failed to parse MUSIC_ENV ({}) as path", music_env));
        println!("current dir is: {:?}", env::current_dir());
}