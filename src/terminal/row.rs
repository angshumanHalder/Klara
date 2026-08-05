use crate::terminal::cell::{Cell, CellContent, CellStyle, HyperlinkId};

pub(super) struct Row {
    cells: Vec<Cell>,
}

impl Row {
    pub(super) fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
        }
    }

    pub(super) fn cell(&self, col: usize) -> &Cell {
        &self.cells[col]
    }

    pub(super) fn clear_cell(&mut self, col: usize) {
        let paired_col = match &self.cells[col].content {
            CellContent::WideLeading(_)
                if col + 1 < self.cells.len()
                    && matches!(&self.cells[col + 1].content, CellContent::WideContinuation) =>
            {
                Some(col + 1)
            }
            CellContent::WideContinuation
                if col > 0
                    && matches!(&self.cells[col - 1].content, CellContent::WideLeading(_)) =>
            {
                Some(col - 1)
            }
            _ => None,
        };
        self.cells[col] = Cell::default();
        if let Some(paired_col) = paired_col {
            self.cells[paired_col] = Cell::default();
        }
    }

    pub(super) fn resize(&mut self, new_cols: usize) {
        let truncates_wide_pair = new_cols < self.cells.len()
            && new_cols > 0
            && matches!(
                &self.cells[new_cols - 1].content,
                CellContent::WideLeading(_)
            )
            && matches!(&self.cells[new_cols].content, CellContent::WideContinuation);
        if truncates_wide_pair {
            self.cells[new_cols - 1] = Cell::default();
        }
        self.cells.resize(new_cols, Cell::default());
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.cells.len()
    }

    pub(super) fn is_blank(&self) -> bool {
        let blank = Cell::default();
        self.cells.iter().all(|c| c == &blank)
    }

    pub(super) fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    pub(super) fn write_narrow(
        &mut self,
        col: usize,
        text: String,
        style: CellStyle,
        hyperlink: Option<HyperlinkId>,
    ) {
        self.clear_cell(col);
        self.cells[col] = Cell {
            content: CellContent::Narrow(text),
            style,
            hyperlink,
        }
    }

    #[cfg(test)]
    pub(super) fn write_wide(
        &mut self,
        col: usize,
        text: String,
        style: CellStyle,
        hyperlink: Option<HyperlinkId>,
    ) -> Result<(), ()> {
        if col >= self.cells.len() || col + 1 >= self.cells.len() {
            Err(())
        } else {
            self.clear_cell(col);
            self.clear_cell(col + 1);
            self.cells[col] = Cell {
                content: CellContent::WideLeading(text),
                style,
                hyperlink,
            };
            self.cells[col + 1] = Cell {
                content: CellContent::WideContinuation,
                style,
                hyperlink,
            };
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::terminal::cell::{CellFlags, Color};

    use super::*;

    #[test]
    fn new_creates_requested_number_of_default_cells() {
        let row = Row::new(4);

        assert_eq!(row.len(), 4);
        assert!(row.is_blank());
        for col in 0..row.len() {
            assert_eq!(row.cell(col), &Cell::default());
        }
    }

    #[test]
    fn clearing_wide_leader_clears_its_continuation() {
        let mut row = Row::new(3);
        row.write_wide(
            1,
            "界".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        )
        .unwrap();

        row.clear_cell(1);

        assert_eq!(row.cell(1), &Cell::default());
        assert_eq!(row.cell(2), &Cell::default());
    }

    #[test]
    fn clearing_wide_continuation_clears_its_leader() {
        let mut row = Row::new(3);
        row.write_wide(
            1,
            "界".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        )
        .unwrap();

        row.clear_cell(2);

        assert_eq!(row.cell(1), &Cell::default());
        assert_eq!(row.cell(2), &Cell::default());
    }

    #[test]
    fn shrinking_across_wide_pair_clears_retained_leader() {
        let mut row = Row::new(3);
        row.write_wide(
            1,
            "界".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        )
        .unwrap();

        row.resize(2);

        assert_eq!(row.len(), 2);
        assert_eq!(row.cell(1), &Cell::default());
    }

    #[test]
    fn growing_preserves_existing_content() {
        let mut row = Row::new(2);
        row.write_narrow(
            1,
            "A".into(),
            CellStyle {
                fg: Color::Indexed(2),
                bg: Color::Rgb(10, 20, 30),
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        );

        row.resize(4);

        assert_eq!(row.len(), 4);
        assert_eq!(row.cell(1).content, CellContent::Narrow("A".into()));
        assert_eq!(row.cell(1).style.fg, Color::Indexed(2));
        assert_eq!(row.cell(1).style.bg, Color::Rgb(10, 20, 30));
        assert_eq!(row.cell(2), &Cell::default());
        assert_eq!(row.cell(3), &Cell::default());
    }

    #[test]
    fn narrow_write_over_a_wide_leader() {
        let mut row = Row::new(3);
        row.write_wide(
            1,
            "界".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        )
        .unwrap();
        row.write_narrow(
            1,
            "A".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        );

        assert_eq!(row.cell(1).content, CellContent::Narrow("A".into()));
        assert_eq!(row.cell(2).content, CellContent::Empty);
    }

    #[test]
    fn narrow_write_over_a_wide_continuation() {
        let mut row = Row::new(3);
        row.write_wide(
            1,
            "界".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        )
        .unwrap();
        row.write_narrow(
            2,
            "A".into(),
            CellStyle {
                fg: Color::Default,
                bg: Color::Default,
                flags: CellFlags::empty(),
                ..CellStyle::default()
            },
            None,
        );

        assert_eq!(row.cell(1).content, CellContent::Empty);
        assert_eq!(row.cell(2).content, CellContent::Narrow("A".into()));
    }

    #[test]
    fn wide_write_into_empty_cells() {
        let mut row = Row::new(3);

        let style = CellStyle {
            fg: Color::Indexed(2),
            bg: Color::Rgb(10, 20, 30),
            flags: CellFlags::empty(),
            ..CellStyle::default()
        };

        row.write_wide(1, "界".into(), style, None).unwrap();

        assert_eq!(row.cell(1).content, CellContent::WideLeading("界".into()));
        assert_eq!(row.cell(2).content, CellContent::WideContinuation);
    }

    #[test]
    fn wide_write_overlapping_an_existing_pair_on_the_left() {
        let mut row = Row::new(3);

        let style = CellStyle {
            fg: Color::Indexed(2),
            bg: Color::Rgb(10, 20, 30),
            flags: CellFlags::empty(),
            ..CellStyle::default()
        };

        row.write_wide(1, "猫".into(), style, None).unwrap();
        row.write_wide(1, "好".into(), style, None).unwrap();

        assert_eq!(row.cell(1).content, CellContent::WideLeading("好".into()));
        assert_eq!(row.cell(2).content, CellContent::WideContinuation);
    }

    #[test]
    fn wide_write_overlapping_an_existing_pair_on_the_right() {
        let mut row = Row::new(4);

        let style = CellStyle {
            fg: Color::Indexed(2),
            bg: Color::Rgb(10, 20, 30),
            flags: CellFlags::empty(),
            ..CellStyle::default()
        };

        row.write_wide(2, "猫".into(), style, None).unwrap();
        row.write_wide(1, "好".into(), style, None).unwrap();

        assert_eq!(row.cell(1).style.fg, Color::Indexed(2));
        assert_eq!(row.cell(1).style.bg, Color::Rgb(10, 20, 30));
        assert_eq!(row.cell(2).style.fg, Color::Indexed(2));
        assert_eq!(row.cell(2).style.bg, Color::Rgb(10, 20, 30));

        assert_eq!(row.cell(1).content, CellContent::WideLeading("好".into()));
        assert_eq!(row.cell(2).content, CellContent::WideContinuation);
        assert_eq!(row.cell(3), &Cell::default());
    }

    #[test]
    fn wide_write_at_last_column_returns_error_without_mutation() {
        let mut row = Row::new(3);

        let style = CellStyle {
            fg: Color::Indexed(1),
            bg: Color::Default,
            flags: CellFlags::empty(),
            ..CellStyle::default()
        };

        row.write_narrow(2, "A".into(), style, None);

        assert!(row.write_wide(2, "猫".into(), style, None).is_err());

        assert_eq!(row.cell(2).content, CellContent::Narrow("A".into()));
        assert_eq!(row.cell(2).style.fg, Color::Indexed(1));
    }

    #[test]
    fn clear_preserves_length_resets_all_cells() {
        let mut row = Row::new(2);

        let style = CellStyle {
            fg: Color::Indexed(1),
            bg: Color::Default,
            flags: CellFlags::empty(),
            ..CellStyle::default()
        };

        row.write_narrow(1, "A".into(), style, None);

        row.clear();

        assert!(row.is_blank());
        assert_eq!(row.len(), 2);
    }

    #[test]
    fn narrow_write_preserves_hyperlink() {
        let mut row = Row::new(2);
        let style = CellStyle {
            ..Default::default()
        };
        row.write_narrow(0, "A".into(), style, Some(HyperlinkId(7)));
        assert_eq!(row.cell(0).hyperlink, Some(HyperlinkId(7)));
    }

    #[test]
    fn wide_write_applies_hyperlink_to_both_cells() {
        let mut row = Row::new(4);
        let style = CellStyle {
            ..Default::default()
        };
        row.write_wide(1, "猫".into(), style, Some(HyperlinkId(7)))
            .unwrap();

        assert_eq!(row.cell(1).hyperlink, Some(HyperlinkId(7)));
        assert_eq!(row.cell(2).hyperlink, Some(HyperlinkId(7)));
    }
}
