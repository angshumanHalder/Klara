use bitflags::bitflags;

#[derive(Debug, Clone, PartialEq, Copy, Eq)]
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
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            content: CellContent::Empty,
            style: CellStyle::default(),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct CellFlags: u16 {
        const BOLD = 1 << 0;
        const DIM = 1 << 1;
        const ITALIC = 1 << 2;
        const UNDERLINE = 1 << 3;
        const STRIKEOUT = 1 << 4;
        const REVERSE = 1 << 5;
        const HIDDEN = 1 << 6;
        const BLINK = 1 << 7;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CellStyle {
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
}

impl Default for CellStyle {
    fn default() -> Self {
        CellStyle {
            fg: Color::Default,
            bg: Color::Default,
            flags: CellFlags::empty(),
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

    #[test]
    fn default_flags_are_empty() {
        assert!(CellFlags::default().is_empty());
    }

    #[test]
    fn multiple_flags_can_coexist() {
        let flags = CellFlags::BOLD | CellFlags::ITALIC;
        assert!(flags.contains(CellFlags::ITALIC));
        assert!(flags.contains(CellFlags::BOLD));
        assert!(!flags.contains(CellFlags::DIM));
    }

    #[test]
    fn removing_one_flag_preserves_another() {
        let mut flags = CellFlags::BOLD | CellFlags::ITALIC;
        assert!(flags.contains(CellFlags::ITALIC));
        assert!(flags.contains(CellFlags::BOLD));

        flags.remove(CellFlags::BOLD);
        assert!(flags.contains(CellFlags::ITALIC));
        assert!(!flags.contains(CellFlags::BOLD));
    }

    #[test]
    fn toggling_the_flag_twice_restores_original_state() {
        let mut flag = CellFlags::BOLD;
        flag.toggle(CellFlags::BOLD);
        assert_ne!(flag, CellFlags::BOLD);
        flag.toggle(CellFlags::BOLD);
        assert_eq!(flag, CellFlags::BOLD);
    }

    #[test]
    fn default_style_has_default_properties() {
        let style = CellStyle::default();
        assert_eq!(style.fg, Color::Default);
        assert_eq!(style.bg, Color::Default);
        assert_eq!(style.flags, CellFlags::empty());
    }

    #[test]
    fn a_style_can_hold_several_flags() {
        let mut style = CellStyle::default();
        style.flags = CellFlags::BOLD | CellFlags::ITALIC | CellFlags::BLINK;

        assert!(style.flags.contains(CellFlags::BOLD));
        assert!(style.flags.contains(CellFlags::ITALIC));
        assert!(style.flags.contains(CellFlags::BLINK));
    }

    #[test]
    fn copying_and_modifying_one_style_does_not_modify_original() {
        let style = CellStyle::default();
        let mut copied_style = style;

        copied_style.flags = CellFlags::BLINK;

        assert!(style.flags.is_empty());
        assert!(copied_style.flags.contains(CellFlags::BLINK));
    }
}
