use crate::config::DiffConfig;
use crate::container::ZipContainer;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary, SheetId};
use crate::diffable::{DiffContext, Diffable};
use crate::permission_bindings::{PermissionBindingsStatus, permission_bindings_warning};
use crate::progress::ProgressCallback;
use crate::sink::{DiffSink, NoFinishSink, SinkFinishGuard, VecSink};
use crate::string_pool::StringPool;
use crate::vba::VbaModule;
use crate::workbook::{Sheet, Workbook};
#[cfg(feature = "perf-metrics")]
use crate::perf::DiffMetrics;
#[cfg(feature = "excel-open-xml")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "excel-open-xml")]
use std::time::Instant;
#[cfg(feature = "excel-open-xml")]
use thiserror::Error;

/// A parsed workbook plus optional associated content (Power Query and VBA).
///
/// This is the recommended high-level entry point for most callers. It wraps the workbook IR
/// (`Workbook`, `Sheet`, `Grid`) together with:
/// - optional DataMashup content (Power Query / M)
/// - optional extracted VBA modules (for `.xlsm`)
///
/// Diffs produced via [`WorkbookPackage::diff`] and related APIs include grid ops, object ops
/// (named ranges, charts, VBA), and M ops when present.
#[derive(Debug, Clone)]
pub struct WorkbookPackage {
    /// Parsed workbook IR (sheets, grids, named ranges, charts).
    pub workbook: Workbook,
    /// Parsed DataMashup content (Power Query), if present.
    pub data_mashup: Option<DataMashup>,
    /// Extracted VBA modules, if present and the `vba` feature is enabled.
    pub vba_modules: Option<Vec<VbaModule>>,
    #[cfg(feature = "perf-metrics")]
    /// Parse time for this package (ms), captured when opening from bytes.
    pub parse_time_ms: u64,
}

impl From<Workbook> for WorkbookPackage {
    fn from(workbook: Workbook) -> Self {
        Self {
            workbook,
            data_mashup: None,
            vba_modules: None,
            #[cfg(feature = "perf-metrics")]
            parse_time_ms: 0,
        }
    }
}

