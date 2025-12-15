//! OPC (Open Packaging Conventions) container handling.
//!
//! Provides abstraction over ZIP-based Office Open XML packages, validating
//! that required structural elements like `[Content_Types].xml` are present.

use std::io::{Read, Seek};
use thiserror::Error;
use zip::ZipArchive;
use zip::result::ZipError;

/// Errors that can occur when opening or reading an OPC container.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContainerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(String),
    #[error("not a ZIP container")]
    NotZipContainer,
    #[error("not an OPC package (missing [Content_Types].xml)")]
    NotOpcPackage,
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub struct OpcContainer {
    pub(crate) archive: ZipArchive<Box<dyn ReadSeek>>,
}

impl OpcContainer {
    pub fn open_from_reader<R: Read + Seek + 'static>(
        reader: R,
    ) -> Result<OpcContainer, ContainerError> {
        let reader: Box<dyn ReadSeek> = Box::new(reader);
        let archive = ZipArchive::new(reader).map_err(|err| match err {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                ContainerError::NotZipContainer
            }
            ZipError::Io(e) => ContainerError::Io(e),
            other => ContainerError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                other.to_string(),
            )),
        })?;

        let mut container = OpcContainer { archive };
        if container.read_file("[Content_Types].xml").is_err() {
            return Err(ContainerError::NotOpcPackage);
        }

        Ok(container)
    }

    #[cfg(feature = "std-fs")]
    pub fn open_from_path(path: impl AsRef<std::path::Path>) -> Result<OpcContainer, ContainerError> {
        let file = std::fs::File::open(path)?;
        Self::open_from_reader(file)
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

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.archive.file_names()
    }

    pub fn len(&self) -> usize {
        self.archive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
