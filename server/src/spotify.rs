use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tracing::{error, warn};

#[derive(Error, Debug)]
pub enum SpotifyError {
    #[error("Environment variable not found")]
    EnvVarNotFound(#[from] std::env::VarError),
    #[error("Failed to get access token")]
    TokenError,
    #[error("Failed to send request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("No active device found")]
    NoActiveDevice,
    #[error("Invalid device info")]
    InvalidDeviceInfo,
    #[error("API request failed with status: {0}")]
    ApiError(u16),
}

pub type SpotifyResult<T> = Result<T, SpotifyError>;

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
    pub async fn new() -> SpotifyResult<Self> {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID")?;
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")?;
        let refresh_token = std::env::var("SPOTIFY_REFRESH_TOKEN")?;

        let mut ctrl = Self {
            client: Client::new(),
            client_id,
            client_secret,
            refresh_token,
            token: None,
            token_expiry: None,
        };
        ctrl.get_access_token().await?;
        Ok(ctrl)
    }

    pub async fn get_access_token(&mut self) -> SpotifyResult<()> {
        if let Some(expiry) = self.token_expiry {
            if expiry
                .duration_since(SystemTime::now())
                .ok()
                .map_or(false, |d| d > Duration::from_secs(0))
            {
                return Ok(());
            }
        }

        let auth = STANDARD.encode(format!("{}:{}", self.client_id, self.client_secret));
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &self.refresh_token),
        ];

        let res = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", auth))
            .form(&params)
            .send()
            .await
            .map_err(SpotifyError::from)?;

        if !res.status().is_success() {
            error!(
                "Failed to get Spotify access token. Status: {}",
                res.status()
            );
            return Err(SpotifyError::TokenError);
        }

        let data: Value = res.json().await.map_err(SpotifyError::from)?;

        let access_token = data
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or(SpotifyError::TokenError)?
            .to_string();

        let expires_in = data
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);

        self.token = Some(access_token);
        self.token_expiry = Some(SystemTime::now() + Duration::from_secs(expires_in));
        Ok(())
    }

    async fn ensure_token(&mut self) -> SpotifyResult<String> {
        if let Some(expiry) = self.token_expiry {
            if expiry
                .duration_since(SystemTime::now())
                .ok()
                .map_or(false, |d| d > Duration::from_secs(0))
            {
                return Ok(self.token.clone().unwrap_or_default());
            }
        }
        self.get_access_token().await?;
        Ok(self.token.clone().unwrap_or_default())
    }

    pub async fn get_active_device(&mut self) -> SpotifyResult<Value> {
        let token = self.ensure_token().await?;

        let res = self
            .client
            .get("https://api.spotify.com/v1/me/player/devices")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(SpotifyError::from)?;

        if !res.status().is_success() {
            return Err(SpotifyError::ApiError(res.status().as_u16()));
        }

        let data: Value = res.json().await.map_err(SpotifyError::from)?;
        let devices = data
            .get("devices")
            .and_then(|v| v.as_array())
            .ok_or(SpotifyError::InvalidDeviceInfo)?;

        if devices.is_empty() {
            warn!("No devices found. Please open Spotify on a device.");
            return Err(SpotifyError::NoActiveDevice);
        }

        devices
            .iter()
            .find(|dev| {
                dev.get("is_active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .cloned()
            .ok_or(SpotifyError::NoActiveDevice)
    }

    pub async fn play_track(&mut self, track_uri: &str) -> SpotifyResult<()> {
        let active_device = self.get_active_device().await?;
        let device_id = active_device
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(SpotifyError::InvalidDeviceInfo)?;

        let token = self.ensure_token().await?;
        let url = format!(
            "https://api.spotify.com/v1/me/player/play?device_id={}",
            device_id
        );
        let body = serde_json::json!({ "uris": [track_uri] });

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
            .map_err(SpotifyError::from)?;

        let status = resp.status().as_u16();
        if ![204, 202].contains(&status) {
            return Err(SpotifyError::ApiError(status));
        }
        Ok(())
    }

    pub async fn pause(&mut self) -> SpotifyResult<()> {
        let active_device = self.get_active_device().await?;
        let device_id = active_device
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or(SpotifyError::InvalidDeviceInfo)?;

        let token = self.ensure_token().await?;

        let resp = self
            .client
            .put("https://api.spotify.com/v1/me/player/pause")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .query(&[("device_id", device_id)])
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(SpotifyError::from)?;

        let status = resp.status().as_u16();
        if ![200, 202, 204].contains(&status) {
            return Err(SpotifyError::ApiError(status));
        }
        Ok(())
    }
}