#[cfg(feature = "excel-open-xml")]
fn open_profile_enabled() -> bool {
    match std::env::var("EXCEL_DIFF_PROFILE_OPEN") {
        Ok(value) => value == "1" || value.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

#[cfg(feature = "excel-open-xml")]
/// Errors that can occur when diffing two Open XML workbooks without fully materializing both
/// `WorkbookPackage`s up-front (e.g. when using part-level skip/fingerprints).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OpenXmlDiffError {
    #[error("{0}")]
    Package(#[from] crate::excel_open_xml::PackageError),
    #[error("{0}")]
    Diff(#[from] DiffError),
}

#[cfg(feature = "excel-open-xml")]
impl OpenXmlDiffError {
    pub fn code(&self) -> &'static str {
        match self {
            OpenXmlDiffError::Package(err) => err.code(),
            OpenXmlDiffError::Diff(err) => err.code(),
        }
    }
}

#[cfg(feature = "excel-open-xml")]
#[derive(Debug, Clone)]
struct SheetTargetMeta {
    sheet_id: Option<u32>,
    name_lower: String,
    target: String,
}

#[cfg(feature = "excel-open-xml")]
fn read_workbook_xml_checked(
    container: &mut crate::container::OpcContainer,
) -> Result<Vec<u8>, crate::excel_open_xml::PackageError> {
    container
        .read_file_checked("xl/workbook.xml")
        .map_err(|e| match e {
            crate::ContainerError::FileNotFound { .. } => {
                if container.file_names().any(|name| name == "xl/workbook.bin") {
                    crate::excel_open_xml::PackageError::UnsupportedFormat {
                        message:
                            "XLSB detected (xl/workbook.bin present); convert to .xlsx/.xlsm"
                                .to_string(),
                    }
                } else {
                    crate::excel_open_xml::PackageError::MissingPart {
                        path: "xl/workbook.xml".to_string(),
                    }
                }
            }
            other => crate::excel_open_xml::PackageError::ReadPartFailed {
                part: "xl/workbook.xml".to_string(),
                message: other.to_string(),
            },
        })
}

#[cfg(feature = "excel-open-xml")]
fn workbook_sheet_targets(
    container: &mut crate::container::OpcContainer,
) -> Result<Vec<SheetTargetMeta>, crate::excel_open_xml::PackageError> {
    let workbook_bytes = read_workbook_xml_checked(container)?;
    let sheets = crate::grid_parser::parse_workbook_xml(&workbook_bytes)
        .map_err(|e| crate::excel_open_xml::wrap_grid_parse_error(e, "xl/workbook.xml"))?;

    let workbook_rels_bytes = container.read_file_optional_checked("xl/_rels/workbook.xml.rels")?;
    let relationships = match workbook_rels_bytes {
        Some(bytes) => crate::grid_parser::parse_relationships(&bytes).map_err(|e| {
            crate::excel_open_xml::wrap_grid_parse_error(e, "xl/_rels/workbook.xml.rels")
        })?,
        None => HashMap::new(),
    };

    let mut metas = Vec::with_capacity(sheets.len());
    for (idx, sheet) in sheets.iter().enumerate() {
        let target = crate::grid_parser::resolve_sheet_target(sheet, &relationships, idx);
        metas.push(SheetTargetMeta {
            sheet_id: sheet.sheet_id,
            name_lower: sheet.name.to_lowercase(),
            target,
        });
    }
    Ok(metas)
}

#[cfg(feature = "excel-open-xml")]
fn duplicate_sheet_ids(metas: &[SheetTargetMeta]) -> HashSet<u32> {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for meta in metas {
        let Some(id) = meta.sheet_id else { continue };
        *counts.entry(id).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(id, count)| if count > 1 { Some(id) } else { None })
        .collect()
}

#[cfg(feature = "excel-open-xml")]
fn duplicate_sheet_names(metas: &[SheetTargetMeta]) -> HashSet<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for meta in metas {
        *counts.entry(meta.name_lower.as_str()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(name, count)| if count > 1 { Some(name.to_string()) } else { None })
        .collect()
}

#[cfg(feature = "excel-open-xml")]
fn compute_sheet_grid_parse_targets(
    old_container: &mut crate::container::OpcContainer,
    new_container: &mut crate::container::OpcContainer,
) -> Result<(HashSet<String>, HashSet<String>), crate::excel_open_xml::PackageError> {
    let old_metas = workbook_sheet_targets(old_container)?;
    let new_metas = workbook_sheet_targets(new_container)?;

    let mut old_parse: HashSet<String> = HashSet::new();
    let mut new_parse: HashSet<String> = HashSet::new();

    let mut ambiguous_ids = duplicate_sheet_ids(&old_metas);
    ambiguous_ids.extend(duplicate_sheet_ids(&new_metas));

    let mut ambiguous_names = duplicate_sheet_names(&old_metas);
    ambiguous_names.extend(duplicate_sheet_names(&new_metas));

    for meta in &old_metas {
        if meta
            .sheet_id
            .is_some_and(|id| ambiguous_ids.contains(&id))
            || ambiguous_names.contains(&meta.name_lower)
        {
            old_parse.insert(meta.target.clone());
        }
    }
    for meta in &new_metas {
        if meta
            .sheet_id
            .is_some_and(|id| ambiguous_ids.contains(&id))
            || ambiguous_names.contains(&meta.name_lower)
        {
            new_parse.insert(meta.target.clone());
        }
    }

    let shared_old = old_container.file_fingerprint_optional_checked("xl/sharedStrings.xml")?;
    let shared_new = new_container.file_fingerprint_optional_checked("xl/sharedStrings.xml")?;
    let shared_same = shared_old == shared_new;

    let mut old_consumed = vec![false; old_metas.len()];
    let mut new_consumed = vec![false; new_metas.len()];

    let mut old_by_id: HashMap<u32, usize> = HashMap::new();
    for (idx, meta) in old_metas.iter().enumerate() {
        let Some(id) = meta.sheet_id else { continue };
        if ambiguous_ids.contains(&id) {
            continue;
        }
        old_by_id.insert(id, idx);
    }
    let mut new_by_id: HashMap<u32, usize> = HashMap::new();
    for (idx, meta) in new_metas.iter().enumerate() {
        let Some(id) = meta.sheet_id else { continue };
        if ambiguous_ids.contains(&id) {
            continue;
        }
        new_by_id.insert(id, idx);
    }

    for (id, &old_idx) in &old_by_id {
        let Some(&new_idx) = new_by_id.get(id) else {
            continue;
        };

        old_consumed[old_idx] = true;
        new_consumed[new_idx] = true;

        let old_target = &old_metas[old_idx].target;
        let new_target = &new_metas[new_idx].target;

        if !shared_same {
            old_parse.insert(old_target.clone());
            new_parse.insert(new_target.clone());
            continue;
        }

        if old_parse.contains(old_target) || new_parse.contains(new_target) {
            continue;
        }

        let old_fp = old_container.file_fingerprint_checked(old_target)?;
        let new_fp = new_container.file_fingerprint_checked(new_target)?;
        if old_fp != new_fp {
            old_parse.insert(old_target.clone());
            new_parse.insert(new_target.clone());
        }
    }

    let mut old_by_name: HashMap<&str, usize> = HashMap::new();
    for (idx, meta) in old_metas.iter().enumerate() {
        if old_consumed[idx] {
            continue;
        }
        if ambiguous_names.contains(&meta.name_lower) {
            continue;
        }
        old_by_name.insert(meta.name_lower.as_str(), idx);
    }

    let mut new_by_name: HashMap<&str, usize> = HashMap::new();
    for (idx, meta) in new_metas.iter().enumerate() {
        if new_consumed[idx] {
            continue;
        }
        if ambiguous_names.contains(&meta.name_lower) {
            continue;
        }
        new_by_name.insert(meta.name_lower.as_str(), idx);
    }

    for (name, &old_idx) in &old_by_name {
        let Some(&new_idx) = new_by_name.get(name) else {
            continue;
        };

        let old_target = &old_metas[old_idx].target;
        let new_target = &new_metas[new_idx].target;

        if !shared_same {
            old_parse.insert(old_target.clone());
            new_parse.insert(new_target.clone());
            continue;
        }

        if old_parse.contains(old_target) || new_parse.contains(new_target) {
            continue;
        }

        let old_fp = old_container.file_fingerprint_checked(old_target)?;
        let new_fp = new_container.file_fingerprint_checked(new_target)?;
        if old_fp != new_fp {
            old_parse.insert(old_target.clone());
            new_parse.insert(new_target.clone());
        }
    }

    Ok((old_parse, new_parse))
}

#[cfg(feature = "excel-open-xml")]
fn open_workbook_package_from_container_with_grid_filter(
    container: &mut crate::container::OpcContainer,
    pool: &mut StringPool,
    grid_targets_to_parse: &HashSet<String>,
) -> Result<WorkbookPackage, crate::excel_open_xml::PackageError> {
    #[cfg(feature = "perf-metrics")]
    let total_start = Instant::now();
    let workbook = crate::excel_open_xml::open_workbook_from_container_with_grid_filter(
        container,
        pool,
        Some(grid_targets_to_parse),
    )?;

    let raw = crate::excel_open_xml::open_data_mashup_from_container(container)?;
    let data_mashup = match raw {
        Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
        None => None,
    };

    let vba_modules = crate::excel_open_xml::open_vba_modules_from_container(container, pool)?;

    #[cfg(feature = "perf-metrics")]
    let parse_time_ms = total_start.elapsed().as_millis() as u64;

    Ok(WorkbookPackage {
        workbook,
        data_mashup,
        vba_modules,
        #[cfg(feature = "perf-metrics")]
        parse_time_ms,
    })
}

impl WorkbookPackage {
    #[cfg(feature = "excel-open-xml")]
    /// Parse a workbook from any `Read + Seek` source.
    ///
    /// This is available when the `excel-open-xml` feature is enabled (enabled by default).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use excel_diff::WorkbookPackage;
    /// use std::fs::File;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let _pkg = WorkbookPackage::open(File::open("workbook.xlsx")?)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
    ) -> Result<Self, crate::excel_open_xml::PackageError> {
        crate::with_default_session(|session| {
            let profile_enabled = open_profile_enabled();
            let total_start = Instant::now();
            let mut container = crate::container::OpcContainer::open_from_reader(reader)?;

            let workbook_start = Instant::now();
            let workbook =
                crate::excel_open_xml::open_workbook_from_container(&mut container, &mut session.strings)?;
            let workbook_ms = workbook_start.elapsed().as_millis() as u64;

            let data_mashup_start = Instant::now();
            let raw = crate::excel_open_xml::open_data_mashup_from_container(&mut container)?;
            let data_mashup = match raw {
                Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
                None => None,
            };
            let data_mashup_ms = data_mashup_start.elapsed().as_millis() as u64;

            let vba_start = Instant::now();
            let vba_modules = crate::excel_open_xml::open_vba_modules_from_container(
                &mut container,
                &mut session.strings,
            )?;
            let vba_ms = vba_start.elapsed().as_millis() as u64;

            #[cfg(feature = "perf-metrics")]
            let parse_time_ms = total_start.elapsed().as_millis() as u64;
            #[cfg(not(feature = "perf-metrics"))]
            let parse_time_ms = 0_u64;

            if profile_enabled {
                let vba_modules_count = vba_modules.as_ref().map(|items| items.len()).unwrap_or(0);
                println!(
                    "PERF_OPEN_PACKAGE total_ms={} workbook_ms={} data_mashup_ms={} vba_ms={} parse_time_ms={} sheets={} has_data_mashup={} vba_modules={}",
                    total_start.elapsed().as_millis() as u64,
                    workbook_ms,
                    data_mashup_ms,
                    vba_ms,
                    parse_time_ms,
                    workbook.sheets.len(),
                    data_mashup.is_some(),
                    vba_modules_count
                );
            }

            Ok(Self {
                workbook,
                data_mashup,
                vba_modules,
                #[cfg(feature = "perf-metrics")]
                parse_time_ms,
            })
        })
    }

    #[cfg(feature = "excel-open-xml")]
    /// Parse a workbook with custom container limits (trusted overrides).
    pub fn open_with_limits<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
        limits: crate::ContainerLimits,
    ) -> Result<Self, crate::excel_open_xml::PackageError> {
        crate::with_default_session(|session| {
            let profile_enabled = open_profile_enabled();
            let total_start = Instant::now();
            let mut container =
                crate::container::OpcContainer::open_from_reader_with_limits(reader, limits)?;

            let workbook_start = Instant::now();
            let workbook =
                crate::excel_open_xml::open_workbook_from_container(&mut container, &mut session.strings)?;
            let workbook_ms = workbook_start.elapsed().as_millis() as u64;

            let data_mashup_start = Instant::now();
            let raw = crate::excel_open_xml::open_data_mashup_from_container(&mut container)?;
            let data_mashup = match raw {
                Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
                None => None,
            };
            let data_mashup_ms = data_mashup_start.elapsed().as_millis() as u64;

            let vba_start = Instant::now();
            let vba_modules = crate::excel_open_xml::open_vba_modules_from_container(
                &mut container,
                &mut session.strings,
            )?;
            let vba_ms = vba_start.elapsed().as_millis() as u64;

            #[cfg(feature = "perf-metrics")]
            let parse_time_ms = total_start.elapsed().as_millis() as u64;
            #[cfg(not(feature = "perf-metrics"))]
            let parse_time_ms = 0_u64;

            if profile_enabled {
                let vba_modules_count = vba_modules.as_ref().map(|items| items.len()).unwrap_or(0);
                println!(
                    "PERF_OPEN_PACKAGE total_ms={} workbook_ms={} data_mashup_ms={} vba_ms={} parse_time_ms={} sheets={} has_data_mashup={} vba_modules={}",
                    total_start.elapsed().as_millis() as u64,
                    workbook_ms,
                    data_mashup_ms,
                    vba_ms,
                    parse_time_ms,
                    workbook.sheets.len(),
                    data_mashup.is_some(),
                    vba_modules_count
                );
            }

            Ok(Self {
                workbook,
                data_mashup,
                vba_modules,
                #[cfg(feature = "perf-metrics")]
                parse_time_ms,
            })
        })
    }

    #[cfg(feature = "excel-open-xml")]
    /// Stream a workbook diff directly from two Open XML containers, skipping unchanged sheets
    /// based on ZIP central-directory fingerprints.
    ///
    /// This avoids fully parsing both workbooks up-front and can be orders of magnitude faster
    /// for mostly-identical comparisons.
    pub fn diff_openxml_streaming_fast<S: DiffSink>(
        old_reader: impl std::io::Read + std::io::Seek + 'static,
        new_reader: impl std::io::Read + std::io::Seek + 'static,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, OpenXmlDiffError> {
        Self::diff_openxml_streaming_fast_with_limits_and_progress(
            old_reader,
            new_reader,
            crate::ContainerLimits::default(),
            config,
            sink,
            &crate::NoProgress,
        )
    }

    #[cfg(feature = "excel-open-xml")]
    pub fn diff_openxml_streaming_fast_with_progress<S: DiffSink>(
        old_reader: impl std::io::Read + std::io::Seek + 'static,
        new_reader: impl std::io::Read + std::io::Seek + 'static,
        config: &DiffConfig,
        sink: &mut S,
        progress: &dyn ProgressCallback,
    ) -> Result<DiffSummary, OpenXmlDiffError> {
        Self::diff_openxml_streaming_fast_with_limits_and_progress(
            old_reader,
            new_reader,
            crate::ContainerLimits::default(),
            config,
            sink,
            progress,
        )
    }

    #[cfg(feature = "excel-open-xml")]
    pub fn diff_openxml_streaming_fast_with_limits<S: DiffSink>(
        old_reader: impl std::io::Read + std::io::Seek + 'static,
        new_reader: impl std::io::Read + std::io::Seek + 'static,
        limits: crate::ContainerLimits,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, OpenXmlDiffError> {
        Self::diff_openxml_streaming_fast_with_limits_and_progress(
            old_reader,
            new_reader,
            limits,
            config,
            sink,
            &crate::NoProgress,
        )
    }

    #[cfg(feature = "excel-open-xml")]
    pub fn diff_openxml_streaming_fast_with_limits_and_progress<S: DiffSink>(
        old_reader: impl std::io::Read + std::io::Seek + 'static,
        new_reader: impl std::io::Read + std::io::Seek + 'static,
        limits: crate::ContainerLimits,
        config: &DiffConfig,
        sink: &mut S,
        progress: &dyn ProgressCallback,
    ) -> Result<DiffSummary, OpenXmlDiffError> {
        crate::with_default_session(|session| {
            let mut old_container = crate::container::OpcContainer::open_from_reader_with_limits(
                old_reader,
                limits,
            )
            .map_err(crate::excel_open_xml::PackageError::from)?;
            let mut new_container = crate::container::OpcContainer::open_from_reader_with_limits(
                new_reader,
                limits,
            )
            .map_err(crate::excel_open_xml::PackageError::from)?;

            progress.on_progress("parse", 0.0);
            #[cfg(feature = "perf-metrics")]
            let fingerprint_started = Instant::now();
            let (old_grid_targets, new_grid_targets) =
                compute_sheet_grid_parse_targets(&mut old_container, &mut new_container)?;
            #[cfg(feature = "perf-metrics")]
            let fingerprint_ms = fingerprint_started.elapsed().as_millis() as u64;

            let old_pkg = open_workbook_package_from_container_with_grid_filter(
                &mut old_container,
                &mut session.strings,
                &old_grid_targets,
            )?;
            let new_pkg = open_workbook_package_from_container_with_grid_filter(
                &mut new_container,
                &mut session.strings,
                &new_grid_targets,
            )?;
            progress.on_progress("parse", 1.0);

            #[allow(unused_mut)]
            let mut summary = old_pkg.diff_streaming_with_progress_with_pool(
                &new_pkg,
                &mut session.strings,
                config,
                sink,
                progress,
            )?;

            #[cfg(feature = "perf-metrics")]
            if let Some(metrics) = summary.metrics.as_mut() {
                metrics.parse_time_ms = metrics.parse_time_ms.saturating_add(fingerprint_ms);
                metrics.total_time_ms = metrics.total_time_ms.saturating_add(fingerprint_ms);
                metrics.diff_time_ms = metrics.total_time_ms.saturating_sub(metrics.parse_time_ms);
            }

            Ok(summary)
        })
    }

    /// Diff this package against `other`, returning an in-memory [`DiffReport`].
    ///
    /// This collects all ops into memory and returns a report containing both the ops and the
    /// string table required to resolve [`StringId`] values referenced by ops.
    ///
    /// For very large workbooks, consider [`WorkbookPackage::diff_streaming`] instead.
    pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
        crate::with_default_session(|session| {
            self.diff_with_pool(other, &mut session.strings, config)
        })
    }

    /// Like [`WorkbookPackage::diff`], but reports best-effort progress via `progress`.
    pub fn diff_with_progress(
        &self,
        other: &Self,
        config: &DiffConfig,
        progress: &dyn ProgressCallback,
    ) -> DiffReport {
        crate::with_default_session(|session| {
            self.diff_with_progress_with_pool(other, &mut session.strings, config, progress)
        })
    }

    pub fn diff_with_pool(
        &self,
        other: &Self,
        pool: &mut crate::string_pool::StringPool,
        config: &DiffConfig,
    ) -> DiffReport {
        let mut report = {
            let mut ctx = DiffContext::new(pool, config);
            self.workbook.diff(&other.workbook, &mut ctx)
        };

        let mut object_ops =
            crate::object_diff::diff_named_ranges(&self.workbook, &other.workbook, pool);
        object_ops.extend(crate::object_diff::diff_charts(
            &self.workbook,
            &other.workbook,
            pool,
        ));
        object_ops.extend(crate::object_diff::diff_vba_modules(
            self.vba_modules.as_deref(),
            other.vba_modules.as_deref(),
            pool,
        ));
        report.ops.extend(object_ops);

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        report.ops.extend(m_ops);
        report.strings = pool.strings().to_vec();
        #[cfg(feature = "perf-metrics")]
        apply_parse_metrics(self, other, &mut report.metrics);
        append_permission_bindings_warnings(&mut report, &self.data_mashup, &other.data_mashup);
        report
    }

    pub fn diff_with_progress_with_pool(
        &self,
        other: &Self,
        pool: &mut crate::string_pool::StringPool,
        config: &DiffConfig,
        progress: &dyn ProgressCallback,
    ) -> DiffReport {
        let mut report = crate::engine::diff_workbooks_with_progress(
            &self.workbook,
            &other.workbook,
            pool,
            config,
            progress,
        );

        let mut object_ops =
            crate::object_diff::diff_named_ranges(&self.workbook, &other.workbook, pool);
        object_ops.extend(crate::object_diff::diff_charts(
            &self.workbook,
            &other.workbook,
            pool,
        ));
        object_ops.extend(crate::object_diff::diff_vba_modules(
            self.vba_modules.as_deref(),
            other.vba_modules.as_deref(),
            pool,
        ));
        report.ops.extend(object_ops);

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        report.ops.extend(m_ops);
        report.strings = pool.strings().to_vec();
        #[cfg(feature = "perf-metrics")]
        apply_parse_metrics(self, other, &mut report.metrics);
        append_permission_bindings_warnings(&mut report, &self.data_mashup, &other.data_mashup);
        report
    }

    /// Diff this package against `other`, streaming ops into `sink`.
    ///
    /// This is the preferred API for very large workbooks because it does not require holding
    /// the entire op list in memory. Instead, ops are emitted incrementally and a [`DiffSummary`]
    /// is returned at the end.
    ///
    /// Streaming output follows the contract in `docs/streaming_contract.md` (determinism,
    /// sink lifecycle, and JSONL string table invariants).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use excel_diff::{DiffConfig, JsonLinesSink, WorkbookPackage};
    /// use std::fs::File;
    /// use std::io::{self, BufWriter};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let old_pkg = WorkbookPackage::open(File::open("old.xlsx")?)?;
    /// let new_pkg = WorkbookPackage::open(File::open("new.xlsx")?)?;
    ///
    /// let stdout = io::stdout();
    /// let mut sink = JsonLinesSink::new(BufWriter::new(stdout.lock()));
    /// let summary = old_pkg.diff_streaming(&new_pkg, &DiffConfig::default(), &mut sink)?;
    /// eprintln!("complete={} ops={}", summary.complete, summary.op_count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            self.diff_streaming_with_pool(other, &mut session.strings, config, sink)
        })
    }

    /// Like [`WorkbookPackage::diff_streaming`], but reports best-effort progress via `progress`.
    pub fn diff_streaming_with_progress<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
        progress: &dyn ProgressCallback,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            self.diff_streaming_with_progress_with_pool(
                other,
                &mut session.strings,
                config,
                sink,
                progress,
            )
        })
    }

    /// Streaming variant of [`WorkbookPackage::diff`], using a caller-provided string pool.
    ///
    /// Most callers should prefer [`WorkbookPackage::diff_streaming`], which uses the default
    /// session string pool internally.
    pub fn diff_streaming_with_pool<S: DiffSink>(
        &self,
        other: &Self,
        pool: &mut crate::string_pool::StringPool,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        let mut object_ops =
            crate::object_diff::diff_named_ranges(&self.workbook, &other.workbook, pool);
        object_ops.extend(crate::object_diff::diff_charts(
            &self.workbook,
            &other.workbook,
            pool,
        ));
        object_ops.extend(crate::object_diff::diff_vba_modules(
            self.vba_modules.as_deref(),
            other.vba_modules.as_deref(),
            pool,
        ));

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        let grid_result = {
            let mut no_finish = NoFinishSink::new(sink);
            crate::engine::try_diff_workbooks_streaming(
                &self.workbook,
                &other.workbook,
                pool,
                config,
                &mut no_finish,
            )
        };

        let mut summary = match grid_result {
            Ok(summary) => summary,
            Err(e) => {
                let _ = sink.finish();
                return Err(e);
            }
        };

        for op in object_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        for op in m_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        sink.finish()?;

        #[cfg(feature = "perf-metrics")]
        apply_parse_metrics(self, other, &mut summary.metrics);

        append_permission_bindings_warnings_summary(&mut summary, &self.data_mashup, &other.data_mashup);
        Ok(summary)
    }

    pub fn diff_streaming_with_progress_with_pool<S: DiffSink>(
        &self,
        other: &Self,
        pool: &mut crate::string_pool::StringPool,
        config: &DiffConfig,
        sink: &mut S,
        progress: &dyn ProgressCallback,
    ) -> Result<DiffSummary, DiffError> {
        let mut object_ops =
            crate::object_diff::diff_named_ranges(&self.workbook, &other.workbook, pool);
        object_ops.extend(crate::object_diff::diff_charts(
            &self.workbook,
            &other.workbook,
            pool,
        ));
        object_ops.extend(crate::object_diff::diff_vba_modules(
            self.vba_modules.as_deref(),
            other.vba_modules.as_deref(),
            pool,
        ));

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        let grid_result = {
            let mut no_finish = NoFinishSink::new(sink);
            crate::engine::try_diff_workbooks_streaming_with_progress(
                &self.workbook,
                &other.workbook,
                pool,
                config,
                &mut no_finish,
                progress,
            )
        };

        let mut summary = match grid_result {
            Ok(summary) => summary,
            Err(e) => {
                let _ = sink.finish();
                return Err(e);
            }
        };

        for op in object_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        for op in m_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        sink.finish()?;

        #[cfg(feature = "perf-metrics")]
        apply_parse_metrics(self, other, &mut summary.metrics);

        append_permission_bindings_warnings_summary(&mut summary, &self.data_mashup, &other.data_mashup);
        Ok(summary)
    }

    /// Diff a single sheet using key-based row alignment ("database mode").
    ///
    /// `sheet_name` must exist in both workbooks (matching is case-insensitive). `key_columns`
    /// are 0-based column indices (A=0, B=1, ...).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use excel_diff::{DiffConfig, WorkbookPackage};
    /// use std::fs::File;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let old_pkg = WorkbookPackage::open(File::open("old.xlsx")?)?;
    /// let new_pkg = WorkbookPackage::open(File::open("new.xlsx")?)?;
    ///
    /// let keys = vec![0u32, 2u32]; // A,C
    /// let report = old_pkg.diff_database_mode(&new_pkg, "Data", &keys, &DiffConfig::default())?;
    /// println!("complete={} ops={}", report.complete, report.ops.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn diff_database_mode(
        &self,
        other: &Self,
        sheet_name: &str,
        key_columns: &[u32],
        config: &DiffConfig,
    ) -> Result<DiffReport, DiffError> {
        crate::with_default_session(|session| {
            self.diff_database_mode_with_pool(
                other,
                sheet_name,
                key_columns,
                &mut session.strings,
                config,
            )
        })
    }

    /// Like [`WorkbookPackage::diff_database_mode`], but uses a caller-provided string pool.
    pub fn diff_database_mode_with_pool(
        &self,
        other: &Self,
        sheet_name: &str,
        key_columns: &[u32],
        pool: &mut StringPool,
        config: &DiffConfig,
    ) -> Result<DiffReport, DiffError> {
        let (old_sheet, new_sheet, sheet_id) =
            find_sheets_case_insensitive(&self.workbook, &other.workbook, sheet_name, pool)?;

        let mut sink = VecSink::new();
        let mut op_count = 0usize;

        let summary = crate::engine::try_diff_grids_database_mode_streaming(
            sheet_id,
            &old_sheet.grid,
            &new_sheet.grid,
            key_columns,
            pool,
            config,
            &mut sink,
            &mut op_count,
        )?;

        let mut object_ops =
            crate::object_diff::diff_named_ranges(&self.workbook, &other.workbook, pool);
        object_ops.extend(crate::object_diff::diff_charts(
            &self.workbook,
            &other.workbook,
            pool,
        ));
        object_ops.extend(crate::object_diff::diff_vba_modules(
            self.vba_modules.as_deref(),
            other.vba_modules.as_deref(),
            pool,
        ));

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        let mut ops = sink.into_ops();
        ops.extend(object_ops);
        ops.extend(m_ops);

        let strings = pool.strings().to_vec();
        let mut report = DiffReport::from_ops_and_summary(ops, summary, strings);
        append_permission_bindings_warnings(&mut report, &self.data_mashup, &other.data_mashup);
        Ok(report)
    }

    /// Streaming database mode diff. Emits ops into `sink` and returns a [`DiffSummary`].
    ///
    /// Streaming output follows the contract in `docs/streaming_contract.md`.
    pub fn diff_database_mode_streaming<S: DiffSink>(
        &self,
        other: &Self,
        sheet_name: &str,
        key_columns: &[u32],
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            self.diff_database_mode_streaming_with_pool(
                other,
                sheet_name,
                key_columns,
                &mut session.strings,
                config,
                sink,
            )
        })
    }

    /// Like [`WorkbookPackage::diff_database_mode_streaming`], but uses a caller-provided string pool.
    pub fn diff_database_mode_streaming_with_pool<S: DiffSink>(
        &self,
        other: &Self,
        sheet_name: &str,
        key_columns: &[u32],
        pool: &mut StringPool,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        let mut object_ops =
            crate::object_diff::diff_named_ranges(&self.workbook, &other.workbook, pool);
        object_ops.extend(crate::object_diff::diff_charts(
            &self.workbook,
            &other.workbook,
            pool,
        ));
        object_ops.extend(crate::object_diff::diff_vba_modules(
            self.vba_modules.as_deref(),
            other.vba_modules.as_deref(),
            pool,
        ));

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        let (old_sheet, new_sheet, sheet_id) =
            find_sheets_case_insensitive(&self.workbook, &other.workbook, sheet_name, pool)?;

        let grid_result = {
            let mut no_finish = NoFinishSink::new(sink);
            crate::engine::try_diff_grids_database_mode_streaming(
                sheet_id,
                &old_sheet.grid,
                &new_sheet.grid,
                key_columns,
                pool,
                config,
                &mut no_finish,
                &mut 0usize,
            )
        };

        let mut summary = match grid_result {
            Ok(summary) => summary,
            Err(e) => {
                let _ = sink.finish();
                return Err(e);
            }
        };

        for op in object_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        for op in m_ops {
            if let Err(e) = sink.emit(op) {
                let _ = sink.finish();
                return Err(e);
            }
            summary.op_count = summary.op_count.saturating_add(1);
        }

        sink.finish()?;

        append_permission_bindings_warnings_summary(&mut summary, &self.data_mashup, &other.data_mashup);
        Ok(summary)
    }
}

