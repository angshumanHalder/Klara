use crate::terminal::cell::{CellContent, CellFlags};

use super::*;
use vte::Parser;

#[test]
fn test_print_places_char_at_cursor() {
    let mut term = Terminal::new(24, 80);
    term.print('A');
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
    assert_eq!(term.cursor_col(), 1);
}

#[test]
fn test_lf_moves_cursor_down() {
    let mut term = Terminal::new(24, 80);
    term.execute(0x0a);
    assert_eq!(term.cursor_row(), 1);
    assert_eq!(term.cursor_col(), 0);
}

#[test]
fn test_cr_resets_col() {
    let mut term = Terminal::new(24, 80);
    term.cursor_mut().col = 10;
    term.execute(0x0d);
    assert_eq!(term.cursor_col(), 0);
}

#[test]
fn test_sgr_sets_fg_color() {
    let mut term = Terminal::new(24, 80);
    let mut parser = Parser::new();
    // \x1b[32m - green foreground
    for &b in b"\x1b[32m" {
        parser.advance(&mut term, b);
    }
    term.print('X');
    let cell = term.cell(0, 0);
    assert!(matches!(cell.style.fg, Color::Indexed(2)));
}

#[test]
fn test_sgr_resets_clears_color() {
    let mut term = Terminal::new(24, 80);
    let mut parser = Parser::new();
    for &b in b"\x1b[32m\x1b[0m" {
        parser.advance(&mut term, b);
    }

    term.print('X');
    assert!(matches!(term.cell(0, 0).style.fg, Color::Default));
}

#[test]
fn test_cursor_movement() {
    let mut term = Terminal::new(24, 80);
    let mut parser = Parser::new();
    // \x1b[5;10H - move to row 5 col 10
    for &b in b"\x1b[5;10H" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.cursor_row(), 4);
    assert_eq!(term.cursor_col(), 9);
}

#[test]
fn test_scroll_up_on_overflow() {
    let mut term = Terminal::new(3, 80);
    term.print('A');
    term.execute(0x0d);
    term.execute(0x0a);
    term.print('B');
    term.execute(0x0d);
    term.execute(0x0a);
    term.print('C');
    term.execute(0x0d);
    term.execute(0x0a);
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("B".into()));
    assert_eq!(term.cell(1, 0).content, CellContent::Narrow("C".into()));
}

#[test]
fn test_alternate_screen_switch() {
    let mut term = Terminal::new(24, 80);
    let mut parser = Parser::new();
    term.print('A');
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
    for &b in b"\x1b[?1049h" {
        parser.advance(&mut term, b);
    }
    assert!(term.in_alternate_screen());
    assert_eq!(term.cell(0, 0).content, CellContent::Empty);
    for &b in b"\x1b[?1049l" {
        parser.advance(&mut term, b);
    }
    assert!(!term.in_alternate_screen());
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
}

#[test]
fn test_alternate_screen_restores_cursor() {
    let mut term = Terminal::new(24, 80);
    let mut parser = Parser::new();
    let cursor = term.cursor_mut();
    cursor.row = 5;
    cursor.col = 10;
    for &b in b"\x1b[?1049h" {
        parser.advance(&mut term, b);
    }
    assert_eq!(term.cursor_row(), 0);
    assert_eq!(term.cursor_col(), 0);
    for &b in b"\x1b[?1049l" {
        parser.advance(&mut term, b);
    }
    assert_eq!(term.cursor_row(), 5);
    assert_eq!(term.cursor_col(), 10);
}

#[test]
fn test_decscusr_set_cursor_style() {
    let mut term = Terminal::new(24, 80);
    let mut parser = Parser::new();
    for &b in b"\x1b[4 q" {
        parser.advance(&mut term, b);
    }
    assert_eq!(term.cursor_style, CursorStyle::Underline);
    for &b in b"\x1b[2 q" {
        parser.advance(&mut term, b);
    }
    assert_eq!(term.cursor_style, CursorStyle::Block);
}

