I reviewed your **updated_codebase.md** specifically against items **#1, #2, and #4** from the “What I would do next (highest ROI to raise ship confidence)” section.

You’ve implemented the *core* of the plan well. There are just a couple of “last 10%” issues that keep it from being **fully** correct/complete as-written—mostly around making the **memory gate truly “hard”** and making the **fixture guard cover all the fixture types you now use**.

## What you implemented well

### Item #1: Full-scale perf + memory as a hard gate

You have the right structure in place:

* A dedicated **full-scale perf workflow** that runs on pushes to `main/master`, on a schedule, and manually, and it **requires a baseline** and exports artifacts. 
* A **release gating job** (`perf-fullscale-gate`) that runs the full-scale suite and blocks downstream build/publish jobs.
* A `check_perf_thresholds.py` implementation that includes **full-scale thresholds** and includes **peak memory caps** for full-scale tests. 

That matches the intent of item #1: “no-excuses gate before ship.”

### Item #2: Unified fixtures and “missing file” prevention

You did the big things right:

* You added PBIX fixtures to the **fixture manifest** so tests can reliably reference generated files (rather than depending on ad-hoc local files/corpus). 
* Your PBIX tests reference fixtures by name (e.g., `pbix_legacy_one_query_a.pbix`, `pbix_no_datamashup.pbix`), which aligns with the manifest-based approach.
* You strengthened `check_fixture_references.py` to scan both Rust test code and YAML under tests and to enforce that referenced fixture names exist in the right manifest(s). 

This is exactly the “fixture hygiene” you want before shipping, and it matches the intent of item #2.

### Item #4: Workspace feature hygiene, especially desktop

This is in a very good spot now:

* The **desktop crate** defines the `model-diff` and `perf-metrics` features and forwards them into the `excel_diff` crate. 
* CI explicitly checks the desktop crate with **`-Dunexpected_cfgs`** and checks compilation under feature combinations. 

That’s precisely the “tighten workspace feature hygiene” goal.

## What’s not fully correct or complete

### Issue 1: The memory gate is not truly “hard” if the metric goes missing

Right now, your perf threshold check does this:

* `actual_peak = suite_metrics[test_name].get("peak_memory_bytes", 0)`
* Then compares it against `max_peak_memory_bytes`
* Which means: **if `peak_memory_bytes` is missing**, it silently becomes **0** and passes the memory limit check. 

That undermines the spirit of item #1 (hard gate), because the most dangerous failure mode is “we stopped measuring memory and didn’t notice.”

### Issue 2: Your fixture guard regex won’t catch `.xlsb` fixtures (and you reference `.xlsb`)

Your fixture guard looks for fixture names in tests via:

* `RE_FIXTURE_NAME` only matching extensions `xlsx|xlsm|pbix|pbit|zip|txt|bin`
* and your workflow guard similarly excludes `.xlsb`. 

But you have test scenario YAML that references an `.xlsb` fixture:

* `file: "xlsb_stub.xlsb"` 

So right now, **a missing or renamed `.xlsb` fixture can slip past the guard**, and you’ll only find out later when tests explode.

This is a completeness gap relative to item #2.

## Remediation plan with code

Below are the two concrete fixes I’d make. Each change is shown as:

1. the code you should replace
2. the code to replace it with

No diffs.

---

## Fix 1: Expand fixture reference detection to include xlsb and improve robustness

### File: `scripts/check_fixture_references.py`

Replace this:

```python
RE_FIXTURE_NAME = re.compile(r'"([A-Za-z0-9._-]+\.(?:xlsx|xlsm|pbix|pbit|zip|txt|bin))"')
RE_WORKFLOW_REF = re.compile(r"fixtures/generated/([A-Za-z0-9._-]+\.(?:xlsx|xlsm|pbix|pbit|zip|txt|bin))")
```

With this:

```python
RE_FIXTURE_NAME = re.compile(
    r"""['"]([A-Za-z0-9._-]+\.(?:xlsx|xlsm|xltx|xltm|xlsb|pbix|pbit|zip|txt|bin))['"]"""
)

RE_WORKFLOW_REF = re.compile(
    r"fixtures/generated/([A-Za-z0-9._-]+\.(?:xlsx|xlsm|xltx|xltm|xlsb|pbix|pbit|zip|txt|bin))"
)
```

