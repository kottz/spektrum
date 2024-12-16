use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, SystemTime};
use tracing::{error, warn};

#[derive(Clone)]
pub struct SpotifyController {
    pub client: Client,
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    pub token: Option<String>,
    pub token_expiry: Option<SystemTime>,
}

impl SpotifyController {
    pub async fn new() -> Option<Self> {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").ok()?;
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").ok()?;
        let refresh_token = std::env::var("SPOTIFY_REFRESH_TOKEN").ok()?;

        let mut ctrl = Self {
            client: Client::new(),
            client_id,
            client_secret,
            refresh_token,
            token: None,
            token_expiry: None,
        };
        if !ctrl.get_access_token().await {
            return None;
        }
        Some(ctrl)
    }

    pub async fn get_access_token(&mut self) -> bool {
        if let Some(expiry) = self.token_expiry {
            if let Ok(remaining) = expiry.duration_since(SystemTime::now()) {
                if remaining > Duration::from_secs(0) {
                    return true;
                }
            }
        }
        let auth = STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &self.refresh_token),
        ];

        let res = match self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", auth))
            .form(&params)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(_) => {
                error!("Failed to send token request.");
                return false;
            }
        };

        if !res.status().is_success() {
            error!(
                "Failed to get Spotify access token. Status: {}",
                res.status()
            );
            return false;
        }

        let data: Value = match res.json().await {
            Ok(d) => d,
            Err(_) => {
                error!("Failed to parse Spotify token response JSON.");
                return false;
            }
        };

        let access_token = match data.get("access_token").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                error!("No access_token field in Spotify token response.");
                return false;
            }
        };
        let expires_in = data
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        self.token = Some(access_token);
        self.token_expiry = Some(SystemTime::now() + Duration::from_secs(expires_in));
        true
    }

    async fn _check_token(&mut self) -> bool {
        if let Some(expiry) = self.token_expiry {
            if let Ok(remaining) = expiry.duration_since(SystemTime::now()) {
                if remaining > Duration::from_secs(0) {
                    return true;
                }
            }
        }
        self.get_access_token().await
    }

    pub async fn get_active_device(&mut self) -> Option<Value> {
        if !self._check_token().await {
            return None;
        }
        let token = self.token.clone()?;
        let res = self
            .client
            .get("https://api.spotify.com/v1/me/player/devices")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .ok()?;

        if !res.status().is_success() {
            error!("Error retrieving devices from Spotify: {}", res.status());
            return None;
        }
        let data: Value = res.json().await.ok()?;
        let devices = data.get("devices")?.as_array()?;
        if devices.is_empty() {
            warn!("No devices found. Please open Spotify on a device.");
            return None;
        }
        let active_device = devices.iter().find(|dev| {
            dev.get("is_active")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        });
        active_device.cloned()
    }

    pub async fn play_track(&mut self, track_uri: &str) -> bool {
        if !self._check_token().await {
            return false;
        }
        let active_device = match self.get_active_device().await {
            Some(d) => d,
            None => {
                warn!("No active Spotify device found. Cannot play track.");
                return false;
            }
        };
        let device_id = match active_device.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => {
                error!("No device id in active device info.");
                return false;
            }
        };

        let token = self.token.clone().unwrap_or_default();
        let url = format!(
            "https://api.spotify.com/v1/me/player/play?device_id={}",
            device_id
        );
        let body = serde_json::json!({ "uris": [track_uri] });

        let resp = match self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return false,
        };
        if ![204, 202].contains(&resp.status().as_u16()) {
            error!(
                "Failed to play track on Spotify. Status code: {}",
                resp.status()
            );
            return false;
        }
        true
    }

    pub async fn pause(&mut self) -> bool {
        if !self._check_token().await {
            return false;
        }
        let active_device = match self.get_active_device().await {
            Some(d) => d,
            None => {
                warn!("No active device to pause.");
                return false;
            }
        };
        let device_id = match active_device.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return false,
        };
        let token = self.token.clone().unwrap_or_default();
        let url = format!(
            "https://api.spotify.com/v1/me/player/pause?device_id={}",
            device_id
        );

        let resp = match self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return false,
        };
        [204, 202].contains(&resp.status().as_u16())
    }
}
