use garde::Validate;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Packs.ts"))]
#[garde(allow_unvalidated)]
pub struct PackFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<QuestionKind>>,
    #[garde(custom(valid_opt_tag_refs))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags_all: Option<Vec<String>>,
    #[garde(custom(valid_opt_tag_refs))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags_any: Option<Vec<String>>,
    #[garde(custom(valid_opt_tag_refs))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags_none: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants_any: Option<Vec<VariantName>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Packs.ts"))]
#[garde(allow_unvalidated)]
#[garde(custom(pack_has_content))]
#[serde(deny_unknown_fields)]
pub struct Pack {
    #[garde(custom(valid_pack_id))]
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
    #[garde(custom(valid_opt_locale))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_lang: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_gamemodes: Option<Vec<String>>,
    #[garde(custom(valid_pack_ids))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub includes: Option<Vec<String>>,
    #[garde(custom(valid_opt_question_ids))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<String>>,
    #[garde(dive)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<PackFilter>,
    #[serde(default = "default_published")]
    pub status: ContentStatus,
}

fn pack_has_content(pack: &Pack, _ctx: &()) -> garde::Result {
    let has_includes = pack.includes.as_ref().is_some_and(|v| !v.is_empty());
    let has_questions = pack.questions.as_ref().is_some_and(|v| !v.is_empty());
    let has_filter = pack.filter.is_some();
    if has_includes || has_questions || has_filter {
        Ok(())
    } else {
        Err(garde::Error::new(
            "pack must define at least one of: includes, questions, filter",
        ))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[garde(allow_unvalidated)]
#[serde(deny_unknown_fields)]
pub struct PackOverlay {
    #[garde(custom(valid_pack_id))]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
