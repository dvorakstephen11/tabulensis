use crate::config::DiffConfig;
use crate::container::ZipContainer;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary, SheetId};
use crate::diffable::{DiffContext, Diffable};
use crate::progress::ProgressCallback;
use crate::sink::{DiffSink, NoFinishSink, VecSink};
use crate::string_pool::StringId;
use crate::string_pool::StringPool;
use crate::workbook::{Sheet, Workbook};
#[cfg(feature = "perf-metrics")]
use crate::perf::DiffMetrics;

/// The kind of VBA module contained in an `.xlsm` workbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbaModuleType {
    /// A standard module (e.g., `Module1`).
    Standard,
    /// A class module.
    Class,
    /// A form module.
    Form,
    /// A document module (e.g., `ThisWorkbook`, sheet modules).
    Document,
}

/// A VBA module extracted from a workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaModule {
    /// Module name (interned in the associated string pool).
    pub name: StringId,
    /// Module type (standard/class/form/document).
    pub module_type: VbaModuleType,
    /// Raw module source code.
    pub code: String,
}

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
            #[cfg(feature = "perf-metrics")]
            let start = std::time::Instant::now();
            let mut container = crate::container::OpcContainer::open_from_reader(reader)?;
            let workbook = crate::excel_open_xml::open_workbook_from_container(
                &mut container,
                &mut session.strings,
            )?;
            let raw = crate::excel_open_xml::open_data_mashup_from_container(&mut container)?;
            let data_mashup = match raw {
                Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
                None => None,
            };
            let vba_modules =
                crate::excel_open_xml::open_vba_modules_from_container(&mut container, &mut session.strings)?;
            Ok(Self {
                workbook,
                data_mashup,
                vba_modules,
                #[cfg(feature = "perf-metrics")]
                parse_time_ms: start.elapsed().as_millis() as u64,
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
            #[cfg(feature = "perf-metrics")]
            let start = std::time::Instant::now();
            let mut container =
                crate::container::OpcContainer::open_from_reader_with_limits(reader, limits)?;
            let workbook = crate::excel_open_xml::open_workbook_from_container(
                &mut container,
                &mut session.strings,
            )?;
            let raw = crate::excel_open_xml::open_data_mashup_from_container(&mut container)?;
            let data_mashup = match raw {
                Some(raw) => Some(crate::datamashup::build_data_mashup(&raw)?),
                None => None,
            };
            let vba_modules =
                crate::excel_open_xml::open_vba_modules_from_container(&mut container, &mut session.strings)?;
            Ok(Self {
                workbook,
                data_mashup,
                vba_modules,
                #[cfg(feature = "perf-metrics")]
                parse_time_ms: start.elapsed().as_millis() as u64,
            })
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
        report
    }

    /// Diff this package against `other`, streaming ops into `sink`.
    ///
    /// This is the preferred API for very large workbooks because it does not require holding
    /// the entire op list in memory. Instead, ops are emitted incrementally and a [`DiffSummary`]
    /// is returned at the end.
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
        Ok(DiffReport::from_ops_and_summary(ops, summary, strings))
    }

    /// Streaming database mode diff. Emits ops into `sink` and returns a [`DiffSummary`].
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

                    let mut model_ops =
                        crate::model_diff::diff_models(&old_model, &new_model, &mut session.strings);
                    report.ops.append(&mut model_ops);
                }
            }

            report.strings = session.strings.strings().to_vec();
            report
        })
    }

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
        sink.begin(pool)?;

        let m_ops = crate::m_diff::diff_m_ops_for_packages(
            &self.data_mashup,
            &other.data_mashup,
            pool,
            config,
        );

        let mut op_count = 0usize;
        for op in m_ops {
            sink.emit(op)?;
            op_count = op_count.saturating_add(1);
        }

        #[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
        {
            let old_raw = self.model_schema.as_ref();
            let new_raw = other.model_schema.as_ref();

            if old_raw.is_some() || new_raw.is_some() {
                let old_model = old_raw
                    .map(|r| crate::tabular_schema::build_model(r, pool))
                    .unwrap_or_default();

                let new_model = new_raw
                    .map(|r| crate::tabular_schema::build_model(r, pool))
                    .unwrap_or_default();

                let model_ops = crate::model_diff::diff_models(&old_model, &new_model, pool);
                for op in model_ops {
                    sink.emit(op)?;
                    op_count = op_count.saturating_add(1);
                }
            }
        }

        sink.finish()?;

        Ok(DiffSummary {
            complete: true,
            warnings: Vec::new(),
            op_count,
            #[cfg(feature = "perf-metrics")]
            metrics: None,
        })
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
