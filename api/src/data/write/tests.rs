use std::fs;

use serde::{Deserialize, Serialize};
use tempfile::tempdir;

use super::{read_yaml_file, write_yaml_file_atomic};

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
