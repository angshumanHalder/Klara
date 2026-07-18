use klara::{
    config::Config,
    terminal::grid::{Cell, Color, CursorStyle, Grid},
};

#[test]
fn default_configuration_is_valid() {
    let config = Config::default();

    assert!(!config.font.family.trim().is_empty());
    assert!(config.font.size > 0.0);
    assert!(config.window.width > 0);
    assert!(config.window.height > 0);
    assert!((0.0..=1.0).contains(&config.window.opacity));
}

#[test]
fn new_grid_has_valid_initial_state() {
    let grid = Grid::new(24, 80);

    assert_eq!(grid.rows, 24);
    assert_eq!(grid.cols, 80);
    assert_eq!(grid.cursor_row, 0);
    assert_eq!(grid.cursor_col, 0);
    assert_eq!(grid.cursor_style, CursorStyle::Block);
    assert!(grid.cursor_visible);
    assert!(!grid.in_alternate);
    assert!(!grid.application_cursor);
    assert!(!grid.sgr_mouse);

    for row in 0..grid.rows {
        for col in 0..grid.cols {
            assert_eq!(grid.cell(row, col), &Cell::default());
        }
    }
}

#[test]
fn default_cell_has_no_explicit_colors() {
    let cell = Cell::default();

    assert_eq!(cell.ch, ' ');
    assert_eq!(cell.fg, Color::Default);
    assert_eq!(cell.bg, Color::Default);
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
