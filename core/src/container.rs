//! ZIP container handling.
//!
//! Provides abstraction over ZIP-based packages and validates OPC
//! requirements for Office Open XML containers.

use std::io::{Read, Seek};
use thiserror::Error;
use zip::result::ZipError;
use zip::ZipArchive;

use crate::error_codes;

#[derive(Debug, Clone, Copy)]
pub struct ContainerLimits {
    pub max_entries: usize,
    pub max_part_uncompressed_bytes: u64,
    pub max_total_uncompressed_bytes: u64,
}

/// ZIP central-directory fingerprint for an entry (uncompressed data).
///
/// This can be used to cheaply detect identical parts across two OPC packages without
/// reading/decompressing the part contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZipEntryFingerprint {
    pub crc32: u32,
    pub size: u64,
}

impl Default for ContainerLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_part_uncompressed_bytes: 100 * 1024 * 1024,
            max_total_uncompressed_bytes: 500 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContainerError {
    #[error("[EXDIFF_CTR_001] I/O error: {0}. Suggestion: check the file path and permissions.")]
    Io(#[from] std::io::Error),
    #[error("[EXDIFF_CTR_002] ZIP error: {0}. Suggestion: verify the file is a valid .xlsx and not corrupt.")]
    Zip(String),
    #[error("[EXDIFF_CTR_003] not a ZIP container. Suggestion: verify the input is a ZIP-based .xlsx file.")]
    NotZipContainer,
    #[error("[EXDIFF_CTR_004] not an OPC package (missing [Content_Types].xml). Suggestion: verify the file is a valid .xlsx workbook.")]
    NotOpcPackage,
    #[error("[EXDIFF_CTR_005] archive has too many entries: {entries} (limit: {max_entries}). Suggestion: possible ZIP bomb; increase limits only for trusted files.")]
    TooManyEntries { entries: usize, max_entries: usize },
    #[error("[EXDIFF_CTR_006] part '{path}' is too large: {size} bytes (limit: {limit} bytes). Suggestion: possible ZIP bomb; increase limits only for trusted files.")]
    PartTooLarge { path: String, size: u64, limit: u64 },
    #[error("[EXDIFF_CTR_007] total uncompressed size exceeds limit: would exceed {limit} bytes. Suggestion: possible ZIP bomb; increase limits only for trusted files.")]
    TotalTooLarge { limit: u64 },
    #[error("[EXDIFF_CTR_002] failed to read ZIP entry '{path}': {reason}. Suggestion: the file may be corrupt or truncated.")]
    ZipRead { path: String, reason: String },
    #[error("[EXDIFF_CTR_002] file not found in archive: {path}. Suggestion: the file may be corrupt or incomplete.")]
    FileNotFound { path: String },
}

impl ContainerError {
    pub fn code(&self) -> &'static str {
        match self {
            ContainerError::Io(_) => error_codes::CONTAINER_IO,
            ContainerError::Zip(_) => error_codes::CONTAINER_ZIP,
            ContainerError::NotZipContainer => error_codes::CONTAINER_NOT_ZIP,
            ContainerError::NotOpcPackage => error_codes::CONTAINER_NOT_OPC,
            ContainerError::TooManyEntries { .. } => error_codes::CONTAINER_TOO_MANY_ENTRIES,
            ContainerError::PartTooLarge { .. } => error_codes::CONTAINER_PART_TOO_LARGE,
            ContainerError::TotalTooLarge { .. } => error_codes::CONTAINER_TOTAL_TOO_LARGE,
            ContainerError::ZipRead { .. } => error_codes::CONTAINER_ZIP,
            ContainerError::FileNotFound { .. } => error_codes::CONTAINER_ZIP,
        }
    }
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct ZipContainer {
    archive: ZipArchive<Box<dyn ReadSeek>>,
    limits: ContainerLimits,
    total_read: u64,
}

impl ZipContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(reader: R) -> Result<Self, ContainerError> {
        Self::open_from_reader_with_limits(reader, ContainerLimits::default())
    }

    pub fn open_from_reader_with_limits<R: Read + Seek + 'static>(
        reader: R,
        limits: ContainerLimits,
    ) -> Result<Self, ContainerError> {
        let reader: Box<dyn ReadSeek> = Box::new(reader);
        let archive = ZipArchive::new(reader).map_err(|err| match err {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                ContainerError::NotZipContainer
            }
            ZipError::Io(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::InvalidData => {
                    ContainerError::NotZipContainer
                }
                _ => ContainerError::Io(e),
            },
            other => ContainerError::Zip(other.to_string()),
        })?;

        if archive.len() > limits.max_entries {
            return Err(ContainerError::TooManyEntries {
                entries: archive.len(),
                max_entries: limits.max_entries,
            });
        }

        Ok(Self {
            archive,
            limits,
            total_read: 0,
        })
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path(path: impl AsRef<std::path::Path>) -> Result<Self, ContainerError> {
        Self::open_from_path_with_limits(path, ContainerLimits::default())
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path_with_limits(
        path: impl AsRef<std::path::Path>,
        limits: ContainerLimits,
    ) -> Result<Self, ContainerError> {
        let file = std::fs::File::open(path)?;
        Self::open_from_reader_with_limits(file, limits)
    }

    #[cfg(feature = "std-fs")]
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, ContainerError> {
        Self::open_from_path(path)
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, ZipError> {
        let mut file = self.archive.by_name(name)?;
        let mut buf = Vec::with_capacity(usize::try_from(file.size()).unwrap_or(0));
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn file_fingerprint(&mut self, name: &str) -> Result<ZipEntryFingerprint, ZipError> {
        let file = self.archive.by_name(name)?;
        Ok(ZipEntryFingerprint {
            crc32: file.crc32(),
            size: file.size(),
        })
    }

    pub fn file_fingerprint_checked(
        &mut self,
        name: &str,
    ) -> Result<ZipEntryFingerprint, ContainerError> {
        let file = self.archive.by_name(name).map_err(|e| match e {
            ZipError::FileNotFound => ContainerError::FileNotFound {
                path: name.to_string(),
            },
            ZipError::Io(io_err) => ContainerError::ZipRead {
                path: name.to_string(),
                reason: io_err.to_string(),
            },
            other => ContainerError::ZipRead {
                path: name.to_string(),
                reason: other.to_string(),
            },
        })?;

        let size = file.size();
        if size > self.limits.max_part_uncompressed_bytes {
            return Err(ContainerError::PartTooLarge {
                path: name.to_string(),
                size,
                limit: self.limits.max_part_uncompressed_bytes,
            });
        }

        Ok(ZipEntryFingerprint {
            crc32: file.crc32(),
            size,
        })
    }

    pub fn file_fingerprint_optional_checked(
        &mut self,
        name: &str,
    ) -> Result<Option<ZipEntryFingerprint>, ContainerError> {
        match self.file_fingerprint_checked(name) {
            Ok(fp) => Ok(Some(fp)),
            Err(ContainerError::FileNotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn read_file_checked(&mut self, name: &str) -> Result<Vec<u8>, ContainerError> {
        let mut file = self.archive.by_name(name).map_err(|e| match e {
            ZipError::FileNotFound => ContainerError::FileNotFound {
                path: name.to_string(),
            },
            ZipError::Io(io_err) => ContainerError::ZipRead {
                path: name.to_string(),
                reason: io_err.to_string(),
            },
            other => ContainerError::ZipRead {
                path: name.to_string(),
                reason: other.to_string(),
            },
        })?;
        let size = file.size();

        if size > self.limits.max_part_uncompressed_bytes {
            return Err(ContainerError::PartTooLarge {
                path: name.to_string(),
                size,
                limit: self.limits.max_part_uncompressed_bytes,
            });
        }

        let new_total = self.total_read.saturating_add(size);
        if new_total > self.limits.max_total_uncompressed_bytes {
            return Err(ContainerError::TotalTooLarge {
                limit: self.limits.max_total_uncompressed_bytes,
            });
        }

        let mut buf = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
        file.read_to_end(&mut buf)
            .map_err(|e| ContainerError::ZipRead {
                path: name.to_string(),
                reason: e.to_string(),
            })?;

        self.total_read = new_total;
        Ok(buf)
    }

    pub fn read_file_optional(&mut self, name: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
        match self.read_file(name) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(ZipError::FileNotFound) => Ok(None),
            Err(ZipError::Io(e)) => Err(e),
            Err(e) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        }
    }

    pub fn read_file_optional_checked(
        &mut self,
        name: &str,
    ) -> Result<Option<Vec<u8>>, ContainerError> {
        match self.read_file_checked(name) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(ContainerError::FileNotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.archive.file_names()
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn limits(&self) -> &ContainerLimits {
        &self.limits
    }
}

pub struct OpcContainer {
    inner: ZipContainer,
}

impl OpcContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(
        reader: R,
    ) -> Result<OpcContainer, ContainerError> {
        Self::open_from_reader_with_limits(reader, ContainerLimits::default())
    }

    pub fn open_from_reader_with_limits<R: Read + Seek + 'static>(
        reader: R,
        limits: ContainerLimits,
    ) -> Result<OpcContainer, ContainerError> {
        let mut inner = ZipContainer::open_from_reader_with_limits(reader, limits)?;

        match inner.archive.by_name("[Content_Types].xml") {
            Ok(file) => {
                let size = file.size();
                if size > inner.limits.max_part_uncompressed_bytes {
                    return Err(ContainerError::PartTooLarge {
                        path: "[Content_Types].xml".to_string(),
                        size,
                        limit: inner.limits.max_part_uncompressed_bytes,
                    });
                }
            }
            Err(ZipError::FileNotFound) => return Err(ContainerError::NotOpcPackage),
            Err(ZipError::Io(e)) => return Err(ContainerError::Io(e)),
            Err(other) => return Err(ContainerError::Zip(other.to_string())),
        }

        Ok(Self { inner })
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path(
        path: impl AsRef<std::path::Path>,
    ) -> Result<OpcContainer, ContainerError> {
        Self::open_from_path_with_limits(path, ContainerLimits::default())
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path_with_limits(
        path: impl AsRef<std::path::Path>,
        limits: ContainerLimits,
    ) -> Result<OpcContainer, ContainerError> {
        let file = std::fs::File::open(path)?;
        Self::open_from_reader_with_limits(file, limits)
    }

    #[cfg(feature = "std-fs")]
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<OpcContainer, ContainerError> {
        Self::open_from_path(path)
    }

    pub fn read_file(&mut self, name: &str) -> Result<Vec<u8>, ZipError> {
        self.inner.read_file(name)
    }

    pub fn file_fingerprint(&mut self, name: &str) -> Result<ZipEntryFingerprint, ZipError> {
        self.inner.file_fingerprint(name)
    }

    pub fn file_fingerprint_checked(
        &mut self,
        name: &str,
    ) -> Result<ZipEntryFingerprint, ContainerError> {
        self.inner.file_fingerprint_checked(name)
    }

    pub fn file_fingerprint_optional_checked(
        &mut self,
        name: &str,
    ) -> Result<Option<ZipEntryFingerprint>, ContainerError> {
        self.inner.file_fingerprint_optional_checked(name)
    }

    pub fn read_file_checked(&mut self, name: &str) -> Result<Vec<u8>, ContainerError> {
        self.inner.read_file_checked(name)
    }

    pub fn read_file_optional(&mut self, name: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
        self.inner.read_file_optional(name)
    }

    pub fn read_file_optional_checked(
        &mut self,
        name: &str,
    ) -> Result<Option<Vec<u8>>, ContainerError> {
        self.inner.read_file_optional_checked(name)
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.inner.file_names()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn limits(&self) -> &ContainerLimits {
        self.inner.limits()
    }
}

#[cfg(test)]
mod tests {
    use super::ZipContainer;
    use std::io::{Cursor, Write};
    use zip::write::FileOptions;
    use zip::CompressionMethod;
    use zip::ZipWriter;

    fn make_zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut writer = ZipWriter::new(cursor);
            let options = FileOptions::default().compression_method(CompressionMethod::Stored);
            for (name, contents) in entries {
                writer.start_file(*name, options).expect("start zip entry");
                writer
                    .write_all(contents.as_bytes())
                    .expect("write zip entry");
            }
            writer.finish().expect("finish zip");
        }
        buf
    }

    #[test]
    fn zip_container_opens_non_opc_zip() {
        let bytes = make_zip(&[("hello.txt", "world")]);
        let cursor = Cursor::new(bytes);
        let result = ZipContainer::open_from_reader(cursor);
        assert!(result.is_ok(), "ZipContainer should open non-OPC ZIPs");
    }
}
