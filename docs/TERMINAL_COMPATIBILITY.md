# Klara Terminal Compatibility

Klara is currently a prototype and does not yet provide complete xterm compatibility.

This document records implemented behavior and known gaps. It must be updated whenever terminal protocol support changes.

## Supported Platform

- Apple-silicon Macs (`aarch64-apple-darwin`).
- macOS 15 or newer.
- Primarily developed and tested on macOS 26.5.2.

Compatibility with macOS 15 is best-effort until the release build has passed in
a clean macOS 15 environment. Intel Macs and other operating systems are not
supported by the initial release.

## Advertised Environment

Klara currently sets:

```text
TERM=xterm-256color
COLORTERM=truecolor
TERM_PROGRAM=klara
```

This overstates current compatibility. Applications may emit xterm sequences that Klara does not implement.

Before production release, Klara must either:

1. Reach the required xterm compatibility level, or
2. Ship an accurate Klara terminfo entry.

## Currently Implemented

### Printable output

- UTF-8 decoding through `vte`.
- One Rust `char` stored per cell.
- Basic line wrapping.
- Full-screen upward scrolling.

Current character storage is not yet correct for wide or combining characters.

### C0 controls

- Line feed.
- Vertical tab treated as line feed.
- Form feed treated as line feed.
- Carriage return.
- Backspace.

### Cursor movement

- Cursor up: `CSI Ps A`.
- Cursor down: `CSI Ps B`.
- Cursor forward: `CSI Ps C`.
- Cursor backward: `CSI Ps D`.
- Horizontal absolute: `CSI Ps G`.
- Cursor position: `CSI Ps ; Ps H`.
- Horizontal/vertical position: `CSI Ps ; Ps f`.

### Erasing

- Erase in display: `CSI Ps J`.
- Erase in line: `CSI Ps K`.

Erase behavior does not yet fully match xterm rendition semantics.

### Colors

- Standard foreground colors.
- Standard background colors.
- Bright foreground colors.
- Bright background colors.
- Indexed 256-color foreground and background.
- RGB foreground and background.
- Default foreground and background reset.

Other SGR attributes are not implemented.

### Screen modes

- Cursor visibility.
- Application cursor-key mode.
- SGR mouse-reporting flag.
- Alternate screen mode using private mode 1049.

Mode handling currently processes only the first parameter in a sequence.

### Cursor style

- Block.
- Underline.
- Bar.

Cursor state is parsed but not currently rendered.

### Keyboard input

- Text from `winit` key events.
- Enter, Backspace, Space, Tab, and Escape.
- Arrow keys with normal and application-cursor variants.
- Home, End, Page Up, Page Down, Insert, and Delete.
- Function keys F1 through F12.
- Control plus ASCII alphabetic keys.

### Pane layout

- Binary-tree vertical splits.
- Binary-tree horizontal splits.
- Equal-size split calculation.

## Known Unsupported Behavior

- Correct delayed autowrap.
- Wide characters and continuation cells.
- Combining characters and grapheme clusters.
- Scrollback history.
- Scroll regions.
- Origin and insert modes.
- Insert/delete characters.
- Insert/delete lines.
- Tab stops.
- Save/restore cursor and rendition state.
- Index, reverse index, and next-line ESC commands.
- Complete SGR attributes.
- OSC commands.
- DCS commands.
- Device status and capability replies.
- Bracketed paste.
- Focus reporting.
- Full mouse tracking and event encoding.
- Application keypad mode.
- Synchronized updates.
- Hyperlinks.
- Clipboard and OSC 52.
- Terminal reset behavior.
- Dynamic title and working-directory reporting.
- PTY resize propagation.
- Grid resize and reflow.
- Child-process lifecycle management.
- Cursor rendering.
- IME input.
- Extended keyboard protocols.

## Compatibility Targets

The first compatibility targets are:

1. Interactive POSIX shells.
2. Readline and ZLE editing.
3. Neovim.
4. tmux.
5. fzf.
6. Common prompts and command-line completion interfaces.

A target is not considered supported until it has automated smoke tests or recorded compatibility fixtures.
