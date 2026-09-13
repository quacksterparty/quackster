//! Transport layer — everything axum-aware. Routes, extractors, JSON, the WS
//! upgrade. This module is the ONLY place that imports `axum`.

pub mod auth;
pub mod error;
pub mod locale;
pub mod rest;
pub mod ws;
