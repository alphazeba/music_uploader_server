use std::path::{Path, PathBuf};

use reqwest::{Client as HttpClient, RequestBuilder};
use thiserror::Error;

use crate::clients::plex_model::{GetResources, GetUserInfo};

pub struct PlexClient {
    http_client: HttpClient,
    plex_token: String,
    plex_base: PathBuf,
}

const PLEX_TV_API: &str = "https://plex.tv/api/";

#[derive(Error, Debug)]
pub enum PlexClientError {
    #[error("Failed to build Url")]
    FailedToBuildUrl,
    #[error("Failed to send plex api request: {0}")]
    PlexApiSendFailure(String),
    #[error("Unable to understand plex response: {0}")]
    MisunderstoodPlexResponse(String),
    #[error("Unhappy plex respones: {0:?}")]
    UnhappyPlexResponse(reqwest::Response),
    #[error("XML parsing error")]
    XmlParse(#[from] roxmltree::Error),
}

pub type PlexClientResult<T> = Result<T, PlexClientError>;

impl PlexClient {
    pub fn new(plex_url: &str, plex_token: String) -> Self {
        Self {
            http_client: HttpClient::new(),
            plex_token,
            plex_base: Path::new(plex_url).to_owned(),
        }
    }

    pub async fn trigger_scan(&self, library_id: u16) -> PlexClientResult<String> {
        let url = self.build_local_url(&format!(
            "library/sections/{}/refresh",
            library_id.to_string()
        ))?;
        let request = self.http_client.get(url);
        self.send_with_server_token(request).await
    }

    pub async fn get_resources(&self) -> PlexClientResult<GetResources> {
        let url = Self::build_external_url("resources")?;
        let request = self.http_client.get(url);
        let response = self.send_with_server_token(request).await?;
        GetResources::from_xml(&response)
    }

    pub async fn get_user_info(&self, server_client_id: &str) -> PlexClientResult<GetUserInfo> {
        let url = Self::build_external_url(&format!("servers/{server_client_id}/shared_servers"))?;
        let request = self.http_client.get(url);
        let response = self.send_with_server_token(request).await?;
        GetUserInfo::from_xml(&response)
    }

    async fn send_with_server_token(&self, request: RequestBuilder) -> PlexClientResult<String> {
        Self::send_with_token(request, &self.plex_token).await
    }

    async fn send_with_token(request: RequestBuilder, token: &str) -> PlexClientResult<String> {
        let result = request
            .query(&[("X-Plex-Token", token)])
            .send()
            .await
            .map_err(|e| PlexClientError::PlexApiSendFailure(e.to_string()))?;
        if result.status().is_success() {
            result
                .text()
                .await
                .map_err(|e| PlexClientError::MisunderstoodPlexResponse(e.to_string()))
        } else {
            Err(PlexClientError::UnhappyPlexResponse(result))
        }
    }

    fn build_local_url(&self, path: &str) -> PlexClientResult<String> {
        self.plex_base
            .join(path)
            .to_str()
            .map(str::to_string)
            .ok_or(PlexClientError::FailedToBuildUrl)
    }

    fn build_external_url(path: &str) -> PlexClientResult<String> {
        Path::new(PLEX_TV_API)
            .join(path)
            .to_str()
            .map(str::to_string)
            .ok_or(PlexClientError::FailedToBuildUrl)
    }

    pub async fn add_songs_to_playlist(
        &self,
        server_identifier: &str,
        playlist_id: &str,
        owner_token: &String,
        song_ids: &[String],
    ) -> PlexClientResult<()> {
        // TODO need to set a maximum number of songs that can be added in a single call.
        let url = self.build_local_url(&format!("playlists/{playlist_id}/items"))?;
        let uri = build_song_uri(server_identifier, song_ids);
        println!("adding song uri: {uri} to playlist id: {playlist_id}");
        println!("using url: {url}");
        // maybe docs lied and you can't call query twice.
        let request = self
            .http_client
            .put(url)
            .query(&[("uri", &uri), ("X-Plex-Token", owner_token)]);
        let result = request.send().await.map_err(|e| {
            PlexClientError::PlexApiSendFailure(format!(
                "Failed to send add_songs_to_playlist: {e:?}"
            ))
        })?;
        if result.status().is_success() {
            Ok(())
        } else {
            Err(PlexClientError::UnhappyPlexResponse(result))
        }
    }

    /// DOES NOT USE song_id/metadata_id/rating_key!
    /// instead uses playlist_ids which sources from play_queue_generators.id
    pub async fn remove_songs_from_playlist(
        &self,
        playlist_id: &str,
        owner_token: &str,
        playlist_ids: &[String], // this is wrong
    ) -> PlexClientResult<()> {
        // TODO. when deleting items from a playlist, you do not use the rating key
        // you instead use the "playlistId" which is the first column in play_queue_generators.
        let url = self.build_local_url(&format!("playlists/{playlist_id}/items"))?;
        for playlist_id in playlist_ids {
            let item_url = format!("{url}/{playlist_id}");
            let request = self.http_client.delete(item_url);
            let _result = Self::send_with_token(request, owner_token).await?;
        }
        Ok(())
    }

    pub async fn clear_playlist(
        &self,
        playlist_id: &str,
        owner_token: &str,
    ) -> PlexClientResult<()> {
        let url = self.build_local_url(&format!("playlists/{playlist_id}/items"))?;
        let request = self.http_client.delete(url);
        let _result = Self::send_with_token(request, owner_token).await?;
        Ok(())
    }
}

fn build_song_uri(server_identifier: &str, song_ids: &[String]) -> String {
    let formatted_song_ids = song_ids.join(",");
    let uri_base = build_uri(server_identifier);
    format!("{uri_base}/library/metadata/{formatted_song_ids}")
}

fn build_uri(server_identifier: &str) -> String {
    format!("server://{server_identifier}/com.plexapp.plugins.library")
}

#[cfg(test)]
mod tests {
    use rocket::tokio;

    use crate::clients::plex_client::{PlexClient, PlexClientError};

    #[tokio::test]
    async fn test_add_song_does_not_get_builder_error() {
        let client = PlexClient::new("http://urmum", "urmum".to_string());
        let song_ids = vec!["123".to_string(), "432".to_string()];
        let Err(e) = client
            .add_songs_to_playlist("urmum", "123", &"urmum".to_string(), &song_ids)
            .await
        else {
            panic!("should have failed");
        };
        match e {
            PlexClientError::PlexApiSendFailure(message) => {
                println!("{message}");
                assert!(message.contains("dns error"));
            }
            _ => panic!("wrong error"),
        }
    }

    #[tokio::test]
    async fn test_add_song_does_not_get_builder_error_2() {
        let client = PlexClient::new("http://urmum", "urmum".to_string());
        let song_ids = vec!["123".to_string()];
        let Err(e) = client
            .add_songs_to_playlist("urmum", "123", &"urmum".to_string(), &song_ids)
            .await
        else {
            panic!("should have failed");
        };
        match e {
            PlexClientError::PlexApiSendFailure(message) => {
                println!("{message}");
                assert!(message.contains("dns error"));
            }
            _ => panic!("wrong error"),
        }
    }
}
