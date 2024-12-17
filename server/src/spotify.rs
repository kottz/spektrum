use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::{Client, ClientBuilder};
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime};
use thiserror::Error;
use tracing::{error, warn};

#[derive(Error, Debug)]
pub enum SpotifyError {
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(#[from] std::env::VarError),
    #[error("Failed to get access token: {0}")]
    TokenError(String),
    #[error("Failed to send request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("No active device found")]
    NoActiveDevice,
    #[error("Invalid device info: {0}")]
    InvalidDeviceInfo(String),
    #[error("API request failed with status: {0}, message: {1}")]
    ApiError(u16, String),
    #[error("Rate limit exceeded, retry after: {0} seconds")]
    RateLimitExceeded(u64),
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
    last_api_call: Option<Instant>,
    rate_limit_window: Duration,
}

impl SpotifyController {
    pub async fn new() -> SpotifyResult<Self> {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID").map_err(|e| {
            error!("Failed to get SPOTIFY_CLIENT_ID: {:?}", e);
            e
        })?;
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").map_err(|e| {
            error!("Failed to get SPOTIFY_CLIENT_SECRET: {:?}", e);
            e
        })?;
        let refresh_token = std::env::var("SPOTIFY_REFRESH_TOKEN").map_err(|e| {
            error!("Failed to get SPOTIFY_REFRESH_TOKEN: {:?}", e);
            e
        })?;

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(SpotifyError::RequestError)?;

        let mut ctrl = Self {
            client,
            client_id,
            client_secret,
            refresh_token,
            token: None,
            token_expiry: None,
            last_api_call: None,
            rate_limit_window: Duration::from_millis(50),
        };

        if let Err(e) = ctrl.get_access_token().await {
            error!("Failed to get initial access token: {:?}", e);
        }
        Ok(ctrl)
    }

    async fn respect_rate_limit(&mut self) {
        if let Some(last_call) = self.last_api_call {
            let elapsed = last_call.elapsed();
            if elapsed < self.rate_limit_window {
                tokio::time::sleep(self.rate_limit_window - elapsed).await;
            }
        }
        self.last_api_call = Some(Instant::now());
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

        self.respect_rate_limit().await;

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
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();
            error!(
                "Failed to get Spotify access token. Status: {}, Body: {}",
                status, error_body
            );
            return Err(SpotifyError::TokenError(error_body));
        }

        let data: Value = res.json().await.map_err(SpotifyError::from)?;

        let access_token = data
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SpotifyError::TokenError("No access token in response".to_string()))?
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
        self.respect_rate_limit().await;
        let token = self.ensure_token().await?;

        let res = self
            .client
            .get("https://api.spotify.com/v1/me/player/devices")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(SpotifyError::from)?;

        let status = res.status();
        if !status.is_success() {
            let error_body = res.text().await.unwrap_or_default();
            return Err(SpotifyError::ApiError(status.as_u16(), error_body));
        }

        let data: Value = res.json().await.map_err(SpotifyError::from)?;
        let devices = data
            .get("devices")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                SpotifyError::InvalidDeviceInfo("No devices array in response".to_string())
            })?;

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
        match self._play_track(track_uri).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to play track {}: {:?}", track_uri, e);
                match e {
                    SpotifyError::TokenError(_) => {
                        self.token = None;
                        self._play_track(track_uri).await
                    }
                    _ => Ok(()), // Fail silently for playback issues
                }
            }
        }
    }

    async fn _play_track(&mut self, track_uri: &str) -> SpotifyResult<()> {
        self.respect_rate_limit().await;

        let active_device = match self.get_active_device().await {
            Ok(device) => device,
            Err(e) => {
                warn!("No active device found: {:?}", e);
                return Ok(()); // Fail silently for playback issues
            }
        };

        let device_id = active_device
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SpotifyError::InvalidDeviceInfo("No device ID".to_string()))?;

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
            let error_body = resp.text().await.unwrap_or_default();
            warn!("Spotify API error: {} - {}", status, error_body);
            return Ok(()); // Fail silently for playback issues
        }
        Ok(())
    }

    pub async fn pause(&mut self) -> SpotifyResult<()> {
        self.respect_rate_limit().await;

        let active_device = match self.get_active_device().await {
            Ok(device) => device,
            Err(e) => {
                warn!("No active device found when trying to pause: {:?}", e);
                return Ok(()); // Fail silently for playback issues
            }
        };

        let device_id = active_device
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SpotifyError::InvalidDeviceInfo("No device ID".to_string()))?;

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
            let error_body = resp.text().await.unwrap_or_default();
            warn!(
                "Spotify API error while pausing: {} - {}",
                status, error_body
            );
            return Ok(()); // Fail silently for playback issues
        }
        Ok(())
    }
}
