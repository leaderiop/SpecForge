#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SourceSpan {
    pub file: String,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}
