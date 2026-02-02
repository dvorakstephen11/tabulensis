## Improvements I’d make to the experiment plan

A lot of your plan is already solid (A/B/C variants, tiered workloads, criterion for microbenches, explicit dependency + binary-size checks).  
Here are the main “make this easier to trust / easier to execute” improvements I’d add:

1. **Make dependency-footprint measurements “real” by ensuring the crate actually leaves the graph.**
   Your plan includes `cargo tree -i <crate>` and binary-size deltas , but for many candidates you won’t see meaningful changes unless you:

   * make the third-party dependency `optional = true`, and
   * gate it behind a “baseline feature” (enabled by default), and
   * ensure your **custom** configuration *does not* enable that baseline feature.
     Otherwise Cargo will still build/track the dependency even if your code path no longer references it.

2. **Add a “profiling sanity check” gate before implementing a rewrite.**
   You note “hot-path parsing/serialization” , but I’d formalize: *before* rewriting, capture at least one flamegraph/perf profile of the representative workload, and require that the target crate/function shows up meaningfully. This prevents investing in a rewrite whose ceiling is <1–2%.

3. **Prefer “parity tests” over runtime fallback for C.**
   You already call out C as “for differential testing only” . I’d go further:

   * keep C as *tests-only* (or at least `cfg(test)` / dev-only code),
   * and implement “side-by-side” parity tests that exercise both implementations on the same inputs (you already intend this) .
     Runtime fallback paths are easy to accidentally ship and tend to complicate code/branching in hot paths.

4. **Add one more metric: compile time / incremental build time.**
   Since part of the motivation is “simpler and smaller,” tracking “time to build `cli` + `desktop_wx`” can be valuable alongside binary size. It’s often where removing dependencies pays off the most.

5. **Automate the A/B/C runner (so you don’t accidentally compare apples to oranges).**
   Your plan requires multiple iterations, warm-up discard, median + p95, and recording environment details . I’d strongly recommend a single command (script or `xtask`) that:

   * builds each variant with fixed flags/profiles,
   * runs the same workload set,
   * stores JSON/CSV results with the exact feature set and commit hash embedded.

---

## Candidate to rewrite first: `base64` for DataMashup decoding

This is the cleanest low-risk candidate you identified: `decode_datamashup_base64` currently does a full-copy `String` allocation via `split_whitespace().collect()` before decoding, which can be expensive for multi‑MB DataMashup payloads. 
You also already scoped a focused replacement: a strict, whitespace-ignoring decoder that preserves the existing `DataMashupError::Base64Invalid` mapping. 

Below is a **drop-in custom replacement** that:

* decodes *standard* Base64 (RFC 4648 alphabet, `=` padding),
* ignores ASCII whitespace while decoding (space, tab, CR, LF, VT, FF),
* enforces **canonical padding** and rejects non-canonical trailing bits (matching `base64::engine::general_purpose::STANDARD`’s strict behavior),
* and maps errors to your existing error code path.

It is also structured so you can:

* run **baseline** using the `base64` crate,
* run **custom** using your decoder,
* run **parity tests** with both enabled.

---

## Code: custom base64 decoder + wiring

### 1) `core/Cargo.toml` changes

Make `base64` optional so you can actually measure dependency footprint deltas (and keep it enabled by default for baseline behavior). Your current `core/Cargo.toml` includes `base64 = "0.22"` unconditionally. 

Replace that part with:

```toml
# core/Cargo.toml

[features]
# add base64-crate by default so normal builds keep working
default = ["excel-open-xml", "std-fs", "vba", "dpapi", "base64-crate"]

# baseline implementation (third-party crate)
base64-crate = ["dep:base64"]

# custom implementation (this experiment)
custom-base64 = []

# ... existing features unchanged ...
excel-open-xml = []
vba = ["dep:ovba"]
std-fs = []
perf-metrics = []
dev-apis = []
model-diff = []
legacy-api = []
parallel = ["dep:rayon"]
dpapi = []

[dependencies]
# was: base64 = "0.22"
base64 = { version = "0.22", optional = true }

# ... rest unchanged ...
```

### 2) Update dependents that use `default-features = false`

Because `ui_payload` and `desktop_backend` depend on `excel_diff` with `default-features = false`, they must explicitly enable *some* base64 provider now.

