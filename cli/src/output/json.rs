use anyhow::Result;
use excel_diff::DiffReport;
use serde::Serialize;
use std::io::Write;

pub fn write_json_report<W: Write>(w: &mut W, report: &DiffReport) -> Result<()> {
    write_json_value(w, report)
}

pub fn write_json_value<W: Write, T: Serialize>(w: &mut W, value: &T) -> Result<()> {
    serde_json::to_writer_pretty(&mut *w, value)?;
    writeln!(w)?;
    Ok(())
}
