use crate::types::*;
use anyhow::{Context, Result};
use md5;
use rand::Rng;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

const CLIENT_NAME: &str = "danavi";
const VERSION: &str = "1.16.1";

pub struct SubsonicClient {
    base_url: String,
    username: String,
    password: String,
    client: Client,
}

impl SubsonicClient {
    pub fn new(base_url: String, username: String, password: String) -> Result<Self> {
        // Remove trailing slash
        let base_url = base_url.trim_end_matches('/').to_string();

        let client = Client::builder()
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            base_url,
            username,
            password,
            client,
        })
    }

    fn generate_salt(&self) -> String {
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect()
    }

    fn generate_token(&self, salt: &str) -> String {
        let input = format!("{}{}", self.password, salt);
        format!("{:x}", md5::compute(input.as_bytes()))
    }

    async fn api_call(&self, endpoint: &str, params: &HashMap<&str, String>) -> Result<Value> {
        let salt = self.generate_salt();
        let token = self.generate_token(&salt);

        let mut url = Url::parse(&format!("{}/rest/{}", self.base_url, endpoint))
            .context("Invalid base URL")?;

        let mut query_params = params.clone();
        query_params.insert("u", self.username.clone());
        query_params.insert("t", token);
        query_params.insert("s", salt);
        query_params.insert("v", VERSION.to_string());
        query_params.insert("c", CLIENT_NAME.to_string());
        query_params.insert("f", "json".to_string());

        for (key, value) in query_params {
            url.query_pairs_mut().append_pair(key, &value);
        }

        let response = self
            .client
            .get(url.as_str())
            .send()
            .await
            .context("Failed to send request")?;

        let json: Value = response.json().await.context("Failed to parse response")?;

        let subsonic_response = json
            .get("subsonic-response")
            .context("Invalid response format")?;

        let status = subsonic_response
            .get("status")
            .and_then(|s| s.as_str())
            .context("Missing status field")?;

        if status == "ok" {
            Ok(subsonic_response.clone())
        } else {
            let error_msg = subsonic_response
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            anyhow::bail!("API error: {}", error_msg);
        }
    }

    /// Pings the server to check connectivity
    /// Useful for testing the connection
    /// Make Break ;)
    #[allow(dead_code)]
    pub async fn ping(&self) -> Result<()> {
        let params = HashMap::new();
        self.api_call("ping", &params).await?;
        Ok(())
    }

    pub async fn get_artists(&self) -> Result<ArtistsResponse> {
        let params = HashMap::new();
        let response = self.api_call("getArtists", &params).await?;
        serde_json::from_value(response).context("Failed to parse artists response")
    }

    pub async fn get_artist(&self, id: &str) -> Result<ArtistResponse> {
        let mut params = HashMap::new();
        params.insert("id", id.to_string());
        let response = self.api_call("getArtist", &params).await?;
        serde_json::from_value(response).context("Failed to parse artist response")
    }

    pub async fn get_album(&self, id: &str) -> Result<AlbumResponse> {
        let mut params = HashMap::new();
        params.insert("id", id.to_string());
        let response = self.api_call("getAlbum", &params).await?;
        serde_json::from_value(response).context("Failed to parse album response")
    }

    pub async fn search3(
        &self,
        query: &str,
        artist_count: u32,
        album_count: u32,
        song_count: u32,
    ) -> Result<SearchResponse> {
        let mut params = HashMap::new();
        params.insert("query", query.to_string());
        params.insert("artistCount", artist_count.to_string());
        params.insert("albumCount", album_count.to_string());
        params.insert("songCount", song_count.to_string());
        let response = self.api_call("search3", &params).await?;
        serde_json::from_value(response).context("Failed to parse search response")
    }

    /// Generates a streaming URL for a song
    /// This is used for audio playback
    #[allow(dead_code)] // Will be used when audio playback is implemented
    pub fn get_stream_url(&self, id: &str) -> String {
        let salt = self.generate_salt();
        let token = self.generate_token(&salt);
        format!(
            "{}/rest/stream?id={}&u={}&t={}&s={}&v={}&c={}&f=mp3",
            self.base_url, id, self.username, token, salt, VERSION, CLIENT_NAME
        )
    }
}
