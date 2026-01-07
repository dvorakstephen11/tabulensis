This crate exposes a small set of feature flags that map directly to engine
capabilities. The intent is that a desktop build cannot silently drift from the
engine feature set; each feature here forwards to the matching feature in the
`excel_diff` dependency and gates desktop-only surfaces that rely on it.

`model-diff` enables the model-diff UI and export paths in the desktop shell and
forwards to `excel_diff/model-diff`. `perf-metrics` enables extra perf
instrumentation in the desktop diff runner and forwards to
`excel_diff/perf-metrics`.
