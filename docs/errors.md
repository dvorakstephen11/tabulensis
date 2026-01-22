# Tabulensis Error Codes

This document lists all error codes returned by excel_diff, their meanings, likely causes, and suggested next steps.

## Package Errors (EXDIFF_PKG_xxx)

| Code | Meaning | Likely Cause | Next Step |
|------|---------|--------------|-----------|
| `EXDIFF_PKG_001` | Not a ZIP file | File is not a valid ZIP archive | Verify the file is a valid .xlsx file |
| `EXDIFF_PKG_002` | Not an OPC package | ZIP archive lacks `[Content_Types].xml` | File may be a ZIP but not an Office document |
| `EXDIFF_PKG_003` | Missing required part | Required XML part not found in package | The Excel file may be corrupt or incomplete |
| `EXDIFF_PKG_004` | Invalid XML | XML parsing failed in a package part | Re-save the file in Excel or verify it's valid .xlsx |
| `EXDIFF_PKG_005` | Part too large | ZIP entry exceeds uncompressed size limit | Potential ZIP bomb; increase limits if file is legitimate |
| `EXDIFF_PKG_006` | Too many ZIP entries | Archive has more entries than allowed | Potential ZIP bomb; increase limits if file is legitimate |
| `EXDIFF_PKG_007` | Total size too large | Total uncompressed size exceeds limit | Potential ZIP bomb; increase limits if file is legitimate |
| `EXDIFF_PKG_008` | ZIP read failure | Failed to read a ZIP entry | File may be corrupt or truncated |
| `EXDIFF_PKG_009` | Unsupported format | File format not supported | Use a standard .xlsx file saved by Excel |

## Grid Parse Errors (EXDIFF_GRID_xxx)

| Code | Meaning | Likely Cause | Next Step |
|------|---------|--------------|-----------|
| `EXDIFF_GRID_001` | XML parse error | Invalid XML in worksheet data | Re-save the file in Excel |
| `EXDIFF_GRID_002` | Invalid cell address | Cell address in unexpected format | File may be corrupt |
| `EXDIFF_GRID_003` | Shared string out of bounds | Reference to non-existent shared string | File may be corrupt |

## Container Errors (EXDIFF_CTR_xxx)

| Code | Meaning | Likely Cause | Next Step |
|------|---------|--------------|-----------|
| `EXDIFF_CTR_001` | I/O error | File system or read error | Check file permissions and path |
| `EXDIFF_CTR_002` | ZIP error | General ZIP format error | File may be corrupt |
| `EXDIFF_CTR_003` | Not a ZIP container | Input is not a ZIP file | Verify file format |
| `EXDIFF_CTR_004` | Not an OPC package | Missing `[Content_Types].xml` | Not an Office document |
| `EXDIFF_CTR_005` | Too many entries | Archive entry count exceeds limit | Potential ZIP bomb |
| `EXDIFF_CTR_006` | Part too large | Single part exceeds size limit | Potential ZIP bomb |
| `EXDIFF_CTR_007` | Total too large | Cumulative size exceeds limit | Potential ZIP bomb |

## DataMashup Errors (EXDIFF_DM_xxx)

| Code | Meaning | Likely Cause | Next Step |
|------|---------|--------------|-----------|
| `EXDIFF_DM_001` | Base64 invalid | Invalid base64 encoding in DataMashup | File may be corrupt |
| `EXDIFF_DM_002` | Unsupported version | DataMashup version not supported | Update excel_diff or re-save file |
| `EXDIFF_DM_003` | Framing invalid | DataMashup structure malformed | File may be corrupt |
| `EXDIFF_DM_004` | XML error | Invalid XML in DataMashup | Re-save the file in Excel |
| `EXDIFF_DM_005` | Inner part too large | Embedded package part too large | Potential nested ZIP bomb |
| `EXDIFF_DM_006` | Invalid header | DataMashup header malformed | File may be corrupt |
| `EXDIFF_DM_007` | Inner package too many entries | Embedded package contains too many entries | Potential nested ZIP bomb |
| `EXDIFF_DM_008` | Inner package total too large | Embedded package total uncompressed size too large | Potential nested ZIP bomb |
| `EXDIFF_DM_009` | Permission bindings unverifiable | DPAPI bindings present but cannot be validated or plaintext is malformed | Permissions defaulted to Excel fallback; re-save on the same Windows user or expect defaults |

Note: `EXDIFF_DM_009` is surfaced as a warning and marks the diff `complete=false` rather than aborting the parse.

## Diff Errors (EXDIFF_DIFF_xxx)

| Code | Meaning | Likely Cause | Next Step |
|------|---------|--------------|-----------|
| `EXDIFF_DIFF_001` | Limits exceeded | Sheet dimensions exceed alignment limits | Use larger limits or positional-only diff |
| `EXDIFF_DIFF_002` | Sink error | Error writing diff output | Check output destination |
| `EXDIFF_DIFF_003` | Sheet not found | Requested sheet not in workbook | Check sheet name spelling |
| `EXDIFF_DIFF_004` | Internal error | Unexpected internal condition | Report a bug |

## ZIP Bomb Protection

Tabulensis includes protection against ZIP bombs (malicious archives that expand to very large sizes). The default limits are:

- **Max entries:** 10,000
- **Max part size:** 100 MB (uncompressed)
- **Max total size:** 500 MB (uncompressed)

If you need to process legitimate large files, you can adjust these limits using `ContainerLimits`:

```rust
use excel_diff::{ContainerLimits, OpcContainer};

let limits = ContainerLimits {
    max_entries: 50_000,
    max_part_uncompressed_bytes: 500 * 1024 * 1024,
    max_total_uncompressed_bytes: 2 * 1024 * 1024 * 1024,
};

let container = OpcContainer::open_from_reader_with_limits(reader, limits)?;
```

## Error Context

All errors include contextual information to help diagnose issues:

- **File path:** When available, errors include the source file path
- **Part path:** For XML/part errors, the specific part within the package is identified
- **Line/column:** XML parse errors include position information when available
- **Suggested action:** Error messages include hints for resolution

