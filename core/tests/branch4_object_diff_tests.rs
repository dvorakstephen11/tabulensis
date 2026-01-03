mod common;

use common::open_fixture_pkg;
use excel_diff::{DiffConfig, DiffOp, DiffReport, StringId, with_default_session};

fn resolve<'a>(report: &'a DiffReport, id: StringId) -> &'a str {
    report.strings[id.0 as usize].as_str()
}

#[test]
fn branch4_named_ranges_emit_add_remove_change() {
    let pkg_a = open_fixture_pkg("named_ranges_a.xlsx");
    let pkg_b = open_fixture_pkg("named_ranges_b.xlsx");

    let report = pkg_a.diff(&pkg_b, &DiffConfig::default());

    let mut saw_added = false;
    let mut saw_removed = false;
    let mut saw_changed = false;

    for op in &report.ops {
        match op {
            DiffOp::NamedRangeAdded { name } => {
                assert_eq!(resolve(&report, *name), "GlobalAdd");
                saw_added = true;
            }
            DiffOp::NamedRangeRemoved { name } => {
                assert_eq!(resolve(&report, *name), "GlobalRemove");
                saw_removed = true;
            }
            DiffOp::NamedRangeChanged {
                name,
                old_ref,
                new_ref,
            } => {
                assert_eq!(resolve(&report, *name), "Sheet1!LocalChange");
                assert_eq!(resolve(&report, *old_ref), "Sheet1!$C$1");
                assert_eq!(resolve(&report, *new_ref), "Sheet1!$C$2");
                saw_changed = true;
            }
            _ => {}
        }
    }

    assert!(saw_added, "expected NamedRangeAdded(GlobalAdd)");
    assert!(saw_removed, "expected NamedRangeRemoved(GlobalRemove)");
    assert!(saw_changed, "expected NamedRangeChanged(Sheet1!LocalChange)");
}

#[test]
fn branch4_charts_emit_added_removed_changed() {
    let pkg_a = open_fixture_pkg("charts_a.xlsx");
    let pkg_b = open_fixture_pkg("charts_b.xlsx");

    let report_ab = pkg_a.diff(&pkg_b, &DiffConfig::default());

    let mut saw_changed_chart1 = false;
    let mut saw_added_chart2 = false;

    for op in &report_ab.ops {
        match op {
            DiffOp::ChartChanged { sheet, name } => {
                assert_eq!(resolve(&report_ab, *sheet), "Sheet1");
                assert_eq!(resolve(&report_ab, *name), "Chart 1");
                saw_changed_chart1 = true;
            }
            DiffOp::ChartAdded { sheet, name } => {
                assert_eq!(resolve(&report_ab, *sheet), "Sheet1");
                assert_eq!(resolve(&report_ab, *name), "Chart 2");
                saw_added_chart2 = true;
            }
            _ => {}
        }
    }

    assert!(saw_changed_chart1, "expected ChartChanged(Sheet1, Chart 1)");
    assert!(saw_added_chart2, "expected ChartAdded(Sheet1, Chart 2)");

    let report_ba = pkg_b.diff(&pkg_a, &DiffConfig::default());
    let mut saw_removed_chart2 = false;
    for op in &report_ba.ops {
        if let DiffOp::ChartRemoved { sheet, name } = op {
            assert_eq!(resolve(&report_ba, *sheet), "Sheet1");
            assert_eq!(resolve(&report_ba, *name), "Chart 2");
            saw_removed_chart2 = true;
        }
    }
    assert!(saw_removed_chart2, "expected ChartRemoved(Sheet1, Chart 2)");
}

#[test]
fn branch4_vba_modules_emit_added_removed_changed() {
    let pkg_base = open_fixture_pkg("vba_base.xlsm");
    let pkg_added = open_fixture_pkg("vba_added.xlsm");
    let pkg_changed = open_fixture_pkg("vba_changed.xlsm");

    let report_added = pkg_base.diff(&pkg_added, &DiffConfig::default());
    let mut saw_module2_added = false;
    for op in &report_added.ops {
        if let DiffOp::VbaModuleAdded { name } = op {
            if resolve(&report_added, *name) == "Module2" {
                saw_module2_added = true;
            }
        }
    }
    assert!(saw_module2_added, "expected VbaModuleAdded(Module2)");

    let report_removed = pkg_added.diff(&pkg_base, &DiffConfig::default());
    let mut saw_module2_removed = false;
    for op in &report_removed.ops {
        if let DiffOp::VbaModuleRemoved { name } = op {
            if resolve(&report_removed, *name) == "Module2" {
                saw_module2_removed = true;
            }
        }
    }
    assert!(saw_module2_removed, "expected VbaModuleRemoved(Module2)");

    let report_changed = pkg_base.diff(&pkg_changed, &DiffConfig::default());
    let mut saw_module1_changed = false;
    for op in &report_changed.ops {
        if let DiffOp::VbaModuleChanged { name } = op {
            if resolve(&report_changed, *name) == "Module1" {
                saw_module1_changed = true;
            }
        }
    }
    assert!(saw_module1_changed, "expected VbaModuleChanged(Module1)");
}

#[test]
fn branch4_vba_modules_open_returns_modules() {
    let pkg_base = open_fixture_pkg("vba_base.xlsm");
    let pkg_added = open_fixture_pkg("vba_added.xlsm");

    let base_modules = pkg_base
        .vba_modules
        .as_ref()
        .expect("expected VBA modules in base fixture");
    let base_names: Vec<String> = with_default_session(|session| {
        base_modules
            .iter()
            .map(|module| session.strings.resolve(module.name).to_string())
            .collect()
    });
    assert!(
        base_names.iter().any(|name| name == "Module1"),
        "expected Module1 in base fixture"
    );

    let added_modules = pkg_added
        .vba_modules
        .as_ref()
        .expect("expected VBA modules in added fixture");
    let added_names: Vec<String> = with_default_session(|session| {
        added_modules
            .iter()
            .map(|module| session.strings.resolve(module.name).to_string())
            .collect()
    });
    assert!(
        added_names.iter().any(|name| name == "Module2"),
        "expected Module2 in added fixture"
    );
}
