use std::path::{Path, PathBuf};

use excel_diff::{
    diff_pbip_snapshots, snapshot_pbip_project, PbipChangeKind, PbipNormalizationProfile,
    PbipScanConfig,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn fixtures_generated() -> PathBuf {
    repo_root().join("fixtures").join("generated")
}

fn pbip_fixture_dir(name: &str) -> PathBuf {
    fixtures_generated().join(name)
}

fn snapshot_fixture(path: &Path, profile: PbipNormalizationProfile) -> excel_diff::PbipProjectSnapshot {
    snapshot_pbip_project(path, profile, PbipScanConfig::default())
        .unwrap_or_else(|e| panic!("snapshot failed for {path:?}: {e}"))
}

#[test]
fn pbip_fixture_snapshot_is_deterministic_and_ignores_pbi_dir() {
    let root = pbip_fixture_dir("pbip_small_a");
    let snapshot = snapshot_fixture(&root, PbipNormalizationProfile::Balanced);

    // Paths should be stable (sorted, forward slashes).
    let paths = snapshot.docs.iter().map(|d| d.path.as_str()).collect::<Vec<_>>();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "expected docs to be path-sorted");
    assert!(
        paths.iter().all(|p| !p.contains('\\')),
        "expected forward slashes in rel paths: {paths:?}"
    );

    // `.pbi/` should be ignored by default.
    assert!(
        !paths.iter().any(|p| p.starts_with(".pbi/")),
        "expected .pbi directory to be ignored: {paths:?}"
    );

    // Normalization sanity-check: GUID normalization should apply to id-like keys but not others.
    let def = snapshot
        .docs
        .iter()
        .find(|d| d.path == "report/definition.pbir")
        .expect("definition.pbir present");
    let text = def.snapshot.normalized_text.as_str();
    assert!(text.contains("\"id\": \"GUID_000"), "expected id GUID placeholder: {text}");
    assert!(
        text.contains("\"objectId\": \"GUID_000"),
        "expected objectId GUID placeholder: {text}"
    );
    assert!(
        text.contains("\"guid\": \"GUID_000"),
        "expected guid GUID placeholder: {text}"
    );
    // Balanced mode should *not* normalize GUID-like strings under a non-id-like key.
    assert!(
        text.contains("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
        "expected non-id key GUID preserved in balanced mode: {text}"
    );
}

#[test]
fn pbip_fixture_diff_counts_match_expected_change_kinds() {
    let old_root = pbip_fixture_dir("pbip_small_a");
    let new_root = pbip_fixture_dir("pbip_small_b");

    let old = snapshot_fixture(&old_root, PbipNormalizationProfile::Balanced);
    let new = snapshot_fixture(&new_root, PbipNormalizationProfile::Balanced);

    let report = diff_pbip_snapshots(&old, &new);

    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;
    for doc in &report.docs {
        match doc.change_kind {
            PbipChangeKind::Added => added += 1,
            PbipChangeKind::Removed => removed += 1,
            PbipChangeKind::Modified => modified += 1,
            PbipChangeKind::Unchanged => {}
        }
    }

    assert_eq!(added, 2, "expected added docs: report/pages/page2.pbir + report/broken.pbir");
    assert_eq!(removed, 1, "expected removed doc: report/pages/page1.pbir");
    assert_eq!(modified, 2, "expected modified docs: report/definition.pbir + model/model.tmdl");

    // Entity diffs are best-effort, but should provide useful navigation on this fixture.
    assert!(
        report
            .entities
            .iter()
            .any(|e| e.doc_path == "report/definition.pbir"
                && matches!(e.entity_kind, excel_diff::PbipEntityKind::Page)
                && e.label == "Overview"
                && e.change_kind == PbipChangeKind::Removed),
        "expected a removed Page entity for Overview in report/definition.pbir"
    );
    assert!(
        report
            .entities
            .iter()
            .any(|e| e.doc_path == "report/definition.pbir"
                && matches!(e.entity_kind, excel_diff::PbipEntityKind::Page)
                && e.label == "Summary"
                && e.change_kind == PbipChangeKind::Added),
        "expected an added Page entity for Summary in report/definition.pbir"
    );
    assert!(
        report
            .entities
            .iter()
            .any(|e| e.doc_path == "model/model.tmdl"
                && matches!(e.entity_kind, excel_diff::PbipEntityKind::Measure)
                && e.label.ends_with(".TotalSales")
                && e.change_kind == PbipChangeKind::Added),
        "expected an added Measure entity for TotalSales in model/model.tmdl"
    );

    // Malformed JSON must not crash scanning; instead, it surfaces as a per-doc error.
    let broken = new
        .docs
        .iter()
        .find(|d| d.path == "report/broken.pbir")
        .expect("broken.pbir present");
    assert!(
        broken.snapshot.error.as_deref().unwrap_or("").contains("parse"),
        "expected parse error for broken.pbir: {:?}",
        broken.snapshot.error
    );
}
