use crate::config::DiffConfig;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary, SheetId};
use crate::sink::{DiffSink, NoFinishSink, VecSink};
use crate::string_pool::StringId;
use crate::string_pool::StringPool;
use crate::workbook::{Sheet, Workbook};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbaModuleType {
    Standard,
    Class,
    Form,
    Document,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaModule {
    pub name: StringId,
    pub module_type: VbaModuleType,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct WorkbookPackage {
    pub workbook: Workbook,
    pub data_mashup: Option<DataMashup>,
    pub vba_modules: Option<Vec<VbaModule>>,
}

impl From<Workbook> for WorkbookPackage {
    fn from(workbook: Workbook) -> Self {
        Self {
            workbook,
            data_mashup: None,
            vba_modules: None,
        }
    }
}

impl WorkbookPackage {
    #[cfg(feature = "excel-open-xml")]
    pub fn open<R: std::io::Read + std::io::Seek + 'static>(
        reader: R,
    ) -> Result<Self, crate::excel_open_xml::PackageError> {
        crate::with_default_session(|session| {
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
            })
        })
    }

    pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
        crate::with_default_session(|session| {
            self.diff_with_pool(other, &mut session.strings, config)
        })
    }

    pub fn diff_with_pool(
        &self,
        other: &Self,
        pool: &mut crate::string_pool::StringPool,
        config: &DiffConfig,
    ) -> DiffReport {
        let mut report =
            crate::engine::diff_workbooks(&self.workbook, &other.workbook, pool, config);

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
        report
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

        Ok(summary)
    }

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
