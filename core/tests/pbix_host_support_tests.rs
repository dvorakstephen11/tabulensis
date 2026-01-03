mod common;

use common::fixture_path;
use excel_diff::{DiffConfig, DiffOp, PbixPackage, PackageError};
use std::fs::File;

#[test]
fn open_pbix_loads_datamashup() {
    let path = fixture_path("pbix_legacy_one_query_a.pbix");
    let file = File::open(&path).expect("fixture should exist");
    let pkg = PbixPackage::open(file).expect("pbix should parse");
    assert!(pkg.data_mashup().is_some(), "DataMashup should be present");
}

#[test]
fn diff_pbix_emits_query_ops() {
    let path_a = fixture_path("pbix_legacy_multi_query_a.pbix");
    let path_b = fixture_path("pbix_legacy_multi_query_b.pbix");
    let pkg_a = PbixPackage::open(File::open(&path_a).expect("fixture should exist"))
        .expect("pbix A should parse");
    let pkg_b = PbixPackage::open(File::open(&path_b).expect("fixture should exist"))
        .expect("pbix B should parse");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());
    let has_m_ops = report.ops.iter().any(DiffOp::is_m_op);
    assert!(has_m_ops, "expected at least one Power Query op");
}

#[test]
#[cfg(feature = "model-diff")]
fn pbix_missing_datamashup_uses_model_schema() {
    let path = fixture_path("pbix_no_datamashup.pbix");
    let file = File::open(&path).expect("fixture should exist");
    let pkg = PbixPackage::open(file).expect("pbix should parse with DataModelSchema");
    assert!(pkg.data_mashup().is_none(), "DataMashup should be missing");
}

#[test]
fn pbix_missing_datamashup_and_schema_returns_dedicated_error() {
    let path = fixture_path("pbix_no_datamashup_no_schema.pbix");
    let file = File::open(&path).expect("fixture should exist");
    let err = PbixPackage::open(file).expect_err("expected missing DataMashup error");
    assert!(matches!(err, PackageError::NoDataMashupUseTabularModel));
}