#[test]
fn resize_grows_term_and_preserves_cells() {
    let mut term = Terminal::new(2, 3);

    term.put_char('A');

    let cursor = term.cursor_mut();
    cursor.row = 1;
    cursor.col = 1;
    term.put_char('B');

    term.resize(4, 5).unwrap();

    assert_eq!(term.rows, 4);
    assert_eq!(term.cols, 5);
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
    assert_eq!(term.cell(1, 1).content, CellContent::Narrow("B".into()));
    assert_eq!(term.cell(3, 4), &Cell::default());
    assert_eq!(term.dirty, vec![true; 4]);
}

#[test]
fn resize_shrinks_term_and_clamps_cursor() {
    let mut term = Terminal::new(4, 5);
    let cursor = term.cursor_mut();
    cursor.row = 3;
    cursor.col = 4;

    term.resize(2, 3).unwrap();

    assert_eq!(term.rows, 2);
    assert_eq!(term.cols, 3);
    assert_eq!(term.cursor_row(), 1);
    assert_eq!(term.cursor_col(), 2);
    assert_eq!(term.dirty, vec![true; 2]);
}

#[test]
fn resize_preserves_visible_intersection_when_shrinking() {
    let mut term = Terminal::new(3, 4);
    let cursor = term.cursor_mut();
    cursor.row = 1;
    cursor.col = 2;
    term.put_char('X');

    term.resize(2, 3).unwrap();

    assert_eq!(term.cell(1, 2).content, CellContent::Narrow("X".into()));
}

#[test]
fn resize_updates_primary_and_alternate_buffers() {
    let mut term = Terminal::new(2, 3);
    let mut parser = Parser::new();

    term.put_char('P');

    for &byte in b"\x1b[?1049h" {
        parser.advance(&mut term, byte);
    }

    term.put_char('A');
    term.resize(4, 5).unwrap();

    assert_eq!(term.rows, 4);
    assert_eq!(term.cols, 5);
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
    assert_eq!(term.cell(3, 4), &Cell::default());

    for &byte in b"\x1b[?1049l" {
        parser.advance(&mut term, byte);
    }

    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("P".into()));
    assert_eq!(term.cell(3, 4), &Cell::default());
}

#[test]
fn resize_rejects_zero_dimensions_without_mutating_term() {
    let mut term = Terminal::new(2, 3);
    term.put_char('A');

    let error = term.resize(0, 3).unwrap_err();

    assert_eq!(error, TerminalError::InvalidSize { rows: 0, cols: 3 });
    assert_eq!(term.rows, 2);
    assert_eq!(term.cols, 3);
    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
}

#[test]
fn resizing_to_same_dimensions_is_a_noop() {
    let mut term = Terminal::new(2, 3);
    term.dirty.fill(false);

    term.resize(2, 3).unwrap();

    assert_eq!(term.dirty, vec![false; 2]);
}

#[test]
fn put_char_creates_narrow_content() {
    let mut term = Terminal::new(2, 2);
    term.put_char('A');

    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
}

#[test]
fn erasing_wide_continuation_erases_wide_leading() {
    let mut term = Terminal::new(2, 4);
    let style = CellStyle::default();
    term.active_screen_mut()
        .row_mut(0)
        .write_wide(1, "界".into(), style, None)
        .unwrap();

    let cursor = term.cursor_mut();
    cursor.row = 0;
    cursor.col = 2;

    term.erase_line(0);

    assert_eq!(term.cell(0, 1).content, CellContent::Empty);
    assert_eq!(term.cell(0, 2).content, CellContent::Empty);
}

#[test]
fn erasing_wide_leading_erases_wide_continuation() {
    let mut term = Terminal::new(2, 4);

    let style = CellStyle::default();
    term.active_screen_mut()
        .row_mut(0)
        .write_wide(1, "界".into(), style, None)
        .unwrap();

    let cursor = term.cursor_mut();
    cursor.row = 0;
    cursor.col = 1;

    term.erase_line(1);

    assert_eq!(term.cell(0, 1).content, CellContent::Empty);
    assert_eq!(term.cell(0, 2).content, CellContent::Empty);
}

