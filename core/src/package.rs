use crate::config::DiffConfig;
use crate::datamashup::DataMashup;
use crate::diff::{DiffError, DiffReport, DiffSummary};
use crate::sink::DiffSink;
use crate::workbook::Workbook;

#[derive(Debug, Clone)]
pub struct WorkbookPackage {
    pub workbook: Workbook,
    pub data_mashup: Option<DataMashup>,
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
            Ok(Self {
                workbook,
                data_mashup,
            })
        })
    }

    pub fn diff(&self, other: &Self, config: &DiffConfig) -> DiffReport {
        crate::with_default_session(|session| {
            let mut report = crate::engine::diff_workbooks(
                &self.workbook,
                &other.workbook,
                &mut session.strings,
                config,
            );

            let m_ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            report.ops.extend(m_ops);
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
            let mut summary = crate::engine::try_diff_workbooks_streaming(
                &self.workbook,
                &other.workbook,
                &mut session.strings,
                config,
                sink,
            )?;

            let m_ops = crate::m_diff::diff_m_ops_for_packages(
                &self.data_mashup,
                &other.data_mashup,
                &mut session.strings,
                config,
            );

            for op in m_ops {
                sink.emit(op)?;
                summary.op_count += 1;
            }

            Ok(summary)
        })
    }
}

