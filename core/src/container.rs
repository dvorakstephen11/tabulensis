//! OPC (Open Packaging Conventions) container handling.
//!
//! Provides abstraction over ZIP-based Office Open XML packages, validating
//! that required structural elements like `[Content_Types].xml` are present.

use std::io::{Read, Seek};
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

use crate::error_codes;

#[derive(Debug, Clone, Copy)]
pub struct ContainerLimits {
    pub max_entries: usize,
    pub max_part_uncompressed_bytes: u64,
    pub max_total_uncompressed_bytes: u64,
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

pub struct OpcContainer {
    pub(crate) archive: ZipArchive<Box<dyn ReadSeek>>,
    limits: ContainerLimits,
    total_read: u64,
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

        let mut container = OpcContainer {
            archive,
            limits,
            total_read: 0,
        };

        match container.archive.by_name("[Content_Types].xml") {
            Ok(file) => {
                let size = file.size();
                if size > container.limits.max_part_uncompressed_bytes {
                    return Err(ContainerError::PartTooLarge {
                        path: "[Content_Types].xml".to_string(),
                        size,
                        limit: container.limits.max_part_uncompressed_bytes,
                    });
                }
            }
            Err(ZipError::FileNotFound) => return Err(ContainerError::NotOpcPackage),
            Err(ZipError::Io(e)) => return Err(ContainerError::Io(e)),
            Err(other) => return Err(ContainerError::Zip(other.to_string())),
        }

        Ok(container)
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
        let mut file = self.archive.by_name(name)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn read_file_checked(&mut self, name: &str) -> Result<Vec<u8>, ContainerError> {
        let size = {
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
            file.size()
        };

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

        let mut file = self.archive.by_name(name).map_err(|e| ContainerError::ZipRead {
            path: name.to_string(),
            reason: e.to_string(),
        })?;

        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| ContainerError::ZipRead {
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

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
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
