use vte::{Params, Perform};

#[derive(Debug, Clone)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: Color::Default,
            bg: Color::Default,
        }
    }
}

pub struct Grid {
    pub rows: usize,
    pub cols: usize,
    cells: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    fg: Color,
    bg: Color,
    alternate: Vec<Vec<Cell>>,
    pub in_alternate: bool,
    saved_cursor: (usize, usize),
    pub cursor_style: CursorStyle,
    pub cursor_visible: bool,
    pub application_cursor: bool,
    pub sgr_mouse: bool,
}

impl Grid {
    pub fn new(rows: usize, cols: usize) -> Self {
        Grid {
            rows,
            cols,
            cells: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            fg: Color::Default,
            bg: Color::Default,
            alternate: vec![vec![Cell::default(); cols]; rows],
            in_alternate: false,
            saved_cursor: (0, 0),
            cursor_style: CursorStyle::Block,
            application_cursor: false,
            sgr_mouse: false,
            cursor_visible: true,
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        &self.cells[row][col]
    }

    pub fn put_char(&mut self, ch: char) {
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col] = Cell {
                ch,
                fg: self.fg.clone(),
                bg: self.bg.clone(),
            }
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
        self.cells.push(vec![Cell::default(); self.cols]);
    }

    fn erase_line(&mut self, mode: u16) {
        let row = self.cursor_row;
        match mode {
            0 => (self.cursor_col..self.cols).for_each(|c| self.cells[row][c] = Cell::default()),
            1 => (0..=self.cursor_col).for_each(|c| self.cells[row][c] = Cell::default()),
            2 => (0..self.cols).for_each(|c| self.cells[row][c] = Cell::default()),
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                self.erase_line(0);
                for r in (self.cursor_row + 1)..self.rows {
                    for c in 0..self.cols {
                        self.cells[r][c] = Cell::default();
                    }
                }
            }
            1 => {
                for r in 0..self.cursor_row {
                    for c in 0..self.cols {
                        self.cells[r][c] = Cell::default();
                    }
                }
            }
            2 | 3 => {
                for r in 0..self.rows {
                    for c in 0..self.cols {
                        self.cells[r][c] = Cell::default();
                    }
                }
            }
            _ => {}
        }
    }

    fn enter_alternate_screen(&mut self) {
        self.saved_cursor = (self.cursor_row, self.cursor_col);
        std::mem::swap(&mut self.cells, &mut self.alternate);
        for row in &mut self.cells {
            row.fill(Cell::default());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.in_alternate = true;
    }

    fn leave_alternate_screen(&mut self) {
        std::mem::swap(&mut self.cells, &mut self.alternate);
        (self.cursor_row, self.cursor_col) = self.saved_cursor;
        self.in_alternate = false;
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
    }
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
    use super::*;
    use vte::Parser;

    #[test]
    fn test_print_places_char_at_cursor() {
        let mut grid = Grid::new(24, 80);
        grid.print('A');
        assert_eq!(grid.cell(0, 0).ch, 'A');
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
        assert_eq!(grid.cell(0, 0).ch, 'B');
        assert_eq!(grid.cell(1, 0).ch, 'C');
    }

    #[test]
    fn test_alternate_screen_switch() {
        let mut grid = Grid::new(24, 80);
        let mut parser = Parser::new();
        grid.print('A');
        assert_eq!(grid.cell(0, 0).ch, 'A');
        for &b in b"\x1b[?1049h" {
            parser.advance(&mut grid, b);
        }
        assert!(grid.in_alternate);
        assert_eq!(grid.cell(0, 0).ch, ' ');
        for &b in b"\x1b[?1049l" {
            parser.advance(&mut grid, b);
        }
        assert!(!grid.in_alternate);
        assert_eq!(grid.cell(0, 0).ch, 'A');
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
}
