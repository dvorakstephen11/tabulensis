use excel_diff::{
    DpapiDecryptError, DpapiDecryptor, PermissionBindingsStatus, Permissions, RawDataMashup,
    build_data_mashup_with_decryptor,
};
use sha2::{Digest, Sha256};
use std::io::Write;
use zip::CompressionMethod;
use zip::write::FileOptions;
use zip::ZipWriter;

struct StaticDecryptor {
    result: Result<Vec<u8>, DpapiDecryptError>,
}

impl StaticDecryptor {
    fn new(result: Result<Vec<u8>, DpapiDecryptError>) -> Self {
        Self { result }
    }
}

impl DpapiDecryptor for StaticDecryptor {
    fn decrypt(&self, _blob: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, DpapiDecryptError> {
        self.result.clone()
    }
}

fn make_raw_datamashup(permissions_xml: &[u8], permission_bindings: Vec<u8>) -> RawDataMashup {
    RawDataMashup {
        version: 0,
        package_parts: minimal_package_parts(),
        permissions: permissions_xml.to_vec(),
        metadata: Vec::new(),
        permission_bindings,
    }
}

fn minimal_package_parts() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    writer
        .start_file("Config/Package.xml", options)
        .expect("start Config/Package.xml");
    writer
        .write_all(b"<Package/>")
        .expect("write Config/Package.xml");

    writer
        .start_file("Formulas/Section1.m", options)
        .expect("start Formulas/Section1.m");
    writer
        .write_all(b"section Section1;\nshared Foo = 1;")
        .expect("write Formulas/Section1.m");

    let cursor = writer.finish().expect("finish zip");
    cursor.into_inner()
}

fn permissions_firewall_off_xml() -> Vec<u8> {
    br#"
        <PermissionList>
            <FirewallEnabled>false</FirewallEnabled>
        </PermissionList>
    "#
    .to_vec()
}

#[test]
fn null_byte_sentinel_preserves_permissions() {
    let raw = make_raw_datamashup(&permissions_firewall_off_xml(), vec![0x00]);
    let decryptor = StaticDecryptor::new(Err(DpapiDecryptError::Unavailable));

    let dm =
        build_data_mashup_with_decryptor(&raw, &decryptor).expect("DataMashup should build");

    assert_eq!(
        dm.permission_bindings_status,
        PermissionBindingsStatus::Disabled
    );
    assert!(!dm.permissions.firewall_enabled);
}

#[test]
fn dpapi_blob_unverifiable_defaults_permissions() {
    let raw = make_raw_datamashup(&permissions_firewall_off_xml(), vec![0x01, 0x02, 0x03]);
    let decryptor = StaticDecryptor::new(Err(DpapiDecryptError::Unavailable));

    let dm =
        build_data_mashup_with_decryptor(&raw, &decryptor).expect("DataMashup should build");

    assert_eq!(
        dm.permission_bindings_status,
        PermissionBindingsStatus::Unverifiable
    );
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn malformed_decrypted_plaintext_defaults_permissions() {
    let raw = make_raw_datamashup(&permissions_firewall_off_xml(), vec![0x01, 0x02, 0x03]);
    let decryptor = StaticDecryptor::new(Ok(vec![0x01, 0x02, 0x03]));

    let dm =
        build_data_mashup_with_decryptor(&raw, &decryptor).expect("DataMashup should build");

    assert_eq!(
        dm.permission_bindings_status,
        PermissionBindingsStatus::InvalidOrTampered
    );
    assert_eq!(dm.permissions, Permissions::default());
}

#[test]
fn verified_hashes_preserve_permissions() {
    let permissions_xml = permissions_firewall_off_xml();
    let raw = make_raw_datamashup(&permissions_xml, vec![0x10, 0x20, 0x30]);

    let package_hash = sha256(&raw.package_parts);
    let permissions_hash = sha256(&raw.permissions);
    let plaintext = build_plaintext(package_hash, permissions_hash);
    let decryptor = StaticDecryptor::new(Ok(plaintext));

    let dm =
        build_data_mashup_with_decryptor(&raw, &decryptor).expect("DataMashup should build");

    assert_eq!(
        dm.permission_bindings_status,
        PermissionBindingsStatus::Verified
    );
    assert!(!dm.permissions.firewall_enabled);
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn build_plaintext(package_hash: [u8; 32], permissions_hash: [u8; 32]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(package_hash.len() as u32).to_le_bytes());
    buf.extend_from_slice(&package_hash);
    buf.extend_from_slice(&(permissions_hash.len() as u32).to_le_bytes());
    buf.extend_from_slice(&permissions_hash);
    buf
}
