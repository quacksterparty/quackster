use config::{Config, ConfigError, Environment};
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub admin_secret: Option<String>,

    #[serde(default)]
    pub ytdlp_enabled: bool,

    /// Binary to invoke when `ytdlp_enabled`; default resolves `yt-dlp` via PATH.
    #[serde(default = "default_ytdlp_path")]
    pub ytdlp_path: String,

    #[serde(default = "default_media_cache_dir")]
    pub media_cache_dir: String,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    3000
}
fn default_ytdlp_path() -> String {
    "yt-dlp".into()
}
fn default_media_cache_dir() -> String {
    "./cache/yt".into()
}

pub fn load() -> Result<AppConfig, ConfigError> {
    Config::builder()
        .add_source(Environment::with_prefix("APP"))
        .build()
        .and_then(|s| s.try_deserialize())
}
