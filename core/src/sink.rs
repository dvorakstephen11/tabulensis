use crate::diff::{DiffError, DiffOp};
use crate::string_pool::StringPool;

/// Trait for streaming diff operations to a consumer.
pub trait DiffSink {
    /// Called once before any ops are emitted.
    ///
    /// Default is a no-op so sinks that don't need setup can ignore it.
    fn begin(&mut self, _pool: &StringPool) -> Result<(), DiffError> {
        Ok(())
    }

    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;

    fn finish(&mut self) -> Result<(), DiffError> {
        Ok(())
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
