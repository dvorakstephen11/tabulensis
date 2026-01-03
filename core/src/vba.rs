use crate::string_pool::StringId;

/// The kind of VBA module contained in an `.xlsm` workbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbaModuleType {
    /// A standard module (e.g., `Module1`).
    Standard,
    /// A class module.
    Class,
    /// A form module.
    Form,
    /// A document module (e.g., `ThisWorkbook`, sheet modules).
    Document,
}

/// A VBA module extracted from a workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaModule {
    /// Module name (interned in the associated string pool).
    pub name: StringId,
    /// Module type (standard/class/form/document).
    pub module_type: VbaModuleType,
    /// Raw module source code.
    pub code: String,
}
