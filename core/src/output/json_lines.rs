use crate::diff::{DiffError, DiffOp};
use crate::sink::DiffSink;
use crate::string_pool::StringPool;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct JsonLinesHeader<'a> {
    kind: &'static str,
    version: &'a str,
    strings: &'a [String],
}

pub struct JsonLinesSink<W: Write> {
    w: W,
    wrote_header: bool,
    version: &'static str,
}

impl<W: Write> JsonLinesSink<W> {
    pub fn new(w: W) -> Self {
        Self {
            w,
            wrote_header: false,
            version: crate::diff::DiffReport::SCHEMA_VERSION,
        }
    }

    pub fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        if self.wrote_header {
            return Ok(());
        }

        let header = JsonLinesHeader {
            kind: "Header",
            version: self.version,
            strings: pool.strings(),
        };

        serde_json::to_writer(&mut self.w, &header)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.w
            .write_all(b"\n")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;

        self.wrote_header = true;
        Ok(())
    }
}

impl<W: Write> DiffSink for JsonLinesSink<W> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        serde_json::to_writer(&mut self.w, &op)
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        self.w
            .write_all(b"\n")
            .map_err(|e| DiffError::SinkError { message: e.to_string() })?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.w
            .flush()
            .map_err(|e| DiffError::SinkError { message: e.to_string() })
    }
}
