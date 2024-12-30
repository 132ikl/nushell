use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    style: StyleOptions,
    span: StyleSpan,
}

/// The span which a style should be applied to.
///
/// Not to be confused with `span::Span`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StyleSpan {
    pub start: usize,
    pub end: usize,
}

impl StyleSpan {
    pub fn new(start: usize, end: usize) -> Self {
        StyleSpan { start, end }
    }
}

impl Style {
    pub fn new(style: StyleOptions, span: StyleSpan) -> Self {
        Style { style, span }
    }

    pub fn apply_ansi(&self, string: String) -> String {
        let mut prefix = string;
        let mut spanned = prefix.split_off(self.span.start);
        let suffix = spanned.split_off(self.span.end);

        // TODO: breaks other StyleSpans
        let styled_span = match self.style {
            StyleOptions::Color(r, g, b) => {
                let term_color = nu_ansi_term::Color::Rgb(r, g, b);
                term_color.paint(spanned).to_string()
            }
        };

        prefix + &styled_span + &suffix
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StyleOptions {
    Color(u8, u8, u8),
}
