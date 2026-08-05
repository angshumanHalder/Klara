use klara::{
    config::Config,
    terminal::{
        CursorStyle, Terminal,
        cell::{Cell, CellContent, Color},
    },
};

#[test]
fn default_configuration_is_valid() {
    let config = Config::default();

    assert!(config.window.width > 0);
    assert!(config.window.height > 0);
    assert!((0.0..=1.0).contains(&config.window.opacity));
}

#[test]
fn new_grid_has_valid_initial_state() {
    let terminal = Terminal::new(24, 80);

    assert_eq!(terminal.rows, 24);
    assert_eq!(terminal.cols, 80);
    assert_eq!(terminal.cursor_row(), 0);
    assert_eq!(terminal.cursor_col(), 0);
    assert_eq!(terminal.cursor_style, CursorStyle::Block);
    assert!(!terminal.in_alternate_screen());

    for row in 0..terminal.rows {
        for col in 0..terminal.cols {
            assert_eq!(terminal.cell(row, col), &Cell::default());
        }
    }
}

#[test]
fn default_cell_has_no_explicit_colors() {
    let cell = Cell::default();

    assert_eq!(cell.content, CellContent::Empty);
    assert_eq!(cell.style.fg, Color::Default);
    assert_eq!(cell.style.bg, Color::Default);
}

#[test]
fn configuration_errors_convert_to_application_errors() {
    use std::io;

    use klara::{config::ConfigError, error::KlaraError};

    let config_error = ConfigError::Read {
        path: "missing.toml".into(),
        source: io::Error::new(io::ErrorKind::NotFound, "test error"),
    };

    let application_error = KlaraError::from(config_error);

    assert!(matches!(
        application_error,
        KlaraError::Config(ConfigError::Read { .. })
    ));
}
