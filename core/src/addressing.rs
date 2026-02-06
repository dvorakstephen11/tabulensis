//! Excel cell addressing utilities.
//!
//! Provides conversion between A1-style cell addresses (e.g., "B2", "AA10") and
//! zero-based (row, column) index pairs.

use std::fmt;

/// Error returned when parsing an invalid A1-style cell address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressParseError {
    pub input: String,
}

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cell address: '{}'", self.input)
    }
}

impl std::error::Error for AddressParseError {}

/// Convert zero-based (row, col) indices to an Excel A1 address string.
pub fn index_to_address(row: u32, col: u32) -> String {
    let mut col_index = col;
    let mut col_label = String::new();

    loop {
        let rem = (col_index % 26) as u8;
        col_label.push((b'A' + rem) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }

    col_label.chars().rev().collect::<String>() + &(row + 1).to_string()
}

/// Parse an A1 address into zero-based (row, col) indices.
/// Returns `None` for malformed addresses.
pub fn address_to_index(a1: &str) -> Option<(u32, u32)> {
    let bytes = a1.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut i: usize = 0;
    let mut col: u32 = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_alphabetic() {
            break;
        }
        let upper = b.to_ascii_uppercase();
        col = col
            .checked_mul(26)?
            .checked_add((upper - b'A' + 1) as u32)?;
        i += 1;
    }

    if i == 0 || i >= bytes.len() || col == 0 {
        return None;
    }

    let mut row: u32 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            return None;
        }
        row = row.checked_mul(10)?.checked_add((b - b'0') as u32)?;
        i += 1;
    }

    if row == 0 {
        return None;
    }

    Some((row - 1, col - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_address_examples() {
        assert_eq!(index_to_address(0, 0), "A1");
        assert_eq!(index_to_address(0, 25), "Z1");
        assert_eq!(index_to_address(0, 26), "AA1");
        assert_eq!(index_to_address(0, 27), "AB1");
        assert_eq!(index_to_address(0, 51), "AZ1");
        assert_eq!(index_to_address(0, 52), "BA1");
    }

    #[test]
    fn round_trip_addresses() {
        let addresses = [
            "A1", "B2", "Z10", "AA1", "AA10", "AB7", "AZ5", "BA1", "ZZ10", "AAA1",
        ];
        for addr in addresses {
            let (r, c) = address_to_index(addr).expect("address should parse");
            assert_eq!(index_to_address(r, c), addr);
        }
    }

    #[test]
    fn invalid_addresses_rejected() {
        let invalid = ["", "1A", "A0", "A", "AA0", "A-1", "A1A"];
        for addr in invalid {
            assert!(address_to_index(addr).is_none(), "{addr} should be invalid");
        }
    }
}
