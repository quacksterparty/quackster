use std::fs;
use std::path::PathBuf;

use crate::data::load_dataset;

#[test]
fn loads_real_dataset_without_issues() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data");
    if !path.exists() {
        return;
    }
    let mut ds = load_dataset(&path).expect("load_dataset failed");
    let cross = crate::data::validate::run_cross_file_checks(&ds);
    ds.issues.extend(cross);
    assert!(ds.issues.is_empty(), "real dataset issues: {:?}", ds.issues);
    assert!(ds.questions.len() >= 20);
    assert!(!ds.packs.is_empty());
    assert!(ds.tags.len() >= 10);
}

#[test]
fn sweep_removes_q_edit_sidecars_on_load() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("questions")).unwrap();
    fs::write(
        tmp.path().join("questions/q.yaml"),
        crate::data::test_helpers::VALID_QUESTION,
    )
    .unwrap();
    let stale = tmp.path().join("questions/.q-edit-abc123.yaml");
    fs::write(&stale, "- id: q_ghost\n  kind: text\n").unwrap();
    let swap = tmp.path().join("questions/.foo.yaml");
    fs::write(&swap, "swappy").unwrap();

    let ds = load_dataset(tmp.path()).expect("load");

    assert!(!stale.exists(), "stale .q-edit-* sidecar should be swept");
    assert!(swap.exists(), "non-sidecar dotfile must be left alone");
    assert!(
        !ds.questions.contains_key("q_ghost"),
        "sidecar must not have been parsed into the dataset"
    );
    assert!(ds.questions.contains_key("q_alpha_one"));
}
