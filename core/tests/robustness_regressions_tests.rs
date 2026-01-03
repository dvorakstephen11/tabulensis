mod common;

use common::fixture_path;
use excel_diff::{
    ContainerLimits, DiffConfig, DiffSession, LimitBehavior, PbixPackage, WorkbookPackage,
    build_data_mashup, parse_data_mashup, with_default_session,
};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Suite {
    fixtures: Vec<FixtureSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureSpec {
    file: String,
    #[serde(rename = "type")]
    kind: FixtureKind,
    expectation: Expectation,
    #[serde(default)]
    invariants: Invariants,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FixtureKind {
    Xlsx,
    Xlsm,
    Pbix,
    Pbit,
    DmBytes,
}

#[derive(Debug, Clone, Deserialize)]
struct Expectation {
    result: ExpectationResult,
    error_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ExpectationResult {
    Ok,
    Error,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Invariants {
    no_panic: Option<bool>,
    self_diff_empty: Option<bool>,
    deterministic_open: Option<bool>,
}

#[test]
fn robustness_regressions() {
    let suite = load_suite();
    for fixture in suite.fixtures {
        run_fixture(&fixture);
    }
}

fn load_suite() -> Suite {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("robustness_regressions.yaml");
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_yaml::from_str(&contents)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

fn run_fixture(fixture: &FixtureSpec) {
    let expected_ok = fixture.expectation.result == ExpectationResult::Ok;
    let invariants = normalized_invariants(&fixture.invariants, expected_ok);
    let path = fixture_path(&fixture.file);

    match fixture.kind {
        FixtureKind::Xlsx | FixtureKind::Xlsm => {
            let limits = container_limits();
            let result = open_workbook(&path, limits);
            assert_expectation(result, fixture);
            if expected_ok {
                run_workbook_invariants(&path, limits, &invariants);
            }
        }
        FixtureKind::Pbix | FixtureKind::Pbit => {
            let limits = container_limits();
            let result = open_pbix(&path, limits);
            assert_expectation(result, fixture);
            if expected_ok {
                run_pbix_invariants(&path, limits, &invariants);
            }
        }
        FixtureKind::DmBytes => {
            let bytes = fs::read(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            let result = parse_dm_bytes(&bytes);
            assert_expectation(result, fixture);
            if expected_ok {
                run_dm_invariants(&bytes, &invariants);
            }
        }
    }
}

fn normalized_invariants(invariants: &Invariants, expected_ok: bool) -> Invariants {
    let mut out = invariants.clone();
    if out.no_panic.is_none() {
        out.no_panic = Some(true);
    }
    if expected_ok {
        if out.self_diff_empty.is_none() {
            out.self_diff_empty = Some(true);
        }
        if out.deterministic_open.is_none() {
            out.deterministic_open = Some(true);
        }
    }
    out
}

fn container_limits() -> ContainerLimits {
    ContainerLimits {
        max_entries: 2000,
        max_part_uncompressed_bytes: 10 * 1024 * 1024,
        max_total_uncompressed_bytes: 50 * 1024 * 1024,
    }
}

fn robust_config() -> DiffConfig {
    let mut config = DiffConfig::default();
    config.hardening.on_limit_exceeded = LimitBehavior::ReturnError;
    config.hardening.max_memory_mb = Some(256);
    config.hardening.timeout_seconds = Some(10);
    config.hardening.max_ops = Some(200_000);
    config
}

fn reset_session() {
    with_default_session(|session| *session = DiffSession::new());
}

fn open_workbook(
    path: &Path,
    limits: ContainerLimits,
) -> Result<WorkbookPackage, excel_diff::PackageError> {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()));
    WorkbookPackage::open_with_limits(file, limits)
}

fn open_pbix(
    path: &Path,
    limits: ContainerLimits,
) -> Result<PbixPackage, excel_diff::PackageError> {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()));
    PbixPackage::open_with_limits(file, limits)
}

fn parse_dm_bytes(bytes: &[u8]) -> Result<excel_diff::DataMashup, excel_diff::DataMashupError> {
    let raw = parse_data_mashup(bytes)?;
    build_data_mashup(&raw)
}

fn assert_expectation<T, E>(result: Result<T, E>, fixture: &FixtureSpec)
where
    E: std::fmt::Debug + ErrorCode,
{
    match (fixture.expectation.result.clone(), result) {
        (ExpectationResult::Ok, Ok(_)) => {}
        (ExpectationResult::Ok, Err(err)) => {
            panic!("{} expected ok, got error {err:?}", fixture.file)
        }
        (ExpectationResult::Error, Ok(_)) => {
            panic!("{} expected error, got ok", fixture.file)
        }
        (ExpectationResult::Error, Err(err)) => {
            let expected = fixture
                .expectation
                .error_code
                .as_ref()
                .unwrap_or_else(|| {
                    panic!("{} missing error_code for error expectation", fixture.file)
                });
            assert_eq!(
                expected,
                err.code(),
                "{} error code mismatch",
                fixture.file
            );
        }
    }
}

fn run_workbook_invariants(path: &Path, limits: ContainerLimits, invariants: &Invariants) {
    let config = robust_config();
    if invariants.self_diff_empty == Some(true) {
        assert_workbook_self_diff_empty(path, limits, &config);
    }
    if invariants.deterministic_open == Some(true) {
        assert_workbook_self_diff_empty(path, limits, &config);
    }
}

fn run_pbix_invariants(path: &Path, limits: ContainerLimits, invariants: &Invariants) {
    let config = robust_config();
    if invariants.self_diff_empty == Some(true) {
        assert_pbix_self_diff_empty(path, limits, &config);
    }
    if invariants.deterministic_open == Some(true) {
        assert_pbix_self_diff_empty(path, limits, &config);
    }
}

fn run_dm_invariants(bytes: &[u8], invariants: &Invariants) {
    if invariants.self_diff_empty == Some(true) || invariants.deterministic_open == Some(true) {
        let dm_a = parse_dm_bytes(bytes).expect("DataMashup bytes should parse");
        let dm_b = parse_dm_bytes(bytes).expect("DataMashup bytes should parse again");
        assert_eq!(dm_a, dm_b, "DataMashup parsing should be deterministic");
    }
}

fn assert_workbook_self_diff_empty(
    path: &Path,
    limits: ContainerLimits,
    config: &DiffConfig,
) {
    reset_session();
    let pkg_a = open_workbook(path, limits).expect("fixture should parse");
    let pkg_b = open_workbook(path, limits).expect("fixture should parse again");
    let report = pkg_a.diff(&pkg_b, config);
    assert!(
        report.ops.is_empty(),
        "self-diff should be empty for {}",
        path.display()
    );
    reset_session();
}

fn assert_pbix_self_diff_empty(path: &Path, limits: ContainerLimits, config: &DiffConfig) {
    reset_session();
    let pkg_a = open_pbix(path, limits).expect("fixture should parse");
    let pkg_b = open_pbix(path, limits).expect("fixture should parse again");
    let report = pkg_a.diff(&pkg_b, config);
    assert!(
        report.ops.is_empty(),
        "self-diff should be empty for {}",
        path.display()
    );
    reset_session();
}

trait ErrorCode {
    fn code(&self) -> &'static str;
}

impl ErrorCode for excel_diff::PackageError {
    fn code(&self) -> &'static str {
        excel_diff::PackageError::code(self)
    }
}

impl ErrorCode for excel_diff::DataMashupError {
    fn code(&self) -> &'static str {
        excel_diff::DataMashupError::code(self)
    }
}
