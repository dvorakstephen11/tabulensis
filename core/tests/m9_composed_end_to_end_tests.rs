use excel_diff::{DiffConfig, DiffOp, StepChange, StepDiff, WorkbookPackage};
use std::fs::File;

mod common;
use common::fixture_path;

fn load_package(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).expect("fixture file should open");
    WorkbookPackage::open(file).expect("fixture should parse as WorkbookPackage")
}

fn has_params_changed(detail: &excel_diff::QuerySemanticDetail) -> bool {
    detail.step_diffs.iter().any(|diff| match diff {
        StepDiff::StepModified { changes, .. } => {
            changes.iter().any(|c| matches!(c, StepChange::ParamsChanged))
        }
        _ => false,
    })
}

#[test]
fn composed_grid_mashup_reports_grid_and_query_changes() {
    let pkg_a = load_package("composed_grid_mashup_a.xlsx");
    let pkg_b = load_package("composed_grid_mashup_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());

    let has_grid_op = report.grid_ops().any(|op| {
        matches!(
            op,
            DiffOp::RowAdded { .. }
                | DiffOp::RowRemoved { .. }
                | DiffOp::RowReplaced { .. }
                | DiffOp::CellEdited { .. }
        )
    });
    assert!(has_grid_op, "expected at least one grid op in composed diff");

    let mut saw_definition_change = false;
    let mut saw_metadata_change = false;
    let mut saw_params_change = false;

    for op in report.m_ops() {
        match op {
            DiffOp::QueryDefinitionChanged {
                name,
                semantic_detail: Some(detail),
                ..
            } => {
                if report.resolve(*name) == Some("Section1/SalesWithRegions") {
                    saw_definition_change = true;
                    if has_params_changed(detail) {
                        saw_params_change = true;
                    }
                }
            }
            DiffOp::QueryMetadataChanged { name, .. } => {
                if report.resolve(*name) == Some("Section1/SalesWithRegions") {
                    saw_metadata_change = true;
                }
            }
            _ => {}
        }
    }

    assert!(
        saw_definition_change,
        "expected QueryDefinitionChanged for Section1/SalesWithRegions"
    );
    assert!(
        saw_params_change,
        "expected ParamsChanged in semantic detail for Section1/SalesWithRegions"
    );
    assert!(
        saw_metadata_change,
        "expected QueryMetadataChanged for Section1/SalesWithRegions"
    );
}

#[test]
fn adversarial_steps_report_param_changes() {
    let pkg_a = load_package("m_adversarial_steps_a.xlsx");
    let pkg_b = load_package("m_adversarial_steps_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());

    let mut saw_params_change = false;
    for op in report.m_ops() {
        if let DiffOp::QueryDefinitionChanged {
            name,
            semantic_detail: Some(detail),
            ..
        } = op
        {
            if report.resolve(*name) == Some("Section1/Adversarial") {
                if has_params_changed(detail) {
                    saw_params_change = true;
                }
            }
        }
    }

    assert!(
        saw_params_change,
        "expected ParamsChanged for Section1/Adversarial diff"
    );
}
