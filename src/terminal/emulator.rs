use std::ops::Range;

use crate::terminal::cell::{CellFlags, CellStyle, HyperlinkId, UnderlineStyle};
use crate::terminal::mode::TerminalModes;
use crate::terminal::screen::{Cursor, Screen};

use super::TerminalError;

use super::cell::{Cell, Color};

use vte::{Params, ParamsIter, Perform};

#[derive(Debug, Clone, PartialEq)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenKind {
    Primary,
    Alternate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink {
    pub uri: String,
    pub osc_id: Option<String>,
}

pub struct Terminal {
    primary: Screen,
    alternate: Screen,
    active_screen: ScreenKind,
    current_style: CellStyle,
    active_hyperlink: Option<HyperlinkId>,
    hyperlinks: Vec<Hyperlink>,
    scroll_region: Range<usize>,
    modes: TerminalModes,
    tab_stops: Vec<bool>,

    pub rows: usize,
    pub cols: usize,
    pub cursor_style: CursorStyle,
    pub dirty: Vec<bool>,
}

impl Terminal {
    pub fn new(rows: usize, cols: usize) -> Self {
        Terminal {
            rows,
            cols,
            primary: Screen::new(rows, cols, 10_000),
            alternate: Screen::new(rows, cols, 0),
            active_screen: ScreenKind::Primary,
            current_style: CellStyle::default(),
            active_hyperlink: None,
            hyperlinks: Vec::new(),
            scroll_region: 0..rows,
            cursor_style: CursorStyle::Block,
            dirty: vec![true; rows],
            modes: TerminalModes::default(),
            tab_stops: (0..cols).map(|idx| idx != 0 && idx % 8 == 0).collect(),
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

        if new_cols > self.cols {
            let extension = (self.cols..new_cols).map(|idx| idx % 8 == 0);
            self.tab_stops.extend(extension);
        } else {
            self.tab_stops.truncate(new_cols);
        }

        self.primary.resize(new_rows, new_cols);
        self.alternate.resize(new_rows, new_cols);
        self.rows = new_rows;
        self.cols = new_cols;
        self.scroll_region = 0..new_rows;
        self.dirty = vec![true; new_rows];

        Ok(())
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        self.active_screen().cell(row, col)
    }

    pub fn put_char(&mut self, ch: char) {
        let style = self.current_style;
        let hyperlink = self.active_hyperlink;

        if self.active_screen().pending_wrap && self.modes.auto_wrap {
            self.active_screen_mut().pending_wrap = false;
            self.cursor_mut().col = 0;
            self.line_feed();
        } else if !self.modes.auto_wrap {
            self.active_screen_mut().pending_wrap = false;
        }

        let cursor = self.active_screen().cursor();
        let row = cursor.row;
        let col = cursor.col;

        if row < self.rows && col < self.cols {
            self.active_screen_mut().row_mut(row).write_narrow(
                col,
                ch.to_string(),
                style,
                hyperlink,
            );
            self.dirty[row] = true;
        }
        if col == self.cols - 1 {
            self.cursor_mut().col = col;
            self.active_screen_mut().pending_wrap = self.modes.auto_wrap;
        } else {
            self.cursor_mut().col += 1;
            self.active_screen_mut().pending_wrap = false;
        }
    }

    pub fn hyperlink(&self, id: HyperlinkId) -> Option<&str> {
        self.hyperlinks.get(id.0).map(|link| link.uri.as_str())
    }

    pub fn in_alternate_screen(&self) -> bool {
        self.active_screen == ScreenKind::Alternate
    }

    pub fn cursor_row(&self) -> usize {
        self.active_screen().cursor().row
    }

    pub fn cursor_col(&self) -> usize {
        self.active_screen().cursor().col
    }

    pub fn scroll_viewport_up(&mut self, lines: usize) {
        self.active_screen_mut().scroll_viewport_up(lines);
        self.dirty[0..self.rows].fill(true);
    }

    pub fn scroll_viewport_down(&mut self, lines: usize) {
        self.active_screen_mut().scroll_viewport_down(lines);
        self.dirty.fill(true);
    }

    fn scroll_up(&mut self) {
        let region = self.scroll_region.clone();
        self.active_screen_mut().scroll_up(region.clone());
        self.dirty[region].fill(true);
    }

    fn erase_line(&mut self, mode: u16) {
        let row = self.cursor_row();
        match mode {
            0 => (self.cursor_col()..self.cols).for_each(|c| self.clear_cell(row, c)),
            1 => (0..=self.cursor_col()).for_each(|c| self.clear_cell(row, c)),
            2 => (0..self.cols).for_each(|c| self.clear_cell(row, c)),
            _ => {}
        }
    }

    fn erase_display(&mut self, mode: u16) {
        match mode {
            0 => {
                self.erase_line(0);
                for r in (self.cursor_row() + 1)..self.rows {
                    let was_blank = self.active_screen().row(r).is_blank();
                    if was_blank {
                        continue;
                    }
                    self.active_screen_mut().row_mut(r).clear();
                    self.dirty[r] = true;
                }
            }
            1 => {
                for r in 0..self.cursor_row() {
                    let was_blank = self.active_screen().row(r).is_blank();
                    if was_blank {
                        continue;
                    }
                    self.active_screen_mut().row_mut(r).clear();
                    self.dirty[r] = true;
                }
                self.erase_line(1);
            }
            2 | 3 => {
                self.active_screen_mut().clear();
                self.dirty.fill(true);
            }
            _ => {}
        }
    }

    fn enter_alternate_screen(&mut self) {
        if self.active_screen == ScreenKind::Alternate {
            return;
        }
        self.primary.save_cursor();
        self.active_screen = ScreenKind::Alternate;
        self.alternate.clear();
        self.alternate.reset_cursor();
        self.dirty.fill(true);
    }

    fn leave_alternate_screen(&mut self) {
        if self.active_screen == ScreenKind::Primary {
            return;
        }
        self.active_screen = ScreenKind::Primary;
        self.primary.restore_cursor();
        self.dirty.fill(true);
    }

    fn apply_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();
        while let Some(param) = iter.next() {
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
        self.active_screen_mut().row_mut(row).clear_cell(col);
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

    fn active_screen(&self) -> &Screen {
        match self.active_screen {
            ScreenKind::Primary => &self.primary,
            ScreenKind::Alternate => &self.alternate,
        }
    }

    fn active_screen_mut(&mut self) -> &mut Screen {
        match self.active_screen {
            ScreenKind::Primary => &mut self.primary,
            ScreenKind::Alternate => &mut self.alternate,
        }
    }

    fn cursor_mut(&mut self) -> &mut Cursor {
        self.active_screen_mut().cursor_mut()
    }

    fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let mut top = top as usize;
        let mut bottom = bottom as usize;
        if top == 0 {
            top = 1;
        }
        if bottom == 0 {
            bottom = self.rows;
        }

        if top >= bottom {
            return;
        }
        if bottom > self.rows {
            return;
        }

        self.scroll_region = top - 1..bottom;
        self.active_screen_mut().reset_cursor();
    }

    fn line_feed(&mut self) {
        let rows = self.rows;
        let scroll_region = self.scroll_region.clone();
        let cursor = self.cursor_mut();
        if cursor.row == scroll_region.end - 1 {
            self.scroll_up();
        } else if cursor.row < rows - 1 {
            cursor.row += 1;
        }
    }

    pub fn application_cursor_keys(&self) -> bool {
        self.modes.application_cursor_keys
    }
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

impl Perform for Terminal {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0a..=0x0c => {
                self.line_feed();
            }
            0x0d => self.cursor_mut().col = 0,
            0x08 => {
                let cursor = self.cursor_mut();
                cursor.col = cursor.col.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p: Vec<u16> = params.iter().map(|p| p[0]).collect();
        let p0 = p.first().copied().unwrap_or(0);
        let p1 = p.get(1).copied().unwrap_or(0);
        match action {
            'A' => {
                let row = self.cursor_row().saturating_sub(p0.max(1) as usize);
                self.cursor_mut().row = row;
            }
            'B' => {
                let row = (self.cursor_row() + p0.max(1) as usize).min(self.rows - 1);
                self.cursor_mut().row = row;
            }
            'C' => {
                let col = (self.cursor_col() + p0.max(1) as usize).min(self.cols - 1);
                self.cursor_mut().col = col;
            }
            'D' => {
                let col = self.cursor_col().saturating_sub(p0.max(1) as usize);
                self.cursor_mut().col = col;
            }
            'G' => self.cursor_mut().col = (p0.saturating_sub(1) as usize).min(self.cols - 1),
            'H' | 'f' => {
                let rows = self.rows;
                let cols = self.cols;
                let cursor = self.cursor_mut();
                cursor.row = (p0.saturating_sub(1) as usize).min(rows - 1);
                cursor.col = (p1.saturating_sub(1) as usize).min(cols - 1);
            }
            'J' => self.erase_display(p0),
            'K' => self.erase_line(p0),
            'm' => self.apply_sgr(params),
            'h' if intermediates == [b'?'] => match p0 {
                7 => self.modes.auto_wrap = true,
                25 => self.modes.cursor_visible = true,
                1 => self.modes.application_cursor_keys = true,
                1006 => self.modes.sgr_mouse = true,
                1049 => self.enter_alternate_screen(),
                _ => {}
            },
            'l' if intermediates == [b'?'] => match p0 {
                7 => self.modes.auto_wrap = false,
                25 => self.modes.cursor_visible = false,
                1 => self.modes.application_cursor_keys = false,
                1006 => self.modes.sgr_mouse = false,
                1049 => self.leave_alternate_screen(),
                _ => {}
            },
            'q' if intermediates == [b' '] => {
                self.cursor_style = match p0 {
                    0..=2 => CursorStyle::Block,
                    3 | 4 => CursorStyle::Underline,
                    5 | 6 => CursorStyle::Bar,
                    _ => CursorStyle::Block,
                }
            }
            'r' if intermediates.is_empty() => {
                self.set_scroll_region(p0, p1);
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