#[test]
fn resize_clears_wide_leading_when_continuation_is_truncated() {
    let mut term = Terminal::new(1, 3);

    let style = CellStyle::default();

    term.active_screen_mut()
        .row_mut(0)
        .write_wide(1, "界".into(), style, None)
        .unwrap();

    term.resize(1, 2).unwrap();

    assert_eq!(term.rows, 1);
    assert_eq!(term.cols, 2);
    assert_eq!(term.cell(0, 1).content, CellContent::Empty);
}

#[test]
fn resize_clears_wide_leading_when_continuation_is_truncated_in_alternate() {
    let mut term = Terminal::new(1, 3);

    let style = CellStyle::default();
    term.active_screen_mut()
        .row_mut(0)
        .write_narrow(0, "P".into(), style, None);

    term.enter_alternate_screen();

    term.active_screen_mut()
        .row_mut(0)
        .write_wide(1, "界".into(), style, None)
        .unwrap();

    term.resize(1, 2).unwrap();

    assert_eq!(term.rows, 1);
    assert_eq!(term.cols, 2);
    assert_eq!(term.cell(0, 1).content, CellContent::Empty);

    term.leave_alternate_screen();

    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("P".into()));
}

#[test]
fn printed_cells_retain_style_after_current_style_changes() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    // \x1b[32m - green foreground
    for &b in b"\x1b[32m" {
        parser.advance(&mut term, b);
    }
    term.print('A');

    for &b in b"\x1b[0m" {
        parser.advance(&mut term, b);
    }
    term.print('B');

    assert!(matches!(term.cell(0, 0).style.fg, Color::Indexed(2)));
    assert!(matches!(term.cell(0, 1).style.fg, Color::Default));
}

#[test]
fn sgr_enables_single_attr() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    for &b in b"\x1b[1m" {
        parser.advance(&mut term, b);
    }
    term.print('A');
    assert!(term.cell(0, 0).style.flags.contains(CellFlags::BOLD));
}

#[test]
fn sgr_enables_multiple_attrs() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    for &b in b"\x1b[1;3;4m" {
        parser.advance(&mut term, b);
    }
    term.print('A');
    assert!(term.cell(0, 0).style.flags.contains(CellFlags::BOLD));
    assert!(term.cell(0, 0).style.flags.contains(CellFlags::ITALIC));
    assert_eq!(
        term.cell(0, 0).style.underline_style,
        UnderlineStyle::Single
    );
}

#[test]
fn sgr_selective_reset_preserves_other_attrs() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    for &b in b"\x1b[1;3;4m" {
        parser.advance(&mut term, b);
    }
    for &b in b"\x1b[23m" {
        parser.advance(&mut term, b);
    }
    term.print('A');
    assert!(term.cell(0, 0).style.flags.contains(CellFlags::BOLD));
    assert_eq!(
        term.cell(0, 0).style.underline_style,
        UnderlineStyle::Single
    );
    assert!(!term.cell(0, 0).style.flags.contains(CellFlags::ITALIC));
}

#[test]
fn sgr_22_resets_bold_and_dim() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    for &b in b"\x1b[1;2;4m" {
        parser.advance(&mut term, b);
    }
    for &b in b"\x1b[22m" {
        parser.advance(&mut term, b);
    }
    term.print('A');
    assert_eq!(
        term.cell(0, 0).style.underline_style,
        UnderlineStyle::Single
    );
    assert!(!term.cell(0, 0).style.flags.contains(CellFlags::BOLD));
    assert!(!term.cell(0, 0).style.flags.contains(CellFlags::DIM));
}

