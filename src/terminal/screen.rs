use std::ops::Range;

use crate::terminal::{cell::Cell, row::Row};

pub(super) struct Screen {
    rows: Vec<Row>,
    cursor: Cursor,
    saved_cursor: Cursor,
    pub(super) pending_wrap: bool,
}

impl Screen {
    pub(super) fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows: (0..rows).map(|_| Row::new(cols)).collect(),
            cursor: Cursor::default(),
            saved_cursor: Cursor::default(),
            pending_wrap: false,
        }
    }

    pub(super) fn cell(&self, row: usize, col: usize) -> &Cell {
        self.rows[row].cell(col)
    }

    pub(super) fn row(&self, row: usize) -> &Row {
        &self.rows[row]
    }

    pub(super) fn row_mut(&mut self, row: usize) -> &mut Row {
        &mut self.rows[row]
    }

    pub(super) fn cursor(&self) -> Cursor {
        self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    pub(super) fn clear(&mut self) {
        for row in &mut self.rows {
            row.clear();
        }
    }

    pub(super) fn scroll_up(&mut self, region: Range<usize>) {
        debug_assert!(region.start < region.end);
        debug_assert!(region.end <= self.rows.len());

        let bottom = region.end - 1;
        self.rows[region].rotate_left(1);
        self.rows[bottom].clear();
    }

    pub(super) fn resize(&mut self, new_rows: usize, new_cols: usize) {
        resize_buffer(&mut self.rows, new_rows, new_cols);

        self.cursor = Cursor {
            row: self.cursor.row.min(new_rows - 1),
            col: self.cursor.col.min(new_cols - 1),
        };

        self.saved_cursor = Cursor {
            row: self.saved_cursor.row.min(new_rows - 1),
            col: self.saved_cursor.col.min(new_cols - 1),
        };
    }

    pub(super) fn save_cursor(&mut self) {
        self.saved_cursor = self.cursor;
    }

    pub(super) fn restore_cursor(&mut self) {
        self.cursor = self.saved_cursor;
    }

    pub(super) fn reset_cursor(&mut self) {
        self.cursor = Cursor::default();
        self.pending_wrap = false;
    }
}

fn resize_buffer(buffer: &mut Vec<Row>, new_rows: usize, new_cols: usize) {
    buffer.truncate(new_rows);

    for row in buffer.iter_mut() {
        row.resize(new_cols);
    }

    buffer.resize_with(new_rows, || Row::new(new_cols));
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Cursor {
    pub(super) row: usize,
    pub(super) col: usize,
}

#[cfg(test)]
mod test {
    use crate::terminal::{
        cell::{CellContent, CellStyle},
        screen::{Cursor, Screen},
    };

    #[test]
    fn create_screen_with_default_values() {
        let screen = Screen::new(2, 4);
        assert_eq!(screen.rows.len(), 2);
        assert!(screen.rows.iter().all(|row| row.len() == 4));
        assert_eq!(screen.cursor, Cursor::default());
        assert_eq!(screen.saved_cursor, Cursor::default());
    }

    #[test]
    fn two_screens_have_independent_content_and_cursor_positions() {
        let mut primary = Screen::new(2, 4);
        let mut alternate = Screen::new(2, 4);

        primary.cursor = Cursor { row: 0, col: 2 };
        primary.rows[0].write_narrow(0, 'A'.into(), CellStyle::default(), None);

        assert_eq!(alternate.cursor, Cursor::default());
        assert_eq!(alternate.rows[0].cell(0).content, CellContent::Empty);

        alternate.cursor = Cursor { row: 1, col: 3 };
        alternate.rows[1].write_narrow(0, 'B'.into(), CellStyle::default(), None);

        assert_eq!(primary.cursor, Cursor { row: 0, col: 2 });
        assert_eq!(
            primary.rows[0].cell(0).content,
            CellContent::Narrow("A".into())
        );
    }

    #[test]
    fn resize_changes_dimensions_and_clamps_cursors() {
        let mut screen = Screen::new(3, 4);
        screen.cursor = Cursor { row: 2, col: 3 };
        screen.saved_cursor = Cursor { row: 2, col: 3 };
        screen.resize(2, 2);
        assert_eq!(screen.cursor, Cursor { row: 1, col: 1 });
        assert_eq!(screen.saved_cursor, Cursor { row: 1, col: 1 });
        assert_eq!(screen.rows.len(), 2);
        assert!(screen.rows.iter().all(|row| row.len() == 2));
    }

    #[test]
    fn resize_preserves_cells_in_visible_intersection() {
        let mut screen = Screen::new(3, 4);
        screen.rows[0].write_narrow(1, "A".into(), CellStyle::default(), None);
        screen.resize(2, 2);
        assert_eq!(
            screen.rows[0].cell(1).content,
            CellContent::Narrow("A".into())
        );
    }

    #[test]
    fn save_and_restore_cursor() {
        let mut screen = Screen::new(3, 4);
        screen.cursor = Cursor { row: 0, col: 1 };
        screen.save_cursor();
        screen.cursor = Cursor { row: 1, col: 2 };
        screen.restore_cursor();
        assert_eq!(screen.cursor, Cursor { row: 0, col: 1 });
    }

    #[test]
    fn reset_cursor_preserves_saved_cursor() {
        let mut screen = Screen::new(3, 4);
        screen.cursor = Cursor { row: 0, col: 1 };
        screen.save_cursor();
        screen.reset_cursor();
        assert_eq!(screen.cursor, Cursor::default());
        screen.restore_cursor();
        assert_eq!(screen.cursor, Cursor { row: 0, col: 1 });
    }

    #[test]
    fn scroll_up_only_changes_rows_inside_region() {
        let mut screen = Screen::new(4, 2);
        for (row, ch) in ['A', 'B', 'C', 'D'].into_iter().enumerate() {
            screen.rows[row].write_narrow(0, ch.into(), CellStyle::default(), None);
        }
        screen.scroll_up(1..3);

        assert_eq!(
            screen.rows[0].cell(0).content,
            CellContent::Narrow("A".into())
        );
        assert_eq!(
            screen.rows[1].cell(0).content,
            CellContent::Narrow("C".into())
        );
        assert_eq!(screen.rows[2].cell(0).content, CellContent::Empty);
        assert_eq!(
            screen.rows[3].cell(0).content,
            CellContent::Narrow("D".into())
        );
    }

    #[test]
    fn new_screen_has_pending_wrap_false() {
        let screen = Screen::new(4, 2);
        assert!(!screen.pending_wrap);
    }

    #[test]
    fn reset_cursor_clears_pending_wrap() {
        let mut screen = Screen::new(4, 2);
        screen.pending_wrap = true;
        assert!(screen.pending_wrap);

        screen.reset_cursor();
        assert!(!screen.pending_wrap);
    }
}
