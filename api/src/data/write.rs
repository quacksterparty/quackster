//! Atomic YAML write helpers used by Phase C handlers.
//!
//! The contract: a write either fully lands or leaves the previous file
//! untouched. Crashes mid-write leave a sidecar tmp file (started with
//! `.q-edit-`); the loader does not pick those up because `walk_yaml` filters
//! on `.yaml`/`.yml` only.

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("serialize: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("parent directory missing: {0}")]
    NoParent(String),
}

pub fn read_yaml_file<T: DeserializeOwned>(path: &Path) -> Result<T, ReadError> {
    let text = fs::read_to_string(path)?;
    let value = serde_yaml::from_str(&text)?;
    Ok(value)
}

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_yaml::Error),
}

pub fn write_yaml_file_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), WriteError> {
    let parent = path
        .parent()
        .ok_or_else(|| WriteError::NoParent(path.display().to_string()))?;
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)?;
    }

    let yaml = serde_yaml::to_string(value)?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".q-edit-")
        .suffix(".yaml")
        .tempfile_in(parent)?;
    tmp.write_all(yaml.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests;