/// A parsed PBIX/PBIT package (Power BI) containing Power Query data.
#[derive(Debug, Clone)]
pub struct PbixPackage {
    pub(crate) data_mashup: Option<DataMashup>,
    #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
    pub(crate) model_schema: Option<crate::tabular_schema::RawTabularModel>,
}

impl PbixPackage {
    #[cfg(feature = "excel-open-xml")]
    pub fn open<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
    ) -> Result<Self, crate::excel_open_xml::PackageError> {
        let mut container = ZipContainer::open_from_reader(reader)?;

        let data_mashup_opt = container.read_file_optional_checked("DataMashup")?;

        let data_mashup = if let Some(bytes) = data_mashup_opt {
            let raw = crate::datamashup_framing::parse_data_mashup(&bytes)?;
            Some(crate::datamashup::build_data_mashup(&raw)?)
        } else {
            None
        };

        #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
        let mut model_schema = None;

        if data_mashup.is_none() {
            if looks_like_pbix(&container) {
                #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
                {
                    if let Some(bytes) = container.read_file_optional_checked("DataModelSchema")? {
                        model_schema =
                            Some(crate::tabular_schema::parse_data_model_schema(&bytes)?);
                        return Ok(Self {
                            data_mashup,
                            model_schema,
                        });
                    }
                }

                return Err(crate::excel_open_xml::PackageError::NoDataMashupUseTabularModel);
            }

            return Err(crate::excel_open_xml::PackageError::UnsupportedFormat {
                message: "missing DataMashup at ZIP root".to_string(),
            });
        }

        Ok(Self {
            data_mashup,
            #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
            model_schema,
        })
    }

    #[cfg(feature = "excel-open-xml")]
    pub fn open_with_limits<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
        limits: crate::ContainerLimits,
    ) -> Result<Self, crate::excel_open_xml::PackageError> {
        let mut container = ZipContainer::open_from_reader_with_limits(reader, limits)?;

        let data_mashup_opt = container.read_file_optional_checked("DataMashup")?;

        let data_mashup = if let Some(bytes) = data_mashup_opt {
            let raw = crate::datamashup_framing::parse_data_mashup(&bytes)?;
            Some(crate::datamashup::build_data_mashup(&raw)?)
        } else {
            None
        };

        #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
        let mut model_schema = None;

        if data_mashup.is_none() {
            if looks_like_pbix(&container) {
                #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
                {
                    if let Some(bytes) = container.read_file_optional_checked("DataModelSchema")? {
                        model_schema =
                            Some(crate::tabular_schema::parse_data_model_schema(&bytes)?);
                        return Ok(Self {
                            data_mashup,
                            model_schema,
                        });
                    }
                }

                return Err(crate::excel_open_xml::PackageError::NoDataMashupUseTabularModel);
            }

            return Err(crate::excel_open_xml::PackageError::UnsupportedFormat {
                message: "missing DataMashup at ZIP root".to_string(),
            });
        }

        Ok(Self {
            data_mashup,
            #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
            model_schema,
        })
    }

    pub fn data_mashup(&self) -> Option<&DataMashup> {
        self.data_mashup.as_ref()
    }

    pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
        crate::with_default_session(|session| {
            let mut report = DiffReport::new(Vec::new());
            let mut ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            report.ops.append(&mut ops);

            #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
            {
                let old_raw = self.model_schema.as_ref();
                let new_raw = other.model_schema.as_ref();

                if old_raw.is_some() || new_raw.is_some() {
                    let old_model = old_raw
                        .map(|r| crate::tabular_schema::build_model(r, &mut session.strings))
                        .unwrap_or_default();

                    let new_model = new_raw
                        .map(|r| crate::tabular_schema::build_model(r, &mut session.strings))
                        .unwrap_or_default();

                    let model_result = crate::model_diff::diff_models(
                        &old_model,
                        &new_model,
                        &mut session.strings,
                        config,
                    );
                    report.ops.extend(model_result.ops);
                    if !model_result.complete {
                        report.complete = false;
                        report.warnings.extend(model_result.warnings);
                    }
                }
            }

            report.strings = session.strings.strings().to_vec();
            append_permission_bindings_warnings(&mut report, &self.data_mashup, &other.data_mashup);
            report
        })
    }

    /// Stream a PBIX/PBIT diff into `sink`.
    ///
    /// Streaming output follows the contract in `docs/streaming_contract.md`.
    pub fn diff_streaming<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        crate::with_default_session(|session| {
            self.diff_streaming_with_pool(other, &mut session.strings, config, sink)
        })
    }

    pub fn diff_streaming_with_pool<S: DiffSink>(
        &self,
        other: &Self,
        pool: &mut StringPool,
        config: &DiffConfig,
        sink: &mut S,
    ) -> Result<DiffSummary, DiffError> {
        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
        let model_result = {
            let old_raw = self.model_schema.as_ref();
            let new_raw = other.model_schema.as_ref();

            if old_raw.is_some() || new_raw.is_some() {
                let old_model = old_raw
                    .map(|r| crate::tabular_schema::build_model(r, pool))
                    .unwrap_or_default();

                let new_model = new_raw
                    .map(|r| crate::tabular_schema::build_model(r, pool))
                    .unwrap_or_default();

                Some(crate::model_diff::diff_models(&old_model, &new_model, pool, config))
            } else {
                None
            }
        };

        sink.begin(pool)?;
        let mut finish_guard = SinkFinishGuard::new(sink);

        let mut op_count = 0usize;
        #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
        let (mut complete, mut warnings) = (true, Vec::new());
        #[cfg(not(all(feature = "model-diff", feature = "excel-open-xml")))]
        let (complete, warnings) = (true, Vec::new());
        for op in m_ops {
            sink.emit(op)?;
            op_count = op_count.saturating_add(1);
        }

        #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
        if let Some(model_result) = model_result {
            for op in model_result.ops {
                sink.emit(op)?;
                op_count = op_count.saturating_add(1);
            }

            if !model_result.complete {
                complete = false;
                warnings.extend(model_result.warnings);
            }
        }

        finish_guard.finish_and_disarm()?;

        let mut summary = DiffSummary {
            complete,
            warnings,
            op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        };
        append_permission_bindings_warnings_summary(&mut summary, &self.data_mashup, &other.data_mashup);
        Ok(summary)
    }

    pub fn diff_streaming_with_progress<S: DiffSink>(
        &self,
        other: &Self,
        config: &DiffConfig,
        sink: &mut S,
        progress: &dyn ProgressCallback,
    ) -> Result<DiffSummary, DiffError> {
        progress.on_progress("m_diff", 0.0);
        let out = self.diff_streaming(other, config, sink);
        progress.on_progress("m_diff", 1.0);
        out
    }
}