#[test]
fn sgr_zero_resets_entire_style() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    for &b in b"\x1b[31;1;3m" {
        parser.advance(&mut term, b);
    }
    for &b in b"\x1b[0m" {
        parser.advance(&mut term, b);
    }
    term.print('A');
    assert_eq!(term.cell(0, 0).style.fg, Color::Default);
    assert_eq!(term.cell(0, 0).style.bg, Color::Default);
    assert!(term.cell(0, 0).style.flags.is_empty());
}

#[test]
fn printed_cells_retain_flags_after_selective_reset() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();
    for &b in b"\x1b[1m" {
        parser.advance(&mut term, b);
    }
    term.print('A');
    for &b in b"\x1b[22m" {
        parser.advance(&mut term, b);
    }
    term.print('B');

    assert!(term.cell(0, 0).style.flags.contains(CellFlags::BOLD));
    assert!(!term.cell(0, 1).style.flags.contains(CellFlags::BOLD));
}

#[test]
fn sgr_4_parses_underline_invariants() {
    struct TestCase {
        sequence: &'static [u8],
        expected_style: UnderlineStyle,
    }

    let table = vec![
        TestCase {
            sequence: b"\x1b[4:1m",
            expected_style: UnderlineStyle::Single,
        },
        TestCase {
            sequence: b"\x1b[4:2m",
            expected_style: UnderlineStyle::Double,
        },
        TestCase {
            sequence: b"\x1b[4:3m",
            expected_style: UnderlineStyle::Curly,
        },
        TestCase {
            sequence: b"\x1b[4:4m",
            expected_style: UnderlineStyle::Dotted,
        },
        TestCase {
            sequence: b"\x1b[4:5m",
            expected_style: UnderlineStyle::Dashed,
        },
        TestCase {
            sequence: b"\x1b[4:0m",
            expected_style: UnderlineStyle::default(),
        },
        TestCase {
            sequence: b"\x1b[4m",
            expected_style: UnderlineStyle::Single,
        },
    ];

    for test in table {
        let mut term = Terminal::new(2, 8);
        let mut parser = Parser::new();
        for &b in test.sequence {
            parser.advance(&mut term, b);
        }
        term.print('A');
        assert_eq!(term.cell(0, 0).style.underline_style, test.expected_style);
    }
}

#[test]
fn sgr_24_resets_disables_underline_style() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();

    for &b in b"\x1b[4:1m" {
        parser.advance(&mut term, b);
    }

    term.print('A');

    for &b in b"\x1b[24m" {
        parser.advance(&mut term, b);
    }
    term.print('B');
    assert_eq!(term.cell(0, 1).style.underline_style, UnderlineStyle::None);
}

#[test]
fn test_unsupported_values() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();

    for &b in b"\x1b[4:3m" {
        parser.advance(&mut term, b);
    }
    for &b in b"\x1b[4:99m" {
        parser.advance(&mut term, b);
    }
    term.print('A');

    assert_eq!(term.cell(0, 0).style.underline_style, UnderlineStyle::Curly);
}

#[test]
fn test_underline_colors() {
    struct TestCase {
        sequence: &'static [u8],
        expected_color: Option<Color>,
    }

    let tests = vec![
        TestCase {
            sequence: b"\x1b[58;5;123m",
            expected_color: Some(Color::Indexed(123)),
        },
        TestCase {
            sequence: b"\x1b[58;2;10;20;30m",
            expected_color: Some(Color::Rgb(10, 20, 30)),
        },
        TestCase {
            sequence: b"\x1b[58:2:10:20:30m",
            expected_color: Some(Color::Rgb(10, 20, 30)),
        },
        TestCase {
            sequence: b"\x1b[58:5:123m",
            expected_color: Some(Color::Indexed(123)),
        },
    ];

    for test in tests {
        let mut term = Terminal::new(2, 10);
        let mut parser = Parser::new();
        for &b in test.sequence {
            parser.advance(&mut term, b);
        }
        term.print('A');
        assert_eq!(term.cell(0, 0).style.underline_color, test.expected_color);
    }
}

