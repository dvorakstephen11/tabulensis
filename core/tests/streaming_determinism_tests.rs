mod common;

use common::{
    StructuredOutput, assert_jsonl_determinism_with_fresh_sessions,
    assert_structured_determinism_with_fresh_sessions, fixture_path,
};
use excel_diff::{
    CellValue, DataMashup, DiffConfig, DiffSession, Grid, JsonLinesSink, Metadata, PackageParts,
    PackageXml, PbixPackage, Permissions, SectionDocument, Sheet, SheetKind, StringPool, VbaModule,
    VbaModuleType, Workbook, WorkbookPackage, VecSink, PermissionBindingsStatus,
    try_diff_grids_database_mode_streaming,
};
use std::fs::File;

fn make_workbook(pool: &mut StringPool, value: f64) -> Workbook {
    let mut grid = Grid::new(1, 1);
    grid.insert_cell(0, 0, Some(CellValue::Number(value)), None);

    Workbook {
        sheets: vec![Sheet {
            name: pool.intern("Sheet1"),
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    }
}

fn make_keyed_grid(keys: &[i32], values: &[i32]) -> Grid {
    let rows = keys.len().max(values.len());
    let mut grid = Grid::new(rows as u32, 2);
    for row in 0..rows {
        let key = keys.get(row).copied().unwrap_or_default() as f64;
        let value = values.get(row).copied().unwrap_or_default() as f64;
        grid.insert_cell(row as u32, 0, Some(CellValue::Number(key)), None);
        grid.insert_cell(row as u32, 1, Some(CellValue::Number(value)), None);
    }
    grid
}

fn make_dm(section_source: &str) -> DataMashup {
    DataMashup::new(
        0,
        PackageParts {
            package_xml: PackageXml {
                raw_xml: "<Package/>".to_string(),
            },
            main_section: SectionDocument {
                source: section_source.to_string(),
            },
            embedded_contents: Vec::new(),
        },
        Permissions::default(),
        Metadata { formulas: Vec::new() },
        Vec::new(),
        PermissionBindingsStatus::Missing,
    )
}

fn build_packages(pool: &mut StringPool) -> (WorkbookPackage, WorkbookPackage) {
    let wb_a = make_workbook(pool, 1.0);
    let wb_b = make_workbook(pool, 2.0);

    let dm_a = make_dm("section Section1;\nshared Foo = 1;");
    let dm_b = make_dm("section Section1;\nshared Bar = 1;");

    let vba_name = pool.intern("Module1");
    let vba_modules = Some(vec![VbaModule {
        name: vba_name,
        module_type: VbaModuleType::Standard,
        code: "Sub Foo()\nEnd Sub".to_string(),
    }]);

    let pkg_a = WorkbookPackage {
        workbook: wb_a,
        data_mashup: Some(dm_a),
        vba_modules: None,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };
    let pkg_b = WorkbookPackage {
        workbook: wb_b,
        data_mashup: Some(dm_b),
        vba_modules,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms: 0,
    };

    (pkg_a, pkg_b)
}

#[test]
fn workbook_package_streaming_is_deterministic_with_fresh_sessions() {
    assert_structured_determinism_with_fresh_sessions(2, |session| {
        let (pkg_a, pkg_b) = build_packages(&mut session.strings);
        let mut sink = VecSink::new();
        let summary = pkg_a
            .diff_streaming_with_pool(&pkg_b, &mut session.strings, &DiffConfig::default(), &mut sink)
            .expect("streaming diff should succeed");
        StructuredOutput {
            ops: sink.into_ops(),
            summary,
        }
    });
}

#[test]
fn database_mode_streaming_is_deterministic_with_fresh_sessions() {
    assert_structured_determinism_with_fresh_sessions(2, |session| {
        let grid_a = make_keyed_grid(&[1, 2], &[10, 20]);
        let grid_b = make_keyed_grid(&[1, 2], &[10, 25]);
        let sheet_id = session.strings.intern("Data");

        let mut sink = VecSink::new();
        let mut op_count = 0usize;
        let summary = try_diff_grids_database_mode_streaming(
            sheet_id,
            &grid_a,
            &grid_b,
            &[0],
            &mut session.strings,
            &DiffConfig::default(),
            &mut sink,
            &mut op_count,
        )
        .expect("database streaming diff should succeed");

        StructuredOutput {
            ops: sink.into_ops(),
            summary,
        }
    });
}

#[test]
fn pbix_streaming_jsonl_is_deterministic_with_fresh_sessions() {
    let path_a = fixture_path("pbix_legacy_multi_query_a.pbix");
    let path_b = fixture_path("pbix_legacy_multi_query_b.pbix");

    assert_jsonl_determinism_with_fresh_sessions(2, |_session| {
        excel_diff::with_default_session(|session| *session = DiffSession::new());

        let pkg_a = PbixPackage::open(File::open(&path_a).expect("fixture should exist"))
            .expect("pbix A should parse");
        let pkg_b = PbixPackage::open(File::open(&path_b).expect("fixture should exist"))
            .expect("pbix B should parse");

        let mut out = Vec::<u8>::new();
        let mut sink = JsonLinesSink::new(&mut out);
        pkg_a
            .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
            .expect("pbix streaming should succeed");
        out
    });
}

#[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
#[test]
fn pbit_streaming_jsonl_is_deterministic_with_fresh_sessions() {
    let path_a = fixture_path("pbit_model_a.pbit");
    let path_b = fixture_path("pbit_model_b.pbit");

    assert_jsonl_determinism_with_fresh_sessions(2, |_session| {
        excel_diff::with_default_session(|session| *session = DiffSession::new());

        let pkg_a = PbixPackage::open(File::open(&path_a).expect("fixture should exist"))
            .expect("pbit A should parse");
        let pkg_b = PbixPackage::open(File::open(&path_b).expect("fixture should exist"))
            .expect("pbit B should parse");

        let mut out = Vec::<u8>::new();
        let mut sink = JsonLinesSink::new(&mut out);
        pkg_a
            .diff_streaming(&pkg_b, &DiffConfig::default(), &mut sink)
            .expect("pbit streaming should succeed");
        out
    });
}
