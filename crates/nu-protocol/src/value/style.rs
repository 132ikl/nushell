use serde::{Deserialize, Serialize};

use crate::Span;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    style: StyleOptions,
    span: Span,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StyleOptions {
    Color(u8, u8, u8),
}