#[test]
fn malformed_underline_colors_preserve_current_color() {
    let malformed_sequences: &[&[u8]] = &[b"\x1b[58:5:300m", b"\x1b[58;2;10m", b"\x1b[58:9:123m"];

    for &sequence in malformed_sequences {
        let mut term = Terminal::new(1, 2);
        let mut parser = Parser::new();

        for &byte in b"\x1b[58:5:123m" {
            parser.advance(&mut term, byte);
        }

        for &byte in sequence {
            parser.advance(&mut term, byte);
        }

        term.print('A');

        assert_eq!(
            term.cell(0, 0).style.underline_color,
            Some(Color::Indexed(123)),
            "malformed sequence {sequence:?} changed the underline color"
        );
    }
}

#[test]
fn test_reset_underline_color() {
    let mut term = Terminal::new(2, 8);
    let mut parser = Parser::new();

    for &b in b"\x1b[58:5:123m" {
        parser.advance(&mut term, b);
    }
    term.print('A');

    for &b in b"\x1b[59m" {
        parser.advance(&mut term, b);
    }
    term.print('B');

    assert_eq!(term.cell(0, 1).style.underline_color, None);
}

#[test]
fn printed_cells_receive_active_hyperlink() {
    let mut term = Terminal::new(1, 2);
    term.active_hyperlink = Some(HyperlinkId(7));
    term.print('A');

    assert_eq!(term.cell(0, 0).hyperlink, Some(HyperlinkId(7)));
}

#[test]
fn osc_8_reconstructs_uri_containing_semicolon() {
    let mut term = Terminal::new(1, 2);
    let mut parser = Parser::new();

    for &byte in b"\x1b]8;;https://example.com/a;b\x1b\\A" {
        parser.advance(&mut term, byte);
    }

    let hyperlink_id = term
        .cell(0, 0)
        .hyperlink
        .expect("A should have a hyperlink");

    assert_eq!(
        term.hyperlink(hyperlink_id),
        Some("https://example.com/a;b")
    );
}

#[test]
fn sgr_reset_does_not_close_active_hyperlink() {
    let mut term = Terminal::new(1, 2);
    let mut parser = Parser::new();

    for &byte in b"\x1b]8;;https://example.com\x1b\\\x1b[0mA" {
        parser.advance(&mut term, byte);
    }

    let hyperlink_id = term
        .cell(0, 0)
        .hyperlink
        .expect("SGR reset must not close an OSC 8 hyperlink");

    assert_eq!(term.hyperlink(hyperlink_id), Some("https://example.com"));
}

#[test]
fn osc_8_opens_and_closes_hyperlink() {
    let mut term = Terminal::new(1, 4);
    let mut parser = Parser::new();

    let sequence = b"\x1b]8;id=docs;https://example.com\x1b\\A\x1b]8;;\x1b\\B";
    for &byte in sequence {
        parser.advance(&mut term, byte);
    }

    let hyperlink_id = term.cell(0, 0).hyperlink.expect("A should have hyperlink");

    assert_eq!(term.hyperlink(hyperlink_id), Some("https://example.com"));
    assert_eq!(
        term.hyperlinks[hyperlink_id.0].osc_id.as_deref(),
        Some("docs")
    );
}

#[test]
fn new_term_uses_full_screen_as_scroll_region() {
    let term = Terminal::new(4, 3);
    assert_eq!(term.scroll_region, 0..4);
}

#[test]
fn resize_resets_scroll_region_to_full_screen() {
    let mut term = Terminal::new(4, 3);
    term.scroll_region = 1..3;

    term.resize(5, 3).unwrap();

    assert_eq!(term.scroll_region, 0..5);
}