* `ui_payload/Cargo.toml` currently: 

```toml
excel_diff = { path = "./core", default-features = false, features = ["excel-open-xml"] }
```

Change to:

```toml
excel_diff = { path = "./core", default-features = false, features = ["excel-open-xml", "base64-crate"] }
```

* `desktop/backend/Cargo.toml` currently: 

```toml
excel_diff = { path = "././core", default-features = false, features = ["excel-open-xml", "vba"] }
```

Change to:

```toml
excel_diff = { path = "././core", default-features = false, features = ["excel-open-xml", "vba", "base64-crate"] }
```

(If any other workspace crates also use `default-features = false` on `excel_diff`, they’ll need the same update.)

### 3) Add the custom decoder module

Create a new file:

#### `core/src/custom_base64.rs`

```rust
//! Minimal, strict Base64 decoder for Tabulensis.
//!
//! Intended for hot paths where the input may contain ASCII whitespace (line breaks, indentation).
//!
//! Behavior matches a strict RFC 4648 "standard" Base64 decoder with canonical padding:
//! - Alphabet: A-Z a-z 0-9 + /
//! - Padding: '=' (canonical only; rejects '=' in the first 2 positions of a quad)
//! - Rejects non-canonical trailing bits when padding is present
//! - Ignores ASCII whitespace: space, tab, CR, LF, VT, FF

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodeError {
    InvalidByte(u8),
    InvalidLength,
    InvalidPadding,
    InvalidTrailingBits,
    TrailingData,
}

#[inline]
fn is_ascii_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
}

/// Returns the Base64 sextet value (0..=63), or 64 for '=', or None for invalid bytes.
#[inline]
fn decode_sextet(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        b'=' => Some(64),
        _ => None,
    }
}

/// Decode Base64 while ignoring ASCII whitespace.
///
/// This is intentionally strict:
/// - Requires canonical padding (length divisible by 4 after whitespace removal).
/// - Rejects non-canonical trailing bits when '=' padding is present.
///
/// On success returns the decoded bytes.
pub(crate) fn decode_standard_ws(input: &str) -> Result<Vec<u8>, DecodeError> {
    let bytes = input.as_bytes();

    // Rough prealloc: includes whitespace but avoids realloc in most realistic inputs.
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);

    let mut quad = [0u8; 4];
    let mut q_len: usize = 0;
    let mut finished = false;

    for &b in bytes {
        if is_ascii_ws(b) {
            continue;
        }

        let v = decode_sextet(b).ok_or(DecodeError::InvalidByte(b))?;

        if finished {
            return Err(DecodeError::TrailingData);
        }

        quad[q_len] = v;
        q_len += 1;

        if q_len == 4 {
            let a = quad[0];
            let b = quad[1];
            let c = quad[2];
            let d = quad[3];

            // '=' is not allowed in the first two positions.
            if a == 64 || b == 64 {
                return Err(DecodeError::InvalidPadding);
            }

            if c == 64 {
                // Must be "xx=="
                if d != 64 {
                    return Err(DecodeError::InvalidPadding);
                }

                // Canonical trailing bits: low 4 bits of b must be zero.
                if (b & 0x0F) != 0 {
                    return Err(DecodeError::InvalidTrailingBits);
                }

                out.push((a << 2) | (b >> 4));
                finished = true;
            } else if d == 64 {
                // Must be "xxx="
                // Canonical trailing bits: low 2 bits of c must be zero.
                if (c & 0x03) != 0 {
                    return Err(DecodeError::InvalidTrailingBits);
                }

                out.push((a << 2) | (b >> 4));
                out.push(((b & 0x0F) << 4) | (c >> 2));
                finished = true;
            } else {
                // Full quad: "xxxx"
                out.push((a << 2) | (b >> 4));
                out.push(((b & 0x0F) << 4) | (c >> 2));
                out.push(((c & 0x03) << 6) | d);
            }

            q_len = 0;
        }
    }

    // Must end on a quad boundary.
    if q_len != 0 {
        return Err(DecodeError::InvalidLength);
    }

    Ok(out)
}
```

### 4) Wire it into `datamashup_framing.rs`

In `core/src/lib.rs`, add the module behind the feature:

```rust
// core/src/lib.rs

#[cfg(feature = "custom-base64")]
mod custom_base64;
```

