use std::env;

use music_uploader_server::clients::plex_client::{PlexClient, PlexClientError};
use rocket::tokio;

#[tokio::main]
async fn main() {
    let outcome = test_api().await;
    match outcome {
        Ok(()) => println!("SUCCESS!"),
        Err(e) => println!("failure: {e:?}"),
    }
}

async fn test_api() -> Result<(), PlexClientError> {
    let client = PlexClient::new(
        "urmum",
        env::var("PLEX_TOKEN").expect("PLEX_TOKEN must be present"),
    );
    let thing = client.get_resources().await?;
    let server_client_identifier = thing
        .devices
        .into_iter()
        .find(|item| item.product == "Plex Media Server")
        .map(|item| item.client_identifier)
        .ok_or(PlexClientError::MisunderstoodPlexResponse(
            "Could not find the Plex Media Server object".to_string(),
        ))?;
    let user_info = client
        .get_user_info(&server_client_identifier)
        .await?
        .users
        .into_iter()
        .find(|item| item.username == "RoyalSoup")
        .ok_or(PlexClientError::MisunderstoodPlexResponse(
            "Couldn't find user expected to exist".to_string(),
        ))?;
    println!("UserInfo: {user_info:?}");
    Ok(())
}
