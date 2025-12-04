## Overview

To ensure the diff engine acts as a reliable source of truth, it must not discard M queries simply because their metadata is missing. The system should be robust to metadata inconsistency by synthesizing default metadata for any `Section1` member that lacks an explicit entry in the Metadata XML.

## Fixes Required

### Fix 1: Synthesize Default Metadata for Orphaned Members
- **Addresses Finding**: Queries Without Metadata are Dropped
- **Changes**: 
    - Modify `core/src/datamashup.rs` in `build_queries`.
    - Instead of `continue` when metadata is missing, construct a default `QueryMetadata` instance.
    - Default values: `load_to_sheet: false`, `load_to_model: false`, `is_connection_only: true`, `group_path: None`.
    - Construct `item_path` as `Section1/<MemberName>`.
- **Tests**: 
    - Add a new test case `member_without_metadata_is_preserved` in `core/tests/m5_query_domain_tests.rs`.
    - Create a fixture (or use an existing template via `mashup_inject`) where a query exists in M but not in Metadata.

## Constraints

- The `Query` struct requires a `QueryMetadata` field, so synthesis is the only option to preserve the `Vec<Query>` signature.
- Ensure synthesized metadata accurately reflects "connection only" status (safest assumption).

## Expected Outcome

After remediation, `build_queries` will return a `Query` object for every shared member in `Section1.m`, ensuring 100% code visibility regardless of metadata integrity. The `metadata_orphan_entries` test (orphaned metadata) will remain passing, and the new test (orphaned code) will also pass.