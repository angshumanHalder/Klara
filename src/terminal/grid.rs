use crate::terminal::row::Row;

use super::TerminalError;

use super::cell::{Cell, Color};

use vte::{Params, Perform};

#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    cells: Vec<Row>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    fg: Color,
    bg: Color,
    alternate: Vec<Row>,
    pub in_alternate: bool,
    saved_cursor: (usize, usize),
    pub cursor_style: CursorStyle,
    pub cursor_visible: bool,
    pub application_cursor: bool,
    pub sgr_mouse: bool,
    pub dirty: Vec<bool>,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        Grid {
            rows,
            cols,
            cells: new_buffer(rows, cols),
            cursor_row: 0,
            cursor_col: 0,
            fg: Color::Default,
            bg: Color::Default,
            alternate: new_buffer(rows, cols),
            in_alternate: false,
            saved_cursor: (0, 0),
            cursor_style: CursorStyle::Block,
            application_cursor: false,
            sgr_mouse: false,
            cursor_visible: true,
            dirty: vec![true; rows],
        }
    }

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) -> Result<(), TerminalError> {
        if new_rows == 0 || new_cols == 0 {
            return Err(TerminalError::InvalidSize {
                rows: new_rows,
                cols: new_cols,
            });
        }

        if new_rows == self.rows && new_cols == self.cols {
            return Ok(());
        }

        resize_buffer(&mut self.cells, new_rows, new_cols);
        resize_buffer(&mut self.alternate, new_rows, new_cols);

        self.rows = new_rows;
        self.cols = new_cols;

        self.cursor_row = self.cursor_row.min(new_rows - 1);
        self.cursor_col = self.cursor_col.min(new_cols - 1);

        self.saved_cursor.0 = self.saved_cursor.0.min(new_rows - 1);
        self.saved_cursor.1 = self.saved_cursor.1.min(new_cols - 1);

        self.dirty = vec![true; new_rows];

        Ok(())
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        self.cells[row].cell(col)
    }

    pub fn put_char(&mut self, ch: char) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row].write_narrow(
                self.cursor_col,
                ch.to_string(),
                self.fg.clone(),
                self.bg.clone(),
            );
            self.dirty[self.cursor_row] = true;
        }
        self.cursor_col += 1;
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= self.rows {
                self.scroll_up();
                self.cursor_row = self.rows - 1;
            }
        }
    }

    fn scroll_up(&mut self) {
        self.cells.remove(0);
        self.cells.push(Row::new(self.cols));
        self.dirty.fill(true);
    }

    fn erase_line(&mut self, mode: u16) {
        let row = self.cursor_row;
        match mode {
            0 => (self.cursor_col..self.cols).for_each(|c| self.clear_cell(row, c)),
            1 => (0..=self.cursor_col).for_each(|c| self.clear_cell(row, c)),
            2 => (0..self.cols).for_each(|c| self.clear_cell(row, c)),
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                self.erase_line(0);
                for r in (self.cursor_row + 1)..self.rows {
                    let was_blank = self.cells[r].is_blank();
                    if was_blank {
                        continue;
                    }
                    self.cells[r].clear();
                    self.dirty[r] = true;
                }
            }
            1 => {
                for r in 0..self.cursor_row {
                    let was_blank = self.cells[r].is_blank();
                    if was_blank {
                        continue;
                    }
                    self.cells[r].clear();
                    self.dirty[r] = true;
                }
                self.erase_line(1);
            }
            2 | 3 => {
                for r in 0..self.rows {
                    self.cells[r].clear();
                }
                self.dirty.fill(true);
            }
            _ => {}
        }
    }

    fn enter_alternate_screen(&mut self) {
        self.saved_cursor = (self.cursor_row, self.cursor_col);
        std::mem::swap(&mut self.cells, &mut self.alternate);
        for row in &mut self.cells {
            row.clear();
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.in_alternate = true;
        self.dirty.fill(true);
    }

    fn leave_alternate_screen(&mut self) {
        std::mem::swap(&mut self.cells, &mut self.alternate);
        (self.cursor_row, self.cursor_col) = self.saved_cursor;
        self.in_alternate = false;
        self.dirty.fill(true);
    }

    fn apply_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();
        loop {
            let Some(param) = iter.next() else { break };
            match param[0] {
                0 => {
                    self.fg = Color::Default;
                    self.bg = Color::Default
                }
                30..=37 => self.fg = Color::Indexed(param[0] as u8 - 30),
                38 => {
                    let next = iter.next().map(|p| p[0]).unwrap_or(0);
                    match next {
                        5 => self.fg = Color::Indexed(iter.next().map(|p| p[0] as u8).unwrap_or(0)),
                        2 => {
                            let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            self.fg = Color::Rgb(r, g, b);
                        }
                        _ => {}
                    }
                }
                39 => self.fg = Color::Default,
                40..=47 => self.bg = Color::Indexed(param[0] as u8 - 40),
                48 => {
                    let next = iter.next().map(|p| p[0]).unwrap_or(0);
                    match next {
                        5 => self.bg = Color::Indexed(iter.next().map(|p| p[0] as u8).unwrap_or(0)),
                        2 => {
                            let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            self.bg = Color::Rgb(r, g, b);
                        }
                        _ => {}
                    }
                }
                49 => self.bg = Color::Default,
                90..=97 => self.fg = Color::Indexed(param[0] as u8 - 90 + 8),
                100..=107 => self.bg = Color::Indexed(param[0] as u8 - 100 + 8),
                _ => {}
            }
        }
        self.dirty.fill(true);
    }

    fn clear_cell(&mut self, row: usize, col: usize) {
        self.cells[row].clear_cell(col);
        self.dirty[row] = true;
    }
}

fn new_buffer(rows: usize, cols: usize) -> Vec<Row> {
    (0..rows).map(|_| Row::new(cols)).collect()
}

fn resize_buffer(buffer: &mut Vec<Row>, new_rows: usize, new_cols: usize) {
    buffer.truncate(new_rows);

    for row in buffer.iter_mut() {
        row.resize(new_cols);
    }

    buffer.resize_with(new_rows, || Row::new(new_cols));
}

impl Perform for Grid {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0a | 0x0b | 0x0c => {
                self.cursor_row += 1;
                if self.cursor_row >= self.rows {
                    self.scroll_up();
                    self.cursor_row = self.rows - 1;
                }
            }
            0x0d => self.cursor_col = 0,
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p: Vec<u16> = params.iter().map(|p| p[0]).collect();
        let p0 = p.first().copied().unwrap_or(0);
        let p1 = p.get(1).copied().unwrap_or(0);
        match action {
            'A' => self.cursor_row = self.cursor_row.saturating_sub(p0.max(1) as usize),
            'B' => self.cursor_row = (self.cursor_row + p0.max(1) as usize).min(self.rows - 1),
            'C' => self.cursor_col = (self.cursor_col + p0.max(1) as usize).min(self.cols - 1),
            'D' => self.cursor_col = self.cursor_col.saturating_sub(p0.max(1) as usize),
            'G' => self.cursor_col = (p0.saturating_sub(1) as usize).min(self.cols - 1),
            'H' | 'f' => {
                self.cursor_row = (p0.saturating_sub(1) as usize).min(self.rows - 1);
                self.cursor_col = (p1.saturating_sub(1) as usize).min(self.cols - 1);
            }
            'J' => self.erase_display(p0),
            'K' => self.erase_line(p0),
            'm' => self.apply_sgr(params),
            'h' if intermediates == [b'?'] => match p0 {
                25 => self.cursor_visible = true,
                1 => self.application_cursor = true,
                1006 => self.sgr_mouse = true,
                1049 => self.enter_alternate_screen(),
                _ => {}
            },
            'l' if intermediates == [b'?'] => match p0 {
                25 => self.cursor_visible = false,
                1 => self.application_cursor = false,
                1006 => self.sgr_mouse = false,
                1049 => self.leave_alternate_screen(),
                _ => {}
            },
            'q' if intermediates == [b' '] => {
                self.cursor_style = match p0 {
                    0 | 1 | 2 => CursorStyle::Block,
                    3 | 4 => CursorStyle::Underline,
                    5 | 6 => CursorStyle::Bar,
                    _ => CursorStyle::Block,
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

#[cfg(test)]
mod test {
    use crate::terminal::cell::CellContent;

    use super::*;
    use vte::Parser;

    #[test]
    fn test_print_places_char_at_cursor() {
        let mut grid = Grid::new(24, 80);
        grid.print('A');
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
        assert_eq!(grid.cursor_col, 1);
    }

    #[test]
    fn test_lf_moves_cursor_down() {
        let mut grid = Grid::new(24, 80);
        grid.execute(0x0a);
        assert_eq!(grid.cursor_row, 1);
        assert_eq!(grid.cursor_col, 0);
    }

    #[test]
    fn test_cr_resets_col() {
        let mut grid = Grid::new(24, 80);
        grid.cursor_col = 10;
        grid.execute(0x0d);
        assert_eq!(grid.cursor_col, 0);
    }

    #[test]
    fn test_sgr_sets_fg_color() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        // \x1b[32m - green foreground
        for &b in b"\x1b[32m" {
            parser.advance(&mut grid, b);
        }
        grid.print('X');
        let cell = grid.cell(0, 0);
        assert!(matches!(cell.fg, Color::Indexed(2)));
    }

    #[test]
    fn test_sgr_resets_clears_color() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        for &b in b"\x1b[32m\x1b[0m" {
            parser.advance(&mut grid, b);
        }

        grid.print('X');
        assert!(matches!(grid.cell(0, 0).fg, Color::Default));
    }

    #[test]
    fn test_cursor_movement() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        // \x1b[5;10H - move to row 5 col 10
        for &b in b"\x1b[5;10H" {
            parser.advance(&mut grid, b);
        }

        assert_eq!(grid.cursor_row, 4);
        assert_eq!(grid.cursor_col, 9);
    }

    #[test]
    fn test_scroll_up_on_overflow() {
        let mut grid = Grid::new(3, 80);
        grid.print('A');
        grid.execute(0x0d);
        grid.execute(0x0a);
        grid.print('B');
        grid.execute(0x0d);
        grid.execute(0x0a);
        grid.print('C');
        grid.execute(0x0d);
        grid.execute(0x0a);
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("B".into()));
        assert_eq!(grid.cell(1, 0).content, CellContent::Narrow("C".into()));
    }

    #[test]
    fn test_alternate_screen_switch() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        grid.print('A');
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
        for &b in b"\x1b[?1049h" {
            parser.advance(&mut grid, b);
        }
        assert!(grid.in_alternate);
        assert_eq!(grid.cell(0, 0).content, CellContent::Empty);
        for &b in b"\x1b[?1049l" {
            parser.advance(&mut grid, b);
        }
        assert!(!grid.in_alternate);
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
    }

    #[test]
    fn test_alternate_screen_restores_cursor() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        grid.cursor_row = 5;
        grid.cursor_col = 10;
        for &b in b"\x1b[?1049h" {
            parser.advance(&mut grid, b);
        }
        assert_eq!(grid.cursor_row, 0);
        assert_eq!(grid.cursor_col, 0);
        for &b in b"\x1b[?1049l" {
            parser.advance(&mut grid, b);
        }
        assert_eq!(grid.cursor_row, 5);
        assert_eq!(grid.cursor_col, 10);
    }

    #[test]
    fn test_decscusr_set_cursor_style() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        for &b in b"\x1b[4 q" {
            parser.advance(&mut grid, b);
        }
        assert_eq!(grid.cursor_style, CursorStyle::Underline);
        for &b in b"\x1b[2 q" {
            parser.advance(&mut grid, b);
        }
        assert_eq!(grid.cursor_style, CursorStyle::Block);
    }

    #[test]
    fn resize_grows_grid_and_preserves_cells() {
        let mut grid = Grid::new(2, 3);

        grid.put_char('A');

        grid.cursor_row = 1;
        grid.cursor_col = 1;
        grid.put_char('B');

        grid.resize(4, 5).unwrap();

        assert_eq!(grid.rows, 4);
        assert_eq!(grid.cols, 5);
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
        assert_eq!(grid.cell(1, 1).content, CellContent::Narrow("B".into()));
        assert_eq!(grid.cell(3, 4), &Cell::default());
        assert_eq!(grid.dirty, vec![true; 4]);
    }

    #[test]
    fn resize_shrinks_grid_and_clamps_cursor() {
        let mut grid = Grid::new(4, 5);
        grid.cursor_row = 3;
        grid.cursor_col = 4;

        grid.resize(2, 3).unwrap();

        assert_eq!(grid.rows, 2);
        assert_eq!(grid.cols, 3);
        assert_eq!(grid.cursor_row, 1);
        assert_eq!(grid.cursor_col, 2);
        assert_eq!(grid.dirty, vec![true; 2]);
    }

    #[test]
    fn resize_preserves_visible_intersection_when_shrinking() {
        let mut grid = Grid::new(3, 4);
        grid.cursor_row = 1;
        grid.cursor_col = 2;
        grid.put_char('X');

        grid.resize(2, 3).unwrap();

        assert_eq!(grid.cell(1, 2).content, CellContent::Narrow("X".into()));
    }

    #[test]
    fn resize_updates_primary_and_alternate_buffers() {
        let mut grid = Grid::new(2, 3);
        let mut parser = Parser::new();

        grid.put_char('P');

        for &byte in b"\x1b[?1049h" {
            parser.advance(&mut grid, byte);
        }

        grid.put_char('A');
        grid.resize(4, 5).unwrap();

        assert_eq!(grid.rows, 4);
        assert_eq!(grid.cols, 5);
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
        assert_eq!(grid.cell(3, 4), &Cell::default());

        for &byte in b"\x1b[?1049l" {
            parser.advance(&mut grid, byte);
        }

        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("P".into()));
        assert_eq!(grid.cell(3, 4), &Cell::default());
    }

    #[test]
    fn resize_rejects_zero_dimensions_without_mutating_grid() {
        let mut grid = Grid::new(2, 3);
        grid.put_char('A');

        let error = grid.resize(0, 3).unwrap_err();

        assert_eq!(error, TerminalError::InvalidSize { rows: 0, cols: 3 });
        assert_eq!(grid.rows, 2);
        assert_eq!(grid.cols, 3);
        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
    }

    #[test]
    fn resizing_to_same_dimensions_is_a_noop() {
        let mut grid = Grid::new(2, 3);
        grid.dirty.fill(false);

        grid.resize(2, 3).unwrap();

        assert_eq!(grid.dirty, vec![false; 2]);
    }

    #[test]
    fn put_char_creates_narrow_content() {
        let mut grid = Grid::new(2, 2);
        grid.put_char('A');

        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("A".into()));
    }

    #[test]
    fn erasing_wide_continuation_erases_wide_leading() {
        let mut grid = Grid::new(2, 4);
        grid.cells[0]
            .write_wide(1, "界".into(), Color::Default, Color::Default)
            .unwrap();

        grid.cursor_row = 0;
        grid.cursor_col = 2;

        grid.erase_line(0);

        assert_eq!(grid.cell(0, 1).content, CellContent::Empty);
        assert_eq!(grid.cell(0, 2).content, CellContent::Empty);
    }

    #[test]
    fn erasing_wide_leading_erases_wide_continuation() {
        let mut grid = Grid::new(2, 4);
        grid.cells[0]
            .write_wide(1, "界".into(), Color::Default, Color::Default)
            .unwrap();

        grid.cursor_row = 0;
        grid.cursor_col = 1;

        grid.erase_line(1);

        assert_eq!(grid.cell(0, 1).content, CellContent::Empty);
        assert_eq!(grid.cell(0, 2).content, CellContent::Empty);
    }

    #[test]
    fn resize_clears_wide_leading_when_continuation_is_truncated() {
        let mut grid = Grid::new(1, 3);

        grid.cells[0]
            .write_wide(1, "界".into(), Color::Default, Color::Default)
            .unwrap();

        grid.resize(1, 2).unwrap();

        assert_eq!(grid.rows, 1);
        assert_eq!(grid.cols, 2);
        assert_eq!(grid.cell(0, 1).content, CellContent::Empty);
    }

    #[test]
    fn resize_clears_wide_leading_when_continuation_is_truncated_in_alternate() {
        let mut grid = Grid::new(1, 3);
        grid.cells[0].write_narrow(0, "P".into(), Color::Default, Color::Default);

        grid.enter_alternate_screen();

        grid.cells[0]
            .write_wide(1, "界".into(), Color::Default, Color::Default)
            .unwrap();

        grid.resize(1, 2).unwrap();

        assert_eq!(grid.rows, 1);
        assert_eq!(grid.cols, 2);
        assert_eq!(grid.cell(0, 1).content, CellContent::Empty);

        grid.leave_alternate_screen();

        assert_eq!(grid.cell(0, 0).content, CellContent::Narrow("P".into()));
    }
}
