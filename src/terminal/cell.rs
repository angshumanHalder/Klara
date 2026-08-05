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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

impl CellContent {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Empty | Self::WideContinuation => " ",
            Self::Narrow(text) | Self::WideLeading(text) => text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HyperlinkId(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub content: CellContent,
    pub style: CellStyle,
    pub hyperlink: Option<HyperlinkId>,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            content: CellContent::Empty,
            style: CellStyle::default(),
            hyperlink: None,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct CellFlags: u16 {
        const BOLD = 1 << 0;
        const DIM = 1 << 1;
        const ITALIC = 1 << 2;
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
    pub underline_style: UnderlineStyle,
    pub underline_color: Option<Color>,
}

impl Default for CellStyle {
    fn default() -> Self {
        CellStyle {
            fg: Color::Default,
            bg: Color::Default,
            flags: CellFlags::empty(),
            underline_style: UnderlineStyle::default(),
            underline_color: None,
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
        assert_eq!(style.underline_style, UnderlineStyle::None);
        assert_eq!(style.underline_color, None);
    }

    #[test]
    fn default_underline_style_is_none() {
        assert_eq!(UnderlineStyle::default(), UnderlineStyle::None);
    }

    #[test]
    fn style_stores_explicit_underline_style_and_color() {
        let style = CellStyle {
            underline_style: UnderlineStyle::Curly,
            underline_color: Some(Color::Rgb(10, 20, 30)),
            ..CellStyle::default()
        };

        assert_eq!(style.underline_style, UnderlineStyle::Curly);
        assert_eq!(style.underline_color, Some(Color::Rgb(10, 20, 30)));
    }

    #[test]
    fn copying_style_preserves_underline_style_and_color() {
        let style = CellStyle {
            underline_style: UnderlineStyle::Double,
            underline_color: Some(Color::Indexed(4)),
            ..CellStyle::default()
        };

        let copied_style = style;

        assert_eq!(copied_style.underline_style, UnderlineStyle::Double);
        assert_eq!(copied_style.underline_color, Some(Color::Indexed(4)));
    }

    #[test]
    fn a_style_can_hold_several_flags() {
        let style = CellStyle {
            flags: CellFlags::BOLD | CellFlags::ITALIC | CellFlags::BLINK,
            ..CellStyle::default()
        };

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

    #[test]
    fn default_cell_has_no_hyperlink() {
        assert_eq!(Cell::default().hyperlink, None);
    }

    #[test]
    fn cell_can_reference_a_hyperlink() {
        let cell = Cell {
            hyperlink: Some(HyperlinkId(7)),
            ..Default::default()
        };
        assert_eq!(cell.hyperlink, Some(HyperlinkId(7)));
    }
}
