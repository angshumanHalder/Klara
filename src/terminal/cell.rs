#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellContent {
    Empty,
    Narrow(String),
    WideLeading(String),
    WideContinuation,
}

impl CellContent {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Empty | Self::WideContinuation => " ",
            Self::Narrow(text) | Self::WideLeading(text) => text,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub content: CellContent,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            content: CellContent::Empty,
            fg: Color::Default,
            bg: Color::Default,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_cell_has_empty_content() {
        assert_eq!(Cell::default().content, CellContent::Empty);
    }

    #[test]
    fn cell_content_exposes_renderable_text() {
        assert_eq!(CellContent::Empty.as_str(), " ");
        assert_eq!(CellContent::Narrow("x".into()).as_str(), "x");
        assert_eq!(CellContent::WideLeading("界".into()).as_str(), "界");
        assert_eq!(CellContent::WideContinuation.as_str(), " ");
    }
}
