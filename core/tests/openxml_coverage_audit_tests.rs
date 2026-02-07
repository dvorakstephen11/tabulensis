mod common;

use common::open_fixture_pkg;
use excel_diff::{build_embedded_queries, build_queries, with_default_session};

fn resolve(id: excel_diff::StringId) -> String {
    with_default_session(|session| session.strings.resolve(id).to_string())
}

fn resolve_opt(id: Option<excel_diff::StringId>) -> Option<String> {
    id.map(resolve)
}

#[test]
fn coverage_audit_named_ranges_are_structured() {
    let pkg = open_fixture_pkg("named_ranges_b.xlsx");
    let ranges = &pkg.workbook.named_ranges;
    assert!(
        !ranges.is_empty(),
        "expected named ranges to parse into Workbook.named_ranges"
    );

    let mut saw_global_add = false;
    let mut saw_local_change = false;

    for range in ranges {
        let name = resolve(range.name);
        let refers_to = resolve(range.refers_to);
        let scope = resolve_opt(range.scope);

        if name == "GlobalAdd" {
            assert_eq!(scope, None);
            assert!(
                !refers_to.trim().is_empty(),
                "expected GlobalAdd to have a non-empty refers_to"
            );
            saw_global_add = true;
        }

        if name == "Sheet1!LocalChange" {
            assert_eq!(scope.as_deref(), Some("Sheet1"));
            assert_eq!(refers_to, "Sheet1!$C$2");
            saw_local_change = true;
        }
    }

    assert!(saw_global_add, "expected GlobalAdd named range in fixture");
    assert!(
        saw_local_change,
        "expected Sheet1!LocalChange named range in fixture"
    );
}

#[test]
fn coverage_audit_charts_have_metadata() {
    let pkg = open_fixture_pkg("charts_b.xlsx");
    let charts = &pkg.workbook.charts;
    assert!(
        !charts.is_empty(),
        "expected charts to parse into Workbook.charts"
    );

    let mut saw_chart1 = false;
    let mut saw_chart2 = false;

    for chart in charts {
        let sheet = resolve(chart.sheet);
        let name = resolve(chart.info.name);
        let chart_type = resolve(chart.info.chart_type);
        let data_range = chart.info.data_range.map(resolve);

        if name == "Chart 1" || name == "Chart 2" {
            assert_eq!(sheet, "Sheet1");
            assert_ne!(chart_type, "unknown");
            assert!(
                chart_type.ends_with("Chart"),
                "expected chart_type like '*Chart', got {chart_type:?}"
            );
            assert!(
                data_range.as_ref().is_some_and(|r| !r.trim().is_empty()),
                "expected chart data range for {name}"
            );
        }

        if name == "Chart 1" {
            saw_chart1 = true;
        }
        if name == "Chart 2" {
            saw_chart2 = true;
        }
    }

    assert!(saw_chart1, "expected Chart 1 in fixture");
    assert!(saw_chart2, "expected Chart 2 in fixture");
}

#[test]
fn coverage_audit_optional_parts_datamashup_and_vba_are_detected() {
    let pkg = open_fixture_pkg("m_embedded_change_a.xlsx");
    let dm = pkg
        .data_mashup
        .as_ref()
        .expect("expected DataMashup part to be detected and parsed");

    let embedded = build_embedded_queries(dm);
    assert!(
        embedded
            .iter()
            .any(|q| q.name == "Embedded/Content/efgh.package/Section1/Inner"),
        "expected embedded query to be extracted from DataMashup"
    );

    let main = build_queries(dm).expect("main queries should build");
    assert!(
        main.iter().all(|q| !q.name.starts_with("Embedded/")),
        "build_queries() should not include embedded query prefix"
    );

    let pkg = open_fixture_pkg("vba_base.xlsm");
    let modules = pkg
        .vba_modules
        .as_ref()
        .expect("expected VBA modules to be extracted for .xlsm");

    let module_names: Vec<String> = modules.iter().map(|m| resolve(m.name)).collect();
    assert!(
        module_names.iter().any(|name| name == "Module1"),
        "expected Module1; got {module_names:?}"
    );
}
