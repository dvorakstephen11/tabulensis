#![cfg(all(
    feature = "excel-open-xml",
    feature = "model-diff",
    feature = "perf-metrics"
))]

use excel_diff::datamodel_schema_parse_counts;
use excel_diff::perf::{DiffMetrics, Phase};
use std::fmt::Write as _;

fn iterations_for_payload(payload_len: usize) -> usize {
    let target_bytes: u64 = 50 * 1024 * 1024;
    let size = payload_len.max(1) as u64;
    let mut iterations = (target_bytes / size).max(1) as usize;
    if iterations > 500 {
        iterations = 500;
    }
    iterations
}

fn make_schema_json(
    tables: usize,
    cols_per_table: usize,
    measures_per_table: usize,
    relationships: usize,
) -> String {
    let mut out = String::new();
    out.push_str(r#"{"model":{"tables":["#);

    for t in 0..tables {
        if t > 0 {
            out.push(',');
        }
        write!(&mut out, r#"{{"name":"T{t}","columns":["#).expect("write should succeed");

        for c in 0..cols_per_table {
            if c > 0 {
                out.push(',');
            }
            write!(
                &mut out,
                r#"{{"name":"C{c}","dataType":"decimal","isHidden":false,"formatString":"0.00","sortByColumn":"C0","summarizeBy":"sum","expression":"[C{c}] * 2"}}"#
            )
            .expect("write should succeed");
        }

        out.push_str(r#"],"measures":["#);

        for m in 0..measures_per_table {
            if m > 0 {
                out.push(',');
            }
            write!(
                &mut out,
                r#"{{"name":"M{m}","expression":"SUM(T{t}[C0])"}}"#
            )
            .expect("write should succeed");
        }

        out.push_str("]}");
    }

    out.push_str(r#"],"relationships":["#);

    if tables > 0 {
        for r in 0..relationships {
            if r > 0 {
                out.push(',');
            }
            let from_t = r % tables;
            let to_t = (r + 1) % tables;
            write!(
                &mut out,
                r#"{{"fromTable":"T{from_t}","fromColumn":"C0","toTable":"T{to_t}","toColumn":"C0","crossFilteringBehavior":"oneDirection","cardinality":"ManyToOne","isActive":true,"name":"R{r}"}}"#
            )
            .expect("write should succeed");
        }
    }

    out.push_str(r#"]}}"#);
    out
}

#[test]
#[ignore = "Perf test: run with `cargo test -p excel_diff --release --features \"perf-metrics model-diff\" --test e2e_perf_datamodel_schema_parse -- --ignored --nocapture --test-threads=1`. Note: `custom-json-schema` is default-on; to force the legacy Value parser, use `--no-default-features --features \"excel-open-xml model-diff base64-crate perf-metrics\"`."]
fn e2e_perf_datamodel_schema_parse() {
    let tables = 200usize;
    let cols_per_table = 30usize;
    let measures_per_table = 10usize;
    let relationships = 1_000usize;

    let schema = make_schema_json(tables, cols_per_table, measures_per_table, relationships);
    let iterations = iterations_for_payload(schema.len());

    let mut metrics = DiffMetrics::default();
    metrics.start_phase(Phase::Total);
    metrics.start_phase(Phase::Parse);
    for _ in 0..iterations {
        let counts =
            datamodel_schema_parse_counts(schema.as_bytes()).expect("schema parse should succeed");
        std::hint::black_box(counts);
    }
    metrics.end_phase(Phase::Parse);
    metrics.end_phase(Phase::Total);

    println!(
        "PERF_METRIC datamodel_schema_parse iterations={} schema_bytes={} parse_time_ms={} total_time_ms={} peak_memory_bytes={} tables={} cols_per_table={} measures_per_table={} relationships={}",
        iterations,
        schema.len(),
        metrics.parse_time_ms,
        metrics.total_time_ms,
        metrics.peak_memory_bytes,
        tables,
        cols_per_table,
        measures_per_table,
        relationships
    );
}
