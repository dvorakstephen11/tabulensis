use crate::diff::{DiffError, DiffOp};
use crate::string_pool::StringPool;
use std::marker::PhantomData;

/// Trait for streaming diff operations to a consumer.
///
/// See `docs/streaming_contract.md` for determinism, lifecycle, and string table rules.
///
/// Streaming entry points call sinks in this order:
///
/// 1. `begin(pool)` once (before any ops)
/// 2. `emit(op)` zero or more times
/// 3. `finish()` once (even on most error paths)
///
/// Sinks can use `begin` to access the string table (via `pool.strings()`), e.g. to write a
/// header before streaming ops.
pub trait DiffSink {
    /// Called once before any ops are emitted.
    ///
    /// Default is a no-op so sinks that don't need setup can ignore it.
    fn begin(&mut self, _pool: &StringPool) -> Result<(), DiffError> {
        Ok(())
    }

    /// Emit one diff operation.
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;

    /// Finish the stream (flush/close output destinations).
    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
    }
}

pub(crate) struct SinkFinishGuard<S: DiffSink> {
    sink: *mut S,
    armed: bool,
    _marker: PhantomData<*mut S>,
}

impl<S: DiffSink> SinkFinishGuard<S> {
    pub(crate) fn new(sink: &mut S) -> Self {
        Self {
            sink,
            armed: true,
            _marker: PhantomData,
        }
    }

    pub(crate) fn finish_and_disarm(&mut self) -> Result<(), DiffError> {
        self.armed = false;
        // Safety: the guard is created from an exclusive borrow of `sink` and
        // tied to its lifetime via PhantomData, so the pointer remains valid.
        unsafe { (&mut *self.sink).finish() }
    }
}

impl<S: DiffSink> Drop for SinkFinishGuard<S> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        // Best-effort finish; ignore errors to avoid masking the original error.
        unsafe {
            let _ = (&mut *self.sink).finish();
        }
    }
}

pub(crate) struct NoFinishSink<'a, S: DiffSink> {
    inner: &'a mut S,
}

impl<'a, S: DiffSink> NoFinishSink<'a, S> {
    pub(crate) fn new(inner: &'a mut S) -> Self {
        Self { inner }
    }
}

impl<S: DiffSink> DiffSink for NoFinishSink<'_, S> {
    fn begin(&mut self, pool: &StringPool) -> Result<(), DiffError> {
        self.inner.begin(pool)
    }

    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        self.inner.emit(op)
    }

    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
    }
}

/// A sink that collects ops into a Vec for compatibility.
pub struct VecSink {
    ops: Vec<DiffOp>,
}

impl VecSink {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn op_capacity(&self) -> usize {
        self.ops.capacity()
    }

    pub fn op_len(&self) -> usize {
        self.ops.len()
    }

    pub fn into_ops(self) -> Vec<DiffOp> {
        self.ops
    }
}

impl DiffSink for VecSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        self.ops.push(op);
        Ok(())
    }
}

/// A sink that forwards ops to a callback.
pub struct CallbackSink<F: FnMut(DiffOp)> {
    f: F,
}

impl<F: FnMut(DiffOp)> CallbackSink<F> {
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F: FnMut(DiffOp)> DiffSink for CallbackSink<F> {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError> {
        (self.f)(op);
        Ok(())
    }
}
