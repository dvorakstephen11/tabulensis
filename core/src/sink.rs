use crate::diff::{DiffError, DiffOp};

/// Trait for streaming diff operations to a consumer.
pub trait DiffSink {
    fn emit(&mut self, op: DiffOp) -> Result<(), DiffError>;

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
