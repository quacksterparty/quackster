use std::collections::BTreeMap;

use super::{Dataset, LocaleOverlays, Media, QuestionOverlay};

/// Normalize a practical BCP 47 tag. Full extension semantics are irrelevant for
/// content lookup; validated subtags still allow language, script, region, and variants.
pub fn normalize_locale(locale: &str) -> Option<String> {
    if locale.len() > 64 || locale.contains('_') {
        return None;
    }
    let mut parts = locale.split('-');
    let language = parts.next()?;
    if !(2..=3).contains(&language.len()) || !language.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    let mut normalized = vec![language.to_ascii_lowercase()];
    for part in parts {
        if part.is_empty() || part.len() > 8 || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
        normalized.push(
            if part.len() == 4 && part.chars().all(|c| c.is_ascii_alphabetic()) {
                let mut chars = part.to_ascii_lowercase().chars().collect::<Vec<_>>();
                chars[0].make_ascii_uppercase();
                chars.into_iter().collect()
            } else if (part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic()))
                || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit()))
            {
                part.to_ascii_uppercase()
            } else {
                part.to_ascii_lowercase()
            },
        );
    }
    Some(normalized.join("-"))
}

pub fn fallback_chain(locale: &str) -> Vec<String> {
    let Some(locale) = normalize_locale(locale) else {
        return Vec::new();
    };
    let mut parts: Vec<&str> = locale.split('-').collect();
    let mut chain = Vec::with_capacity(parts.len());
    while !parts.is_empty() {
        chain.push(parts.join("-"));
        parts.pop();
    }
    chain
}

impl Dataset {
    pub fn overlays_for(&self, locale: &str) -> Vec<&LocaleOverlays> {
        fallback_chain(locale)
            .into_iter()
            .filter_map(|wanted| {
                self.overlays
                    .iter()
                    .find(|(available, _)| available.eq_ignore_ascii_case(&wanted))
                    .map(|(_, overlays)| overlays)
            })
            .collect()
    }

    pub fn question_overlays(&self, locale: &str, question_id: &str) -> Vec<&QuestionOverlay> {
        self.overlays_for(locale)
            .into_iter()
            .filter_map(|overlays| overlays.questions.get(question_id).map(|entry| &entry.item))
            .collect()
    }

    pub fn localized_youtube_media(&self, locale: &str, question_ids: &[&str]) -> Vec<Media> {
        let mut media = BTreeMap::new();
        for question_id in question_ids {
            for overlay in self.question_overlays(locale, question_id) {
                let content = &overlay.content;
                let lists = content
                    .prompt
                    .iter()
                    .filter_map(|prompt| prompt.media.as_ref())
                    .chain(
                        content
                            .variants
                            .iter()
                            .filter_map(|variants| variants.multiple_choice.as_ref())
                            .flat_map(|multiple_choice| &multiple_choice.choices)
                            .filter_map(|choice| choice.media.as_ref()),
                    )
                    .chain(
                        content
                            .items
                            .iter()
                            .flatten()
                            .filter_map(|item| item.media.as_ref()),
                    );
                for item in lists
                    .flatten()
                    .filter(|item| item.media_ref.starts_with("youtube:"))
                {
                    media
                        .entry(item.media_ref.clone())
                        .or_insert_with(|| item.clone());
                }
            }
        }
        media.into_values().collect()
    }

    pub fn localized_category_name(
        &self,
        locale: &str,
        game_id: &str,
        game_idx: usize,
        category_idx: usize,
        canonical: &str,
    ) -> String {
        self.overlays_for(locale)
            .into_iter()
            .find_map(|overlays| {
                overlays
                    .games
                    .get(game_id)?
                    .item
                    .games
                    .get(game_idx)?
                    .board
                    .as_ref()?
                    .categories
                    .get(category_idx)?
                    .name
                    .clone()
            })
            .unwrap_or_else(|| canonical.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_builds_progressive_fallback() {
        assert_eq!(
            normalize_locale("ZH-hant-tw").as_deref(),
            Some("zh-Hant-TW")
        );
        assert_eq!(
            fallback_chain("zh-Hant-TW"),
            ["zh-Hant-TW", "zh-Hant", "zh"]
        );
    }

    #[test]
    fn rejects_malformed_locale() {
        for locale in ["de_DE", "d", "en-", "en-@"] {
            assert_eq!(normalize_locale(locale), None, "{locale}");
        }
    }

    #[test]
    fn script_subtag_title_cases_first_letter_only() {
        assert_eq!(normalize_locale("en-latn-us").as_deref(), Some("en-Latn-US"));
        assert_eq!(normalize_locale("sr-cyrl").as_deref(), Some("sr-Cyrl"));
        assert_eq!(normalize_locale("zh-hant").as_deref(), Some("zh-Hant"));
    }

    #[test]
    fn three_digit_region_passes_through() {
        assert_eq!(normalize_locale("en-001").as_deref(), Some("en-001"));
    }
}