#[cfg(feature = "perf-metrics")]
fn apply_parse_metrics(
    old_pkg: &WorkbookPackage,
    new_pkg: &WorkbookPackage,
    metrics: &mut Option<DiffMetrics>,
) {
    let Some(m) = metrics.as_mut() else {
        return;
    };

    let added = old_pkg
        .parse_time_ms
        .saturating_add(new_pkg.parse_time_ms);
    m.parse_time_ms = m.parse_time_ms.saturating_add(added);
    m.total_time_ms = m.total_time_ms.saturating_add(added);
    m.diff_time_ms = m.total_time_ms.saturating_sub(m.parse_time_ms);
}

#[cfg(feature = "excel-open-xml")]
fn looks_like_pbix(container: &ZipContainer) -> bool {
    container.file_names().any(|name| {
        name == "Report/Layout"
            || name == "Report/Version"
            || name == "DataModelSchema"
            || name == "DataModel"
            || name == "Connections"
            || name == "DiagramLayout"
    })
}

fn find_sheets_case_insensitive<'a>(
    old_wb: &'a Workbook,
    new_wb: &'a Workbook,
    sheet_name: &str,
    pool: &StringPool,
) -> Result<(&'a Sheet, &'a Sheet, SheetId), DiffError> {
    let sheet_name_lower = sheet_name.to_lowercase();

    let old_sheet = old_wb.sheets.iter().find(|s| {
        let name = pool.resolve(s.name);
        name.to_lowercase() == sheet_name_lower
    });

    let new_sheet = new_wb.sheets.iter().find(|s| {
        let name = pool.resolve(s.name);
        name.to_lowercase() == sheet_name_lower
    });

    match (old_sheet, new_sheet) {
        (Some(old), Some(new)) => {
            let sheet_id = old.name;
            Ok((old, new, sheet_id))
        }
        _ => {
            let mut available: Vec<String> = old_wb
                .sheets
                .iter()
                .map(|s| pool.resolve(s.name).to_string())
                .collect();
            for s in &new_wb.sheets {
                let name = pool.resolve(s.name).to_string();
                if !available.iter().any(|n| n.to_lowercase() == name.to_lowercase()) {
                    available.push(name);
                }
            }
            available.sort();
            Err(DiffError::SheetNotFound {
                requested: sheet_name.to_string(),
                available,
            })
        }
    }
}

