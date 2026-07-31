use garde::Validate;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Audio,
    Video,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[garde(allow_unvalidated)]
#[garde(custom(valid_media))]
pub struct Media {
    pub kind: MediaKind,
    #[serde(rename = "ref")]
    pub media_ref: String,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// Clip bounds parsed from a `youtube:` ref's query, normalized to ms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct YoutubeClip {
    pub start_ms: Option<u32>,
    pub end_ms: Option<u32>,
}

/// Parse `youtube:<id>[?start=<s>&end=<s>]` into ID + clip bounds.
///
/// Bounds are decimal seconds in the ref (matches YouTube URL convention),
/// normalized to ms here so `start=10` and `start=10.0` are the same clip.
/// Only `start`/`end` params are allowed — typos and pasted tracking junk
/// must fail loudly instead of silently playing the full video.
pub fn parse_youtube_ref(value: &str) -> Result<(&str, YoutubeClip), String> {
    let rest = value.strip_prefix("youtube:").ok_or("not a youtube: ref")?;
    let (id, query) = match rest.split_once('?') {
        Some((id, query)) => (id, Some(query)),
        None => (rest, None),
    };
    // 11 chars today, but Google never promised — keep the range loose
    if !((8..=24).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    {
        return Err("youtube: ref must have an 8-24 char alphanumeric ID".into());
    }
    let mut clip = YoutubeClip::default();
    if let Some(query) = query {
        for pair in query.split('&') {
            let (key, raw) = pair
                .split_once('=')
                .ok_or_else(|| format!("malformed query param: {pair:?}"))?;
            let slot = match key {
                "start" => &mut clip.start_ms,
                "end" => &mut clip.end_ms,
                _ => {
                    return Err(format!(
                        "unknown query param: {key:?} (only start/end allowed)"
                    ));
                }
            };
            let seconds: f64 = raw
                .parse()
                .map_err(|_| format!("{key} must be a number, got: {raw:?}"))?;
            if !seconds.is_finite() || seconds < 0.0 {
                return Err(format!("{key} must be a non-negative number"));
            }
            if slot.replace((seconds * 1000.0).round() as u32).is_some() {
                return Err(format!("duplicate query param: {key}"));
            }
        }
        if let (Some(start), Some(end)) = (clip.start_ms, clip.end_ms)
            && start >= end
        {
            return Err("start must be less than end".into());
        }
    }
    Ok((id, clip))
}

fn valid_media(value: &Media, _ctx: &()) -> garde::Result {
    let media_ref = value.media_ref.as_str();
    // local: — relative path, no leading / or ..
    if let Some(sub) = media_ref.strip_prefix("local:") {
        if sub.is_empty() || sub.starts_with('/') || sub.contains("..") {
            return Err(garde::Error::new(
                "local: ref must be a relative path without leading / or ..",
            ));
        }
        return Ok(());
    }
    if media_ref.starts_with("url:https://") {
        return Ok(());
    }
    if media_ref.starts_with("youtube:") {
        // youtube is inherently temporal — no way to render it as an <img>
        if value.kind == MediaKind::Image {
            return Err(garde::Error::new("youtube: refs cannot be kind: image"));
        }
        return parse_youtube_ref(media_ref)
            .map(|_| ())
            .map_err(garde::Error::new);
    }
    Err(garde::Error::new(
        "media ref must start with local:, url:https://, or youtube:",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(kind: &str, media_ref: &str) -> Media {
        serde_yaml::from_str(&format!("kind: {kind}\nref: \"{media_ref}\""))
            .expect("fixture parses")
    }

    fn assert_valid(kind: &str, media_ref: &str) {
        media(kind, media_ref)
            .validate()
            .unwrap_or_else(|e| panic!("{media_ref} should be valid: {e}"));
    }

    fn assert_invalid(kind: &str, media_ref: &str) {
        assert!(
            media(kind, media_ref).validate().is_err(),
            "{media_ref} should be invalid"
        );
    }

    #[test]
    fn youtube_clip_params() {
        assert_eq!(
            parse_youtube_ref("youtube:dQw4w9WgXcQ?start=95.5&end=110").unwrap(),
            (
                "dQw4w9WgXcQ",
                YoutubeClip {
                    start_ms: Some(95_500),
                    end_ms: Some(110_000),
                }
            )
        );
        assert_eq!(
            parse_youtube_ref("youtube:dQw4w9WgXcQ").unwrap(),
            ("dQw4w9WgXcQ", YoutubeClip::default())
        );
        // one-sided bounds allowed
        assert_valid("audio", "youtube:dQw4w9WgXcQ?start=95");
        assert_valid("video", "youtube:dQw4w9WgXcQ?end=12");
    }

    #[test]
    fn youtube_rejects_bad_params() {
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?strat=10"); // typo
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?start=10&si=track"); // pasted junk
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?start=20&end=10"); // start >= end
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?start=-5");
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?start=abc");
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?start=1&start=2"); // duplicate
        assert_invalid("audio", "youtube:dQw4w9WgXcQ?");
        assert_invalid("audio", "youtube:x?start=10"); // bad ID
    }

    #[test]
    fn youtube_cannot_be_image() {
        assert_invalid("image", "youtube:dQw4w9WgXcQ");
        assert_valid("video", "youtube:dQw4w9WgXcQ");
        assert_valid("image", "local:img/x.png");
    }

    #[test]
    fn ref_grammar() {
        assert_valid("image", "url:https://example.org/x.png");
        assert_invalid("image", "url:http://example.org/x.png");
        assert_invalid("image", "local:/etc/passwd");
        assert_invalid("image", "local:../secrets.png");
        assert_invalid("image", "ftp://example.org/x.png");
    }
}
