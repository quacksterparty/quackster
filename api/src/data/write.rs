//! Atomic YAML write helpers used by Phase C handlers.
//!
//! The contract: a write either fully lands or leaves the previous file
//! untouched. Crashes mid-write leave a sidecar tmp file (`.q-edit-*.yaml`);
//! `load_dataset` skips dotfiles on walk and sweeps stale sidecars at startup,
//! so they never reach the in-memory registries.

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

/// Read a list-shaped YAML file. Returns an empty `Vec` when the file is
/// missing so callers can treat "new file" and "existing list" uniformly.
pub fn load_list_or_empty<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, ReadError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    read_yaml_file::<Vec<T>>(path)
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

/// Recursively merge `patch` into `target` per RFC 7396. A `null` value in
/// the patch removes the field from the target. Non-object patch values
/// replace the target outright.
pub fn json_merge(target: &mut serde_json::Value, patch: serde_json::Value) {
    if let (Some(target_obj), Some(patch_obj)) = (target.as_object_mut(), patch.as_object()) {
        for (key, patch_val) in patch_obj {
            if patch_val.is_null() {
                target_obj.remove(key);
                continue;
            }
            if let Some(existing) = target_obj.get_mut(key) {
                json_merge(existing, patch_val.clone());
            } else {
                target_obj.insert(key.clone(), patch_val.clone());
            }
        }
    } else {
        *target = patch;
    }
}

#[cfg(test)]
mod tests;
