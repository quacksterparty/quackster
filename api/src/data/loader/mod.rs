//! Walk data directories, parse YAML files, build registries.
//!

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fs;
use std::path::{Path, PathBuf};

use garde::Validate;
use serde::de::DeserializeOwned;

use super::error::LoadError;
use super::types::*;

mod games;
mod overlays;
mod packs;
mod questions;
mod tags;

#[cfg(test)]
mod tests;

enum Parsed<T> {
    Ok {
        file: String,
        items: Vec<T>,
        issues: Vec<LoadIssue>,
    },
    Err {
        issues: Vec<LoadIssue>,
    },
}

pub fn load_dataset(data_dir: &Path) -> Result<Dataset, LoadError> {
    sweep_stale_edit_tmpfiles(data_dir);
    let rel = |p: &Path| -> String {
        p.strip_prefix(data_dir)
            .unwrap_or(p)
            .to_string_lossy()
            .into_owned()
    };

    let (questions, q_issues) = questions::load_questions(data_dir, &rel)?;
    let (packs, p_issues) = packs::load_packs(data_dir, &rel)?;
    let (tags, t_issues) = tags::load_tags(data_dir, &rel)?;
    let (games, g_issues) = games::load_games(data_dir, &rel)?;
    let (overlays, o_issues) = overlays::load_overlays(data_dir, &rel)?;

    let issues = [q_issues, p_issues, t_issues, g_issues, o_issues].concat();

    let (questions, draft_questions) = partition_by_status(questions, |q| q.base().status);
    let (packs, draft_packs) = partition_by_status(packs, |p| p.status);
    let (games, draft_games) = partition_by_status(games, |g| g.status);

    Ok(Dataset {
        data_dir: data_dir.to_string_lossy().into_owned(),
        questions,
        packs,
        tags,
        games,
        overlays,
        issues,
        drafts: Drafts {
            questions: draft_questions,
            packs: draft_packs,
            games: draft_games,
        },
    })
}

fn partition_by_status<T>(
    reg: Registry<T>,
    split: impl Fn(&T) -> ContentStatus,
) -> (Registry<T>, Registry<T>) {
    let mut published = Registry::<T>::new();
    let mut drafts = Registry::<T>::new();
    for (id, entry) in reg {
        if matches!(split(&entry.item), ContentStatus::Draft) {
            drafts.insert(id, entry);
        } else {
            published.insert(id, entry);
        }
    }
    (published, drafts)
}

fn load_yaml_dir<T, Raw: DeserializeOwned>(
    dir: &Path,
    rel: &dyn Fn(&Path) -> String,
    decode: fn(Raw) -> Vec<T>,
    validate: Option<fn(&T, &str) -> Vec<LoadIssue>>,
    get_id: impl Fn(&T) -> &str,
    kind_label: &str,
) -> Result<(Registry<T>, Vec<LoadIssue>), LoadError> {
    let files = walk_yaml(dir)?;
    let results: Vec<_> = files
        .iter()
        .map(|path| parse_file(path, &rel(path), decode, validate))
        .collect();
    Ok(collect_registry(results, get_id, kind_label))
}

fn walk_yaml(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    walk_yaml_inner(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn sweep_stale_edit_tmpfiles(data_dir: &Path) {
    fn collect(path: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(path) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(".q-edit-"))
            {
                out.push(p);
            }
        }
    }
    let mut stale = Vec::new();
    collect(data_dir, &mut stale);
    for path in stale {
        match fs::remove_file(&path) {
            Ok(()) => tracing::warn!(path = %path.display(), "removed stale edit tmpfile from previous run"),
            Err(e) => tracing::warn!(error = %e, path = %path.display(), "could not remove stale edit tmpfile"),
        }
    }
}

fn walk_yaml_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), LoadError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_yaml_inner(&path, out)?;
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && !file_name.starts_with('.')
        {
            // Skip dotfiles: atomic-write tmp files (`.q-edit-*.yaml`) and any
            // editor backups (`.#foo.yaml.swp`) must never be loaded as content.
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy();
                if ext == "yaml" || ext == "yml" {
                    out.push(path);
                }
            }
        }
    }
    Ok(())
}

fn parse_file<T, Raw: DeserializeOwned>(
    path: &Path,
    rel_path: &str,
    decode: fn(Raw) -> Vec<T>,
    validate: Option<fn(&T, &str) -> Vec<LoadIssue>>,
) -> Parsed<T> {
    let raw = match parse_yaml_file::<Raw>(path) {
        Ok(v) => v,
        Err(e) => {
            return Parsed::Err {
                issues: vec![yaml_issue(rel_path, e)],
            };
        }
    };
    let items = decode(raw);
    let issues = match validate {
        Some(f) => items.iter().flat_map(|item| f(item, rel_path)).collect(),
        None => Vec::new(),
    };
    Parsed::Ok {
        file: rel_path.to_owned(),
        items,
        issues,
    }
}

fn collect_registry<T>(
    results: Vec<Parsed<T>>,
    get_id: impl Fn(&T) -> &str,
    kind_label: &str,
) -> (Registry<T>, Vec<LoadIssue>) {
    let mut registry = HashMap::new();
    let mut issues = Vec::new();

    for result in results {
        match result {
            Parsed::Err { issues: errs } => issues.extend(errs),
            Parsed::Ok {
                file,
                items,
                issues: validation_issues,
            } => {
                issues.extend(validation_issues);
                for item in items {
                    let id = get_id(&item).to_owned();
                    match registry.entry(id.clone()) {
                        Occupied(_) => {
                            issues.push(LoadIssue::msg(
                                &file,
                                format!("duplicate {kind_label} id '{id}'"),
                            ));
                        }
                        Vacant(entry) => {
                            entry.insert(Entry {
                                file: file.clone(),
                                item,
                            });
                        }
                    }
                }
            }
        }
    }

    (registry, issues)
}

fn parse_yaml_file<T: DeserializeOwned>(path: &Path) -> Result<T, serde_yaml::Error> {
    let text = fs::read_to_string(path)
        .map_err(|e| <serde_yaml::Error as serde::de::Error>::custom(format!("IO: {e}")))?;
    serde_yaml::from_str(&text)
}

fn yaml_issue(file: &str, e: serde_yaml::Error) -> LoadIssue {
    LoadIssue {
        file: file.to_owned(),
        message: e.to_string(),
        path: e
            .location()
            .map(|loc| format!("line {}, col {}", loc.line(), loc.column())),
    }
}

fn garde_issues<T: Validate<Context = ()>>(value: &T, file: &str) -> Vec<LoadIssue> {
    match value.validate() {
        Ok(()) => Vec::new(),
        Err(report) => report
            .iter()
            .map(|(path, error)| LoadIssue {
                file: file.to_owned(),
                message: error.message().to_string(),
                path: Some(path.to_string()),
            })
            .collect(),
    }
}
