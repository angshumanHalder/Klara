use winit::{
    event::{ElementState, KeyEvent, Modifiers},
    keyboard::{Key, NamedKey},
};

pub enum Action {
    SendBytes(Vec<u8>),
    SplitVerticle,
    SplitHorizontal,
    None,
}

enum PrefixState {
    Idle,
    Waiting,
}

pub struct InputHandler {
    state: PrefixState,
    pub modifiers: Modifiers,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            state: PrefixState::Idle,
            modifiers: Modifiers::default(),
        }
    }

    pub fn handle(&mut self, event: &KeyEvent, app_cursor: bool) -> Action {
        if event.state != ElementState::Pressed {
            return Action::None;
        }

        let ctrl = self.modifiers.state().control_key();

        if ctrl {
            if let Key::Character(c) = &event.logical_key {
                if c.as_str() == "t" {
                    match self.state {
                        PrefixState::Idle => {
                            self.state = PrefixState::Waiting;
                            return Action::None;
                        }
                        PrefixState::Waiting => {
                            self.state = PrefixState::Idle;
                            return Action::SendBytes(vec![0x14]);
                        }
                    }
                }
            }
        }

        if matches!(self.state, PrefixState::Waiting) {
            self.state = PrefixState::Idle;
            if let Key::Character(c) = &event.logical_key {
                return match c.as_str() {
                    "s" => Action::SplitHorizontal,
                    "v" => Action::SplitVerticle,
                    _ => Action::None,
                };
            }
            return Action::None;
        }
        Action::SendBytes(translate_key(event, ctrl, app_cursor))
    }
}

fn translate_key(event: &KeyEvent, ctrl: bool, app_cursor: bool) -> Vec<u8> {
    if ctrl {
        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                if ch.is_ascii_alphabetic() {
                    return vec![(ch.to_ascii_lowercase() as u8) - b'a' + 1];
                }
            }
        }
    }

    if let Key::Named(named) = &event.logical_key {
        let seq: &[u8] = match named {
            NamedKey::Enter => b"\r",
            NamedKey::Backspace => b"\x7f",
            NamedKey::Tab => b"\t",
            NamedKey::Escape => b"\x1b",
            NamedKey::ArrowUp => {
                if app_cursor {
                    b"\x1bOA"
                } else {
                    b"\x1b[A"
                }
            }
            NamedKey::ArrowDown => {
                if app_cursor {
                    b"\x1bOB"
                } else {
                    b"\x1b[B"
                }
            }
            NamedKey::ArrowRight => {
                if app_cursor {
                    b"\x1bOC"
                } else {
                    b"\x1b[C"
                }
            }
            NamedKey::ArrowLeft => {
                if app_cursor {
                    b"\x1bOD"
                } else {
                    b"\x1b[D"
                }
            }
            NamedKey::Home => b"\x1b[H",
            NamedKey::End => b"\x1b[F",
            NamedKey::PageUp => b"\x1b[5~",
            NamedKey::PageDown => b"\x1b[6~",
            NamedKey::Insert => b"\x1b[2~",
            NamedKey::Delete => b"\x1b[3~",
            NamedKey::F1 => b"\x1bOP",
            NamedKey::F2 => b"\x1bOQ",
            NamedKey::F3 => b"\x1bOR",
            NamedKey::F4 => b"\x1bOS",
            NamedKey::F5 => b"\x1b[15~",
            NamedKey::F6 => b"\x1b[17~",
            NamedKey::F7 => b"\x1b[18~",
            NamedKey::F8 => b"\x1b[19~",
            NamedKey::F9 => b"\x1b[20~",
            NamedKey::F10 => b"\x1b[21~",
            NamedKey::F11 => b"\x1b[23~",
            NamedKey::F12 => b"\x1b[24~",
            _ => return vec![],
        };
        return seq.to_vec();
    }

    if let Some(text) = &event.text {
        return text.as_str().as_bytes().to_vec();
    }

    vec![]
}
