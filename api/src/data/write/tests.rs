use std::fs;

use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use super::{json_merge, load_list_or_empty, read_yaml_file, write_yaml_file_atomic};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    id: String,
    value: u32,
}

#[test]
fn writes_and_round_trips_yaml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested/points.yaml");
    let value = vec![
        Point {
            id: "a".into(),
            value: 1,
        },
        Point {
            id: "b".into(),
            value: 2,
        },
    ];

    write_yaml_file_atomic(&path, &value).expect("write failed");
    let read = read_yaml_file::<Vec<Point>>(&path).expect("read failed");
    assert_eq!(read, value);
}

#[test]
fn creates_missing_parent_directories() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("deep/down/here/item.yaml");
    write_yaml_file_atomic(
        &path,
        &Point {
            id: "x".into(),
            value: 42,
        },
    )
    .expect("write failed");
    assert!(path.exists());
}

#[test]
fn overwrite_replaces_existing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("item.yaml");
    let first = Point {
        id: "old".into(),
        value: 1,
    };
    let second = Point {
        id: "new".into(),
        value: 99,
    };
    write_yaml_file_atomic(&path, &first).unwrap();
    write_yaml_file_atomic(&path, &second).unwrap();
    let read = read_yaml_file::<Point>(&path).unwrap();
    assert_eq!(read, second);
}

#[test]
fn leaves_no_stray_tmp_on_success() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("item.yaml");
    write_yaml_file_atomic(&path, &Point { id: "x".into(), value: 1 }).unwrap();
    let strays: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".q-edit-")
        })
        .collect();
    assert!(strays.is_empty(), "found leftover tmp files");
}

#[test]
fn load_list_or_empty_returns_empty_when_file_missing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("absent.yaml");
    assert!(load_list_or_empty::<Point>(&path).unwrap().is_empty());
}

#[test]
fn load_list_or_empty_reads_existing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("items.yaml");
    write_yaml_file_atomic(
        &path,
        &vec![
            Point { id: "a".into(), value: 1 },
            Point { id: "b".into(), value: 2 },
        ],
    )
    .unwrap();
    assert_eq!(load_list_or_empty::<Point>(&path).unwrap().len(), 2);
}

#[test]
fn json_merge_rfc7396_object_recursion() {
    let mut target = serde_json::json!({ "a": 1, "nested": { "x": 1, "y": 2 } });
    let patch = serde_json::json!({ "nested": { "y": 99, "z": 3 }, "b": 2 });
    json_merge(&mut target, patch);
    assert_eq!(
        target,
        serde_json::json!({ "a": 1, "nested": { "x": 1, "y": 99, "z": 3 }, "b": 2 })
    );
}

#[test]
fn json_merge_null_removes_field() {
    let mut target = serde_json::json!({ "a": 1, "b": 2 });
    json_merge(&mut target, serde_json::json!({ "b": null }));
    assert_eq!(target, serde_json::json!({ "a": 1 }));
}

#[test]
fn json_merge_non_object_replaces() {
    let mut target = serde_json::json!({ "a": { "x": 1 } });
    json_merge(&mut target, serde_json::json!({ "a": 5 }));
    assert_eq!(target, serde_json::json!({ "a": 5 }));
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct MaybeItem {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[test]
fn omits_none_fields_when_writing_yaml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("item.yaml");
    let item = MaybeItem {
        id: "x".into(),
        name: None,
        note: None,
    };
    write_yaml_file_atomic(&path, &item).unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("id: x"), "raw was:\n{raw}");
    assert!(!raw.contains("name"), "name should be skipped, raw was:\n{raw}");
    assert!(!raw.contains("note"), "note should be skipped, raw was:\n{raw}");

    let back: MaybeItem = read_yaml_file(&path).unwrap();
    assert_eq!(back, item);
}
