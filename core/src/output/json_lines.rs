use crate::diff::{DiffError, DiffOp};
use crate::sink::DiffSink;
use crate::string_pool::StringPool;
use std::io::Write;

#[cfg(feature = "custom-jsonl")]
use crate::output::json_write;

#[cfg(not(feature = "custom-jsonl"))]
use serde::Serialize;

#[cfg(not(feature = "custom-jsonl"))]
#[derive(Serialize)]
struct JsonLinesHeader<'a> {
    kind: &'static str,
    version: &'a str,
    strings: &'a [String],
}

/// A [`DiffSink`] that writes a JSON Lines stream.
///
/// The first line is a header containing the schema version and the string table. Each
/// subsequent line is a JSON-serialized [`DiffOp`].
///
/// All strings referenced by emitted ops must be interned before `begin`, because the
/// header captures the string table once. See `docs/streaming_contract.md`.
pub struct JsonLinesSink<W: Write> {
    w: W,
    wrote_header: bool,
    version: &'static str,
    #[cfg(feature = "custom-jsonl")]
    scratch: Vec<u8>,
}

impl<W: Write> JsonLinesSink<W> {
    /// Create a JSON Lines sink that writes to `w`.
    ///
    /// The output format is:
    ///
    /// 1. A header line: `{ "kind": "Header", "version": "...", "strings": [...] }`
    /// 2. One JSON-serialized [`DiffOp`] per line
    ///
    /// Ops contain interned [`crate::StringId`] values that index into the header's `strings` table.
    pub fn new(w: W) -> Self {
        Self {
            w,
            wrote_header: false,
            version: crate::diff::DiffReport::SCHEMA_VERSION,
            #[cfg(feature = "custom-jsonl")]
            scratch: Vec::new(),
        }
    }

    /// Write the header line (idempotent).
    pub fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        if self.wrote_header {
            return Ok(());
        }

        #[cfg(feature = "custom-jsonl")]
        {
            self.scratch.clear();
            json_write::write_jsonl_header(&mut self.scratch, self.version, pool.strings())
                .map_err(|e| DiffError::SinkError {
                    message: e.to_string(),
                })?;
            self.scratch.push(b'\n');
            self.w
                .write_all(&self.scratch)
                .map_err(|e| DiffError::SinkError {
                    message: e.to_string(),
                })?;
        }

        #[cfg(not(feature = "custom-jsonl"))]
        {
            let header = JsonLinesHeader {
                kind: "Header",
                version: self.version,
                strings: pool.strings(),
            };

            serde_json::to_writer(&mut self.w, &header).map_err(|e| DiffError::SinkError {
                message: e.to_string(),
            })?;
            self.w.write_all(b"\n").map_err(|e| DiffError::SinkError {
                message: e.to_string(),
            })?;
        }

        self.wrote_header = true;
        Ok(())
    }
}

impl<W: Write> DiffSink for JsonLinesSink<W> {
    fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        JsonLinesSink::begin(self, pool)
    }

    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        #[cfg(feature = "custom-jsonl")]
        {
            self.scratch.clear();
            json_write::write_diff_op(&mut self.scratch, &op).map_err(|e| {
                DiffError::SinkError {
                    message: e.to_string(),
                }
            })?;
            self.scratch.push(b'\n');
            self.w
                .write_all(&self.scratch)
                .map_err(|e| DiffError::SinkError {
                    message: e.to_string(),
                })?;
            return Ok(());
        }

        #[cfg(not(feature = "custom-jsonl"))]
        {
            serde_json::to_writer(&mut self.w, &op).map_err(|e| DiffError::SinkError {
                message: e.to_string(),
            })?;
            self.w.write_all(b"\n").map_err(|e| DiffError::SinkError {
                message: e.to_string(),
            })?;
            return Ok(());
        }
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        self.w.flush().map_err(|e| DiffError::SinkError {
            message: e.to_string(),
        })
    }
}