Why this exact change:

* Adds `.xlsb` so `xlsb_stub.xlsb` references are guarded.
* Also adds `.xltx/.xltm` (your spec supports these), so you don’t have to revisit this later.
* Accepts both single and double quotes, which is common across YAML and Rust test strings.

---

## Fix 2: Make peak memory measurement mandatory when a peak-memory threshold is configured

### File: `scripts/check_perf_thresholds.py`

Right now, the relevant block silently defaults missing memory to 0. 
Change it so:

* If `max_peak_memory_bytes` is set
* And the test did not report `peak_memory_bytes`
* The check fails loudly

### Replace this block:

```python
        max_peak = threshold.get("max_peak_memory_bytes")
        actual_peak = suite_metrics[test_name].get("peak_memory_bytes", 0)

        if actual_time_s > max_time_s:
            status = "FAIL"
            failures.append((test_name, actual_time_s, max_time_s))
        else:
            status = "PASS"

        line = f"  {test_name}: {actual_time_s:.3f}s / {max_time_s:.1f}s [{status}]"
        if max_peak is not None:
            if actual_peak > max_peak:
                failures.append((test_name, actual_peak, max_peak))
                mem_status = "FAIL"
            else:
                mem_status = "PASS"
            line += f", peak={actual_peak} / {max_peak} bytes [{mem_status}]"
        print(line)
```

### With this block:

```python
        max_peak = threshold.get("max_peak_memory_bytes")

        if actual_time_s > max_time_s:
            status = "FAIL"
            failures.append((test_name, actual_time_s, max_time_s))
        else:
            status = "PASS"

        line = f"  {test_name}: {actual_time_s:.3f}s / {max_time_s:.1f}s [{status}]"

        if max_peak is not None:
            if "peak_memory_bytes" not in suite_metrics[test_name]:
                failures.append((test_name, "MISSING_PEAK_MEMORY_BYTES", max_peak))
                mem_status = "FAIL"
                line += f", peak=missing / {max_peak} bytes [{mem_status}]"
            else:
                actual_peak = suite_metrics[test_name]["peak_memory_bytes"]
                if actual_peak > max_peak:
                    failures.append((test_name, actual_peak, max_peak))
                    mem_status = "FAIL"
                else:
                    mem_status = "PASS"
                line += f", peak={actual_peak} / {max_peak} bytes [{mem_status}]"

        print(line)
```

Now update the failure printing so missing memory metrics read cleanly.

### Replace this block:

```python
        print("PERF FAILURES:")
        for test_name, actual, max_cap in failures:
            if isinstance(actual, float):
                print(f"  {test_name}: {actual:.3f}s exceeded max of {max_cap:.1f}s")
            else:
                print(f"  {test_name}: peak_memory_bytes {actual} exceeded max of {max_cap}")
```

### With this block:

```python
        print("PERF FAILURES:")
        for test_name, actual, max_cap in failures:
            if isinstance(actual, float):
                print(f"  {test_name}: {actual:.3f}s exceeded max of {max_cap:.1f}s")
            elif actual == "MISSING_PEAK_MEMORY_BYTES":
                print(f"  {test_name}: missing peak_memory_bytes metric (max configured {max_cap} bytes)")
            else:
                print(f"  {test_name}: peak_memory_bytes {actual} exceeded max of {max_cap}")
```

This turns “memory gate” from “best effort” into “hard gate.”

---

## Validation checklist

After the above changes:

1. Run fixture guard locally:

   * `python scripts/check_fixture_references.py`
   * Confirm it now detects `xlsb_stub.xlsb` and that it exists in the manifest output set.

2. Run the normal dev test loop:

   * `python scripts/dev_test.py`
   * This should still generate fixtures and run core+CLI tests.

3. Run full-scale perf gate locally (or in CI):

   * `python scripts/check_perf_thresholds.py --suite full-scale --require-baseline --baseline benchmarks/baselines/full-scale.json --export-json benchmarks/latest_fullscale.json`
   * Confirm it fails if `peak_memory_bytes` is not emitted for any test with a peak-memory cap.

---

If you apply those two fixes, I’d consider items **#1, #2, and #4** implemented correctly and completely, in the “ship confidence” sense that the original document was aiming for.
