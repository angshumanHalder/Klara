use crate::terminal::cell::{CellFlags, CellStyle, HyperlinkId, UnderlineStyle};
use crate::terminal::row::Row;

use super::TerminalError;

use super::cell::{Cell, Color};

use vte::{Params, ParamsIter, Perform};

#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink {
    pub uri: String,
    pub osc_id: Option<String>,
}

pub struct Grid {
    saved_cursor: (usize, usize),
    cells: Vec<Row>,
    current_style: CellStyle,
    alternate: Vec<Row>,
    active_hyperlink: Option<HyperlinkId>,
    hyperlinks: Vec<Hyperlink>,

    pub rows: usize,
    pub cols: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub in_alternate: bool,
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
            current_style: CellStyle::default(),
            alternate: new_buffer(rows, cols),
            in_alternate: false,
            active_hyperlink: None,
            hyperlinks: Vec::new(),
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
                self.current_style,
                self.active_hyperlink,
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

    pub fn hyperlink(&self, id: HyperlinkId) -> Option<&str> {
        self.hyperlinks.get(id.0).map(|link| link.uri.as_str())
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
                    self.current_style = CellStyle::default();
                }
                1 => self.current_style.flags.insert(CellFlags::BOLD),
                2 => self.current_style.flags.insert(CellFlags::DIM),
                3 => self.current_style.flags.insert(CellFlags::ITALIC),
                4 => {
                    let style: Option<UnderlineStyle> = match param.get(1).copied() {
                        None | Some(1) => Some(UnderlineStyle::Single),
                        Some(0) => Some(UnderlineStyle::None),
                        Some(2) => Some(UnderlineStyle::Double),
                        Some(3) => Some(UnderlineStyle::Curly),
                        Some(4) => Some(UnderlineStyle::Dotted),
                        Some(5) => Some(UnderlineStyle::Dashed),
                        _ => None,
                    };
                    if let Some(style) = style {
                        self.current_style.underline_style = style;
                    }
                }
                5 | 6 => self.current_style.flags.insert(CellFlags::BLINK),
                7 => self.current_style.flags.insert(CellFlags::REVERSE),
                8 => self.current_style.flags.insert(CellFlags::HIDDEN),
                9 => self.current_style.flags.insert(CellFlags::STRIKEOUT),
                22 => {
                    self.current_style.flags.remove(CellFlags::BOLD);
                    self.current_style.flags.remove(CellFlags::DIM);
                }
                23 => self.current_style.flags.remove(CellFlags::ITALIC),
                24 => self.current_style.underline_style = UnderlineStyle::None,
                25 => self.current_style.flags.remove(CellFlags::BLINK),
                27 => self.current_style.flags.remove(CellFlags::REVERSE),
                28 => self.current_style.flags.remove(CellFlags::HIDDEN),
                29 => self.current_style.flags.remove(CellFlags::STRIKEOUT),
                30..=37 => self.current_style.fg = Color::Indexed(param[0] as u8 - 30),
                38 => {
                    let next = iter.next().map(|p| p[0]).unwrap_or(0);
                    match next {
                        5 => {
                            self.current_style.fg =
                                Color::Indexed(iter.next().map(|p| p[0] as u8).unwrap_or(0))
                        }
                        2 => {
                            let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            self.current_style.fg = Color::Rgb(r, g, b);
                        }
                        _ => {}
                    }
                }
                39 => self.current_style.fg = Color::Default,
                40..=47 => self.current_style.bg = Color::Indexed(param[0] as u8 - 40),
                48 => {
                    let next = iter.next().map(|p| p[0]).unwrap_or(0);
                    match next {
                        5 => {
                            self.current_style.bg =
                                Color::Indexed(iter.next().map(|p| p[0] as u8).unwrap_or(0))
                        }
                        2 => {
                            let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                            self.current_style.bg = Color::Rgb(r, g, b);
                        }
                        _ => {}
                    }
                }
                49 => self.current_style.bg = Color::Default,
                58 => {
                    if let Some(color) = parse_sgr_color(param, &mut iter) {
                        self.current_style.underline_color = Some(color);
                    }
                }
                59 => self.current_style.underline_color = None,
                90..=97 => self.current_style.fg = Color::Indexed(param[0] as u8 - 90 + 8),
                100..=107 => self.current_style.bg = Color::Indexed(param[0] as u8 - 100 + 8),
                _ => {}
            }
        }
        self.dirty.fill(true);
    }

    fn clear_cell(&mut self, row: usize, col: usize) {
        self.cells[row].clear_cell(col);
        self.dirty[row] = true;
    }

    fn open_hyperlink(&mut self, uri: String, osc_id: Option<String>) {
        let hyperlink = Hyperlink { uri, osc_id };
        let id = self
            .hyperlinks
            .iter()
            .position(|stored| stored == &hyperlink)
            .unwrap_or_else(|| {
                self.hyperlinks.push(hyperlink);
                self.hyperlinks.len() - 1
            });
        self.active_hyperlink = Some(HyperlinkId(id));
    }

    fn close_hyperlink(&mut self) {
        self.active_hyperlink = None;
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

fn parse_sgr_color(param: &[u16], iter: &mut ParamsIter<'_>) -> Option<Color> {
    if param.len() > 1 {
        match param {
            [_, 5, index] => Some(Color::Indexed(u8::try_from(*index).ok()?)),
            [_, 2, red, green, blue] => Some(Color::Rgb(
                u8::try_from(*red).ok()?,
                u8::try_from(*green).ok()?,
                u8::try_from(*blue).ok()?,
            )),
            _ => None,
        }
    } else {
        let mode = iter.next().map(|p| p[0]).unwrap_or(0);
        match mode {
            5 => Some(Color::Indexed(next_sgr_byte(iter.next())?)),
            2 => Some(Color::Rgb(
                next_sgr_byte(iter.next())?,
                next_sgr_byte(iter.next())?,
                next_sgr_byte(iter.next())?,
            )),
            _ => None,
        }
    }
}

fn next_sgr_byte(param: Option<&[u16]>) -> Option<u8> {
    let value = param?.first().copied()?;
    u8::try_from(value).ok()
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

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.first() != Some(&b"8".as_slice()) || params.len() < 3 {
            return;
        }
        let osc_id = params[1]
            .split(|byte| *byte == b':')
            .find_map(|entry| entry.strip_prefix(b"id="))
            .and_then(|id| std::str::from_utf8(id).ok())
            .map(str::to_owned);
        let Some(uri) = parse_osc_uri(params) else {
            return;
        };

        if uri.is_empty() {
            self.close_hyperlink();
        } else {
            self.open_hyperlink(uri, osc_id);
        }
    }
}

fn parse_osc_uri(params: &[&[u8]]) -> Option<String> {
    let mut uri = String::from_utf8(params[2].to_vec()).ok()?;
    for part in &params[3..] {
        uri.push(';');
        uri.push_str(std::str::from_utf8(part).ok()?);
    }
    Some(uri)
}

#[cfg(test)]
mod tests;
