// Source Span and diagnostic messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

// compile error: span for line and column numbers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub span: Span,
    pub msg: String,
}

impl Diag {
    pub fn new(start: usize, end: usize, msg: impl Into<String>) -> Self {
        Diag {
            span: Span { start, end },
            msg: msg.into(),
        }
    }
}