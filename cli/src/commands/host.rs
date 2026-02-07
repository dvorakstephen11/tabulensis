use anyhow::{Context, Result};
use excel_diff::{PbixPackage, WorkbookPackage};
use std::fs::File;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostKind {
    Workbook,
    Pbix,
}

pub(crate) enum Host {
    Workbook(WorkbookPackage),
    Pbix(PbixPackage),
}

pub(crate) fn host_kind_from_path(path: &Path) -> Option<HostKind> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "xlsx" | "xlsm" | "xltx" | "xltm" | "xlsb" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

pub(crate) fn open_host(path: &Path, kind: HostKind, label: &str) -> Result<Host> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open {} file: {}", label, path.display()))?;

    let host =
        match kind {
            HostKind::Workbook => {
                Host::Workbook(WorkbookPackage::open(file).with_context(|| {
                    format!("Failed to parse {} workbook: {}", label, path.display())
                })?)
            }
            HostKind::Pbix => Host::Pbix(PbixPackage::open(file).with_context(|| {
                format!("Failed to parse {} PBIX/PBIT: {}", label, path.display())
            })?),
        };

    Ok(host)
}
