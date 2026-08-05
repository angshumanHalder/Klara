pub(super) struct TerminalModes {
    pub(super) auto_wrap: bool,
    pub(super) cursor_visible: bool,
    pub(super) application_cursor_keys: bool,
    pub(super) sgr_mouse: bool,
}

impl Default for TerminalModes {
    fn default() -> Self {
        Self {
            auto_wrap: true,
            cursor_visible: true,
            application_cursor_keys: false,
            sgr_mouse: false,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::terminal::mode::TerminalModes;

    #[test]
    fn tests_default_values() {
        let modes = TerminalModes::default();
        assert!(modes.auto_wrap);
        assert!(modes.cursor_visible);
        assert!(!modes.application_cursor_keys);
        assert!(!modes.sgr_mouse);
    }
}