Now update `core/src/datamashup_framing.rs` where `decode_datamashup_base64` is defined. Your current implementation uses `split_whitespace().collect()` and `STANDARD.decode(...)`. 

Replace the decoding section with this feature-switched implementation:

```rust
// core/src/datamashup_framing.rs

#[cfg(all(not(feature = "custom-base64"), not(feature = "base64-crate")))]
compile_error!(
    "No Base64 backend selected. Enable feature \"base64-crate\" (default) or \"custom-base64\"."
);

#[cfg(feature = "base64-crate")]
use base64::engine::general_purpose::STANDARD;
#[cfg(feature = "base64-crate")]
use base64::Engine;

#[cfg(feature = "custom-base64")]
use crate::custom_base64;

// Baseline: third-party base64 crate (current behavior).
#[cfg(feature = "base64-crate")]
fn decode_datamashup_base64_crate(text: &str) -> Result<Vec<u8>, DataMashupError> {
    // DataMashup base64 often contains line breaks; strip whitespace first.
    let cleaned: String = text.split_whitespace().collect();
    STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| DataMashupError::Base64Invalid)
}

// Custom: strict decoder that ignores ASCII whitespace while decoding.
#[cfg(feature = "custom-base64")]
fn decode_datamashup_base64_custom(text: &str) -> Result<Vec<u8>, DataMashupError> {
    custom_base64::decode_standard_ws(text).map_err(|_| DataMashupError::Base64Invalid)
}

pub fn decode_datamashup_base64(text: &str) -> Result<Vec<u8>, DataMashupError> {
    #[cfg(feature = "custom-base64")]
    {
        return decode_datamashup_base64_custom(text);
    }

    #[cfg(not(feature = "custom-base64"))]
    {
        return decode_datamashup_base64_crate(text);
    }
}
```

### 5) Add/extend tests for whitespace + parity

In `core/src/datamashup_framing.rs`’s existing tests module, add:

```rust
#[test]
fn decode_datamashup_base64_ignores_ascii_whitespace() {
    // "QQ==" -> b"A"
    let text = "  Q Q==\n\t\r";
    let out = decode_datamashup_base64(text).unwrap();
    assert_eq!(out, b"A");
}

#[cfg(all(feature = "custom-base64", feature = "base64-crate"))]
#[test]
fn custom_base64_matches_base64_crate_for_various_lengths() {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    // Deterministic "pseudo-random" bytes without adding a rand dependency.
    fn make_bytes(len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len);
        let mut x: u32 = 0x1234_5678;
        for _ in 0..len {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            v.push((x >> 24) as u8);
        }
        v
    }

    for &len in &[0usize, 1, 2, 3, 10, 57, 58, 59, 60, 128, 1024] {
        let data = make_bytes(len);
        let b64 = STANDARD.encode(&data);

        // Insert whitespace to simulate DataMashup formatting.
        let mut spaced = String::with_capacity(b64.len() + (b64.len() / 10) + 8);
        for (i, ch) in b64.chars().enumerate() {
            if i % 60 == 0 {
                spaced.push('\n');
            }
            spaced.push(ch);
            if i % 13 == 0 {
                spaced.push(' ');
            }
        }

        let a = decode_datamashup_base64_crate(&spaced).unwrap();
        let b = decode_datamashup_base64_custom(&spaced).unwrap();

        assert_eq!(a, data);
        assert_eq!(b, data);
        assert_eq!(a, b);
    }
}
```

---

## How you run the A/B/C variants for this candidate

* **A (baseline)**: default features (includes `base64-crate`)
  `cargo test -p excel_diff`
  `cargo bench -p excel_diff`

* **B (custom, no base64 crate in the graph)**:
  `cargo test -p excel_diff --no-default-features --features "excel-open-xml std-fs vba dpapi custom-base64"`
  (Adjust the feature list to match what you need enabled in that build.)

* **C (parity tests)**: enable both so you can compare implementations side-by-side
  `cargo test -p excel_diff --features "custom-base64"`
  (Default still includes `base64-crate`, so parity tests compile/run.)

This aligns with your A/B/C approach and parity testing goals.  

---

If you want, after you run this once, the next best “small and safe” follow-up is the `lru` replacement in `desktop/backend/src/diff_runner.rs` (tiny cache sizes, easy to validate, immediate dependency removal). 