fn append_permission_bindings_warnings(
    report: &mut DiffReport,
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
) {
    for warning in collect_permission_bindings_warnings(old_dm, new_dm) {
        report.add_warning(warning);
    }
}

fn append_permission_bindings_warnings_summary(
    summary: &mut DiffSummary,
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
) {
    let warnings = collect_permission_bindings_warnings(old_dm, new_dm);
    if warnings.is_empty() {
        return;
    }
    summary.complete = false;
    summary.warnings.extend(warnings);
}

fn collect_permission_bindings_warnings(
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
) -> Vec<String> {
    let mut warn_unverifiable = false;
    let mut warn_invalid = false;

    for dm in [old_dm, new_dm].iter().filter_map(|dm| dm.as_ref()) {
        match dm.permission_bindings_status {
            PermissionBindingsStatus::Unverifiable => warn_unverifiable = true,
            PermissionBindingsStatus::InvalidOrTampered => warn_invalid = true,
            _ => {}
        }
    }

    let mut warnings = Vec::new();
    if warn_invalid {
        if let Some(warning) =
            permission_bindings_warning(PermissionBindingsStatus::InvalidOrTampered)
        {
            warnings.push(warning);
        }
    }
    if warn_unverifiable {
        if let Some(warning) =
            permission_bindings_warning(PermissionBindingsStatus::Unverifiable)
        {
            warnings.push(warning);
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
    use super::*;
    #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
    use crate::{
        DiffOp, Metadata, PackageParts, PackageXml, PermissionBindingsStatus, Permissions,
        SectionDocument,
    };

    #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
    use crate::tabular_schema::{RawMeasure, RawTabularModel};

    #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
    fn make_dm(section_source: &str) -> DataMashup {
        DataMashup {
            version: 0,
            package_parts: PackageParts {
                package_xml: PackageXml {
                    raw_xml: "<Package/>".to_string(),
                },
                main_section: SectionDocument {
                    source: section_source.to_string(),
                },
                embedded_contents: Vec::new(),
            },
            permissions: Permissions::default(),
            metadata: Metadata { formulas: Vec::new() },
            permission_bindings_raw: Vec::new(),
            permission_bindings_status: PermissionBindingsStatus::Missing,
        }
    }

    #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
    #[test]
    fn pbix_streaming_orders_query_ops_before_model_ops() {
        let dm_a = make_dm("section Section1;\nshared Foo = 1;");
        let dm_b = make_dm("section Section1;\nshared Bar = 1;");

        let raw_a = RawTabularModel {
            tables: Vec::new(),
            relationships: Vec::new(),
            measures: vec![RawMeasure {
                full_name: "Table/Measure1".to_string(),
                expression: "1".to_string(),
            }],
        };
        let raw_b = RawTabularModel {
            tables: Vec::new(),
            relationships: Vec::new(),
            measures: vec![RawMeasure {
                full_name: "Table/Measure1".to_string(),
                expression: "2".to_string(),
            }],
        };

        let pkg_a = PbixPackage {
            data_mashup: Some(dm_a),
            model_schema: Some(raw_a),
        };
        let pkg_b = PbixPackage {
            data_mashup: Some(dm_b),
            model_schema: Some(raw_b),
        };

        let mut pool = StringPool::new();
        let mut sink = VecSink::new();
        pkg_a
            .diff_streaming_with_pool(&pkg_b, &mut pool, &DiffConfig::default(), &mut sink)
            .expect("pbix streaming should succeed");
        let ops = sink.into_ops();

        assert!(ops.iter().any(DiffOp::is_m_op), "expected query ops");
        assert!(
            ops.iter().any(DiffOp::is_model_op),
            "expected model ops"
        );

        let mut seen_model = false;
        for op in ops {
            if op.is_model_op() {
                seen_model = true;
                continue;
            }
            if op.is_m_op() && seen_model {
                panic!("query op appeared after model ops");
            }
        }
    }
}
