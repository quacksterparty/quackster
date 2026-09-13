use serde::{Deserialize, Serialize};

use super::media::Media;

/// Translatable subset of a question — no `correct`, `position`, numeric
/// `answer`, variant config, or metadata fields.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct QuestionOverlay {
    pub id: String,
    pub content: ContentOverlay,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct ContentOverlay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PromptOverlay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<OrderItemOverlay>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<VariantsOverlay>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct PromptOverlay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<Media>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct VariantsOverlay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiple_choice: Option<MultipleChoiceOverlay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<OpenVariantOverlay>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct MultipleChoiceOverlay {
    pub choices: Vec<ChoiceOverlay>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct ChoiceOverlay {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<Media>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct OpenVariantOverlay {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "Overlays.ts"))]
#[serde(deny_unknown_fields)]
pub struct OrderItemOverlay {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<Media>>,
}
