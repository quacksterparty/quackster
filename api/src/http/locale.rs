use axum::http::{HeaderMap, header};

use crate::data::normalize_locale;

struct LocaleCandidate {
    quality: f32,
    header_order: usize,
    locale: String,
}

/// Select highest-quality requested locale accepted by endpoint-specific content.
pub fn preferred_locale(
    headers: &HeaderMap,
    available: impl Fn(&str) -> bool,
) -> Option<String> {
    let header = headers.get(header::ACCEPT_LANGUAGE)?.to_str().ok()?;
    let mut candidates: Vec<LocaleCandidate> = Vec::new();
    for (header_order, item) in header.split(',').enumerate() {
        let mut parts = item.trim().split(';');
        let Some(locale) = normalize_locale(parts.next()?.trim()) else {
            continue;
        };
        let mut quality = 1.0_f32;
        for parameter in parts {
            let Some((name, value)) = parameter.trim().split_once('=') else {
                continue;
            };
            if name.eq_ignore_ascii_case("q")
                && let Ok(q) = value.parse::<f32>()
                && (0.0..=1.0).contains(&q)
            {
                quality = q;
            }
        }
        if quality > 0.0 {
            candidates.push(LocaleCandidate {
                quality,
                header_order,
                locale,
            });
        }
    }
    candidates.sort_by(|a, b| {
        b.quality
            .total_cmp(&a.quality)
            .then(a.header_order.cmp(&b.header_order))
    });
    candidates
        .into_iter()
        .find_map(|candidate| available(&candidate.locale).then_some(candidate.locale))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT_LANGUAGE, value.parse().unwrap());
        headers
    }

    #[test]
    fn quality_order_and_normalization() {
        let headers = headers("fr;q=0.5, DE-ch;q=0.9");
        assert_eq!(
            preferred_locale(&headers, |locale| locale == "de-CH").as_deref(),
            Some("de-CH")
        );
    }

    #[test]
    fn ignores_invalid_and_zero_quality_ranges() {
        let headers = headers("de;q=0, invalid_locale, en");
        assert_eq!(
            preferred_locale(&headers, |locale| locale == "de"),
            None
        );
    }
}
