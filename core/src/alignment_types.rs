#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RowAlignment {
    pub matched: Vec<(u32, u32)>,
    pub inserted: Vec<u32>,
    pub deleted: Vec<u32>,
    pub moves: Vec<RowBlockMove>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBlockMove {
    pub src_start_row: u32,
    pub dst_start_row: u32,
    pub row_count: u32,
}