#[test]
fn term_scroll_up_uses_active_scroll_region() {
    let mut term = Terminal::new(4, 2);

    for (row, ch) in ['A', 'B', 'C', 'D'].into_iter().enumerate() {
        term.active_screen_mut().row_mut(row).write_narrow(
            0,
            ch.into(),
            CellStyle::default(),
            None,
        );
    }

    term.scroll_region = 1..3;
    term.dirty.fill(false);
    term.scroll_up();

    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
    assert_eq!(term.cell(1, 0).content, CellContent::Narrow("C".into()));
    assert_eq!(term.cell(2, 0).content, CellContent::Empty);
    assert_eq!(term.cell(3, 0).content, CellContent::Narrow("D".into()));

    assert_eq!(term.dirty, vec![false, true, true, false]);
}

#[test]
fn decstbm_support_partial_region() {
    let mut term = Terminal::new(4, 3);
    let mut parser = Parser::new();

    term.active_screen_mut().cursor_mut().row = 1;
    term.active_screen_mut().cursor_mut().col = 2;

    for &b in b"\x1b[2;4r" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.scroll_region, 1..4);
    assert_eq!(term.active_screen().cursor(), Cursor { row: 0, col: 0 });
}

#[test]
fn decstbm_zero_params_restore_full_region() {
    let mut term = Terminal::new(4, 2);
    let mut parser = Parser::new();

    term.scroll_region = 1..4;

    for &b in b"\x1b[0;0r" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.scroll_region, 0..4);
}

#[test]
fn decstbm_omitted_params_restore_full_region() {
    let mut term = Terminal::new(4, 2);
    let mut parser = Parser::new();
    term.scroll_region = 1..4;

    for &b in b"\x1b[r" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.scroll_region, 0..4);
}

#[test]
fn decstbm_cursor_invalid_scroll_region_is_rejected() {
    let mut term = Terminal::new(6, 4);
    let mut parser = Parser::new();

    term.scroll_region = 1..3;

    term.active_screen_mut().cursor_mut().row = 1;
    term.active_screen_mut().cursor_mut().col = 2;

    for &b in b"\x1b[3;3r" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.scroll_region, 1..3);
    assert_eq!(term.active_screen().cursor(), Cursor { row: 1, col: 2 });

    for &b in b"\x1b[5;2r" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.scroll_region, 1..3);
    assert_eq!(term.active_screen().cursor(), Cursor { row: 1, col: 2 });

    for &b in b"\x1b[2;8r" {
        parser.advance(&mut term, b);
    }

    assert_eq!(term.scroll_region, 1..3);
    assert_eq!(term.active_screen().cursor(), Cursor { row: 1, col: 2 });
}

#[test]
fn decstbm_ignores_csi_r_with_intermediates() {
    let mut term = Terminal::new(6, 4);
    let mut parser = Parser::new();

    term.scroll_region = 1..3;
    term.active_screen_mut().cursor_mut().row = 1;
    term.active_screen_mut().cursor_mut().col = 2;

    for &byte in b"\x1b[?2;4r" {
        parser.advance(&mut term, byte);
    }

    assert_eq!(term.scroll_region, 1..3);
    assert_eq!(term.active_screen().cursor(), Cursor { row: 1, col: 2 });
}

#[test]
fn line_feed_at_bottom_margin_scrolls_region() {
    let mut term = Terminal::new(4, 2);

    for (row, ch) in ['A', 'B', 'C', 'D'].into_iter().enumerate() {
        term.active_screen_mut().row_mut(row).write_narrow(
            0,
            ch.into(),
            CellStyle::default(),
            None,
        );
    }

    term.cursor_mut().row = 2;
    term.cursor_mut().col = 1;
    term.scroll_region = 1..3;
    term.execute(0x0a);

    assert_eq!(term.cell(0, 0).content, CellContent::Narrow("A".into()));
    assert_eq!(term.cell(1, 0).content, CellContent::Narrow("C".into()));
    assert_eq!(term.cell(2, 0).content, CellContent::Empty);
    assert_eq!(term.cell(3, 0).content, CellContent::Narrow("D".into()));

    assert_eq!(term.cursor_row(), 2);
    assert_eq!(term.cursor_col(), 1);
}
