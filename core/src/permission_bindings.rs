use crate::datamashup::Permissions;
use crate::datamashup_framing::RawDataMashup;
use crate::error_codes;
use sha2::{Digest, Sha256};

const DPAPI_ENTROPY: &[u8] = b"DataExplorer Package Components";
const SHA256_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionBindingsKind {
    Missing,
    NullByteSentinel,
    DpapiEncryptedBlob,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionBindingsStatus {
    Missing,
    Disabled,
    Verified,
    InvalidOrTampered,
    Unverifiable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpapiDecryptError {
    Unavailable,
    Failed,
}

pub trait DpapiDecryptor {
    fn decrypt(&self, blob: &[u8], entropy: &[u8]) -> Result<Vec<u8>, DpapiDecryptError>;
}

#[allow(dead_code)]
pub struct NoDpapiDecryptor;

impl DpapiDecryptor for NoDpapiDecryptor {
    fn decrypt(&self, _blob: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, DpapiDecryptError> {
        Err(DpapiDecryptError::Unavailable)
    }
}

#[cfg(all(windows, feature = "dpapi"))]
pub struct WindowsDpapiDecryptor;

#[cfg(all(windows, feature = "dpapi"))]
impl DpapiDecryptor for WindowsDpapiDecryptor {
    fn decrypt(&self, blob: &[u8], entropy: &[u8]) -> Result<Vec<u8>, DpapiDecryptError> {
        use windows_sys::Win32::Foundation::LocalFree;
        use windows_sys::Win32::Security::Cryptography::CRYPTPROTECT_UI_FORBIDDEN;
        use windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB;
        use windows_sys::Win32::Security::Cryptography::CryptUnprotectData;

        if blob.is_empty() {
            return Err(DpapiDecryptError::Failed);
        }

        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: blob.len() as u32,
            pbData: blob.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let entropy_blob = CRYPT_INTEGER_BLOB {
            cbData: entropy.len() as u32,
            pbData: entropy.as_ptr() as *mut u8,
        };
        let entropy_ptr = if entropy.is_empty() {
            std::ptr::null()
        } else {
            &entropy_blob as *const CRYPT_INTEGER_BLOB
        };

        let ok = unsafe {
            CryptUnprotectData(
                &in_blob as *const CRYPT_INTEGER_BLOB,
                std::ptr::null_mut(),
                entropy_ptr,
                std::ptr::null(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob as *mut CRYPT_INTEGER_BLOB,
            )
        };

        if ok == 0 || out_blob.pbData.is_null() {
            return Err(DpapiDecryptError::Failed);
        }

        let data = unsafe {
            let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
            let out = slice.to_vec();
            LocalFree(out_blob.pbData as *mut core::ffi::c_void);
            out
        };

        Ok(data)
    }
}

pub(crate) fn default_dpapi_decryptor() -> &'static dyn DpapiDecryptor {
    #[cfg(all(windows, feature = "dpapi"))]
    {
        static DECRYPTOR: WindowsDpapiDecryptor = WindowsDpapiDecryptor;
        &DECRYPTOR
    }
    #[cfg(not(all(windows, feature = "dpapi")))]
    {
        static DECRYPTOR: NoDpapiDecryptor = NoDpapiDecryptor;
        &DECRYPTOR
    }
}

pub fn classify_permission_bindings(raw: &[u8]) -> PermissionBindingsKind {
    if raw.is_empty() {
        return PermissionBindingsKind::Missing;
    }
    if raw.len() == 1 && raw[0] == 0x00 {
        return PermissionBindingsKind::NullByteSentinel;
    }
    PermissionBindingsKind::DpapiEncryptedBlob
}

pub fn validate_permission_bindings(
    raw: &RawDataMashup,
    decryptor: &dyn DpapiDecryptor,
) -> PermissionBindingsStatus {
    match classify_permission_bindings(&raw.permission_bindings) {
        PermissionBindingsKind::Missing => PermissionBindingsStatus::Missing,
        PermissionBindingsKind::NullByteSentinel => PermissionBindingsStatus::Disabled,
        PermissionBindingsKind::DpapiEncryptedBlob => {
            let plaintext = match decryptor.decrypt(&raw.permission_bindings, DPAPI_ENTROPY) {
                Ok(bytes) => bytes,
                Err(_) => return PermissionBindingsStatus::Unverifiable,
            };

            let Some((package_hash, permissions_hash)) = parse_plaintext_hashes(&plaintext) else {
                return PermissionBindingsStatus::InvalidOrTampered;
            };

            let expected_package_hash = sha256(&raw.package_parts);
            let expected_permissions_hash = sha256(&raw.permissions);

            if package_hash == expected_package_hash && permissions_hash == expected_permissions_hash {
                PermissionBindingsStatus::Verified
            } else {
                PermissionBindingsStatus::InvalidOrTampered
            }
        }
    }
}

pub fn effective_permissions(
    parsed_permissions: Permissions,
    status: PermissionBindingsStatus,
) -> Permissions {
    match status {
        PermissionBindingsStatus::Missing
        | PermissionBindingsStatus::Disabled
        | PermissionBindingsStatus::Verified => parsed_permissions,
        PermissionBindingsStatus::InvalidOrTampered | PermissionBindingsStatus::Unverifiable => {
            Permissions::default()
        }
    }
}

pub fn permission_bindings_warning(status: PermissionBindingsStatus) -> Option<String> {
    let default_desc = "FirewallEnabled=true and WorkbookGroupType=null";
    match status {
        PermissionBindingsStatus::Unverifiable => Some(format!(
            "[{}] Permission bindings are DPAPI-encrypted and could not be validated on this platform; permissions have been defaulted to {} to match Excel fallback behavior.",
            error_codes::DM_PERMISSION_BINDINGS_UNVERIFIED,
            default_desc
        )),
        PermissionBindingsStatus::InvalidOrTampered => Some(format!(
            "[{}] Permission bindings failed validation (hash mismatch or malformed plaintext); permissions have been defaulted to {} to match Excel fallback behavior.",
            error_codes::DM_PERMISSION_BINDINGS_UNVERIFIED,
            default_desc
        )),
        _ => None,
    }
}

fn parse_plaintext_hashes(plaintext: &[u8]) -> Option<([u8; SHA256_LEN], [u8; SHA256_LEN])> {
    let (len_a, rest) = read_length_prefix(plaintext)?;
    if len_a != SHA256_LEN {
        return None;
    }
    let (hash_a, rest) = rest.split_at(len_a);
    let (len_b, rest) = read_length_prefix(rest)?;
    if len_b != SHA256_LEN || rest.len() != len_b {
        return None;
    }
    let hash_b = rest;

    let hash_a: [u8; SHA256_LEN] = hash_a.try_into().ok()?;
    let hash_b: [u8; SHA256_LEN] = hash_b.try_into().ok()?;
    Some((hash_a, hash_b))
}

fn read_length_prefix(bytes: &[u8]) -> Option<(usize, &[u8])> {
    if bytes.len() < 4 {
        return None;
    }
    let len = u32::from_le_bytes(bytes[0..4].try_into().ok()?) as usize;
    let rest = bytes.get(4..)?;
    if rest.len() < len {
        return None;
    }
    Some((len, rest))
}

fn sha256(bytes: &[u8]) -> [u8; SHA256_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
