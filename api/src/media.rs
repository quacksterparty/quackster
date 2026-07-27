//! yt-dlp segment cache — the only way `youtube:` refs become playable.
//!
//! The client never sees youtube; refs resolve to files under the cache dir
//! (served at `/media-cache`) once downloaded. Downloads are prefetched at
//! room spawn (board is fixed then); a segment that isn't ready when its
//! question opens projects as no media — the agreed degrade path.

use std::{path::PathBuf, sync::Arc};

use tokio::sync::mpsc;

use crate::{
    config::AppConfig,
    data::{Media, MediaKind, parse_youtube_ref},
    protocol::{MediaFetchStatus, RoomMessage},
};

pub struct MediaFetcher {
    enabled: bool,
    ytdlp_path: String,
    cache_dir: PathBuf,
}

impl MediaFetcher {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.ytdlp_enabled,
            ytdlp_path: config.ytdlp_path.clone(),
            cache_dir: PathBuf::from(&config.media_cache_dir),
        }
    }

    #[cfg(test)]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ytdlp_path: String::new(),
            cache_dir: PathBuf::new(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Client URL for a cached segment; `None` when disabled, not a youtube
    /// ref, or not downloaded (yet).
    pub fn resolve(&self, media: &Media) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let name = cache_name(media)?;
        self.cache_dir
            .join(&name)
            .exists()
            .then(|| format!("/media-cache/{name}"))
    }

    // TODO: maybe parallelize if lobby-time prefetch is too slow
    // TODO: maybe cross-room in-flight dedup. Two rooms racing the same
    // ref may both spawn yt-dlp, worst case one fails (collision on the
    // target file), mod hits RetryMediaFetch, retry succeeds.
    pub fn prefetch(
        self: &Arc<Self>,
        media_list: Vec<Media>,
        command_tx: mpsc::Sender<RoomMessage>,
    ) {
        if !self.enabled || media_list.is_empty() {
            return;
        }
        let fetcher = Arc::clone(self);
        tokio::spawn(async move {
            for media in media_list {
                fetcher.run(&media, &command_tx).await;
            }
        });
    }

    async fn run(&self, media: &Media, command_tx: &mpsc::Sender<RoomMessage>) {
        let Some(cache_name) = cache_name(media) else {
            return;
        };

        if self.cache_dir.join(cache_name).exists() {
            self.report(command_tx, &media.media_ref, MediaFetchStatus::Ready)
                .await;
            return;
        }

        self.report(command_tx, &media.media_ref, MediaFetchStatus::Downloading)
            .await;

        match self.download(media).await {
            Ok(()) => {
                self.report(command_tx, &media.media_ref, MediaFetchStatus::Ready)
                    .await;
            }
            Err(error) => {
                self.report(
                    command_tx,
                    &media.media_ref,
                    MediaFetchStatus::Failed { message: error },
                )
                .await;
            }
        }
    }

    async fn report(
        &self,
        command_tx: &mpsc::Sender<RoomMessage>,
        media_ref: &str,
        status: MediaFetchStatus,
    ) {
        let _ = command_tx
            .send(RoomMessage::MediaStatus {
                media_ref: media_ref.to_string(),
                status,
            })
            .await;
    }

    async fn download(&self, media: &Media) -> Result<(), String> {
        let name = cache_name(media).ok_or("not a youtube ref")?;
        let final_path = self.cache_dir.join(&name);
        if final_path.exists() {
            return Ok(());
        }

        let (id, clip) =
            parse_youtube_ref(&media.media_ref).expect("cache_name already parsed the ref");

        // -o gets the stem only; yt-dlp fills %(ext)s and the forced
        // m4a/mp4 postprocessing makes the final name match `cache_name`.
        let stem = final_path.with_extension("");
        let mut cmd = tokio::process::Command::new(&self.ytdlp_path);
        match media.kind {
            MediaKind::Audio => cmd.args(["-f", "bestaudio/best", "-x", "--audio-format", "m4a"]),
            // Image can't be a youtube ref (validation rejects it)
            MediaKind::Video | MediaKind::Image => cmd.args([
                "-f",
                "bv*[height<=1080]+ba/b",
                "--merge-output-format",
                "mp4",
            ]),
        };
        if clip.start_ms.is_some() || clip.end_ms.is_some() {
            let start = clip.start_ms.unwrap_or(0) as f64 / 1000.0;
            let end = clip
                .end_ms
                .map(|value| (value as f64 / 1000.0).to_string())
                .unwrap_or_else(|| "inf".into());
            cmd.args([
                "--download-sections",
                &format!("*{start}-{end}"),
                // frame-accurate clip bounds, keyframe cuts can be seconds off
                "--force-keyframes-at-cuts",
            ]);
        }
        cmd.arg("-o")
            .arg(format!("{}.%(ext)s", stem.display()))
            .arg("--")
            .arg(id);

        tracing::info!(media_ref = %media.media_ref, "downloading youtube segment");
        let output = cmd.output().await.map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        Ok(())
    }
}

/// Deterministic cache filename: `{id}_{start}_{end}.{ext}` with bounds in
/// normalized ms (so `start=10` and `start=10.0` share a cache entry). `x`
/// marks an unset bound so clipped and unclipped segments never collide.
fn cache_name(media: &Media) -> Option<String> {
    let (id, clip) = parse_youtube_ref(&media.media_ref).ok()?;
    let ext = match media.kind {
        MediaKind::Audio => "m4a",
        MediaKind::Video | MediaKind::Image => "mp4",
    };
    let start = clip
        .start_ms
        .map_or_else(|| "x".into(), |value| value.to_string());
    let end = clip
        .end_ms
        .map_or_else(|| "x".into(), |value| value.to_string());
    Some(format!("{id}_{start}_{end}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(media_ref: &str, kind: MediaKind) -> Media {
        serde_yaml::from_str::<Media>(&format!(
            "kind: {}\nref: \"{media_ref}\"",
            match kind {
                MediaKind::Audio => "audio",
                MediaKind::Video => "video",
                MediaKind::Image => "image",
            }
        ))
        .expect("fixture parses")
    }

    #[test]
    fn cache_name_encodes_normalized_bounds() {
        let one_sided = media("youtube:abc123XYZ_-?start=10", MediaKind::Audio);
        assert_eq!(cache_name(&one_sided).unwrap(), "abc123XYZ_-_10000_x.m4a");

        let clip = media("youtube:abc123XYZ_-?start=0.5&end=9.5", MediaKind::Video);
        assert_eq!(cache_name(&clip).unwrap(), "abc123XYZ_-_500_9500.mp4");

        assert!(cache_name(&media("local:img/x.png", MediaKind::Image)).is_none());
    }

    #[test]
    fn disabled_fetcher_never_resolves() {
        let unclipped = media("youtube:abc123XYZ_-", MediaKind::Audio);
        assert!(MediaFetcher::disabled().resolve(&unclipped).is_none());
    }
}
