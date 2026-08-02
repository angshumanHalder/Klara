# Klara Production Hardening Roadmap

This document tracks the work required to turn Klara's current prototype into a reliable, production-grade terminal emulator. Work proceeds from the lowest-level correctness guarantees toward user-facing features.

## Working Rules

- Target Apple-silicon Macs running macOS 15 or newer.
- Treat macOS 26.5.2 as the primary development environment, not the only valid environment.
- Keep terminal state independent of platform integration so Linux can be considered later without redesigning the emulator core.
- Fix terminal correctness before adding product features.
- Add a regression test before or alongside every bug fix.
- Keep terminal state independent of windowing and rendering.
- Avoid advertising terminal capabilities that Klara does not implement.
- Prefer correctness and clear ownership before performance optimization.
- Do not silently discard PTY, rendering, configuration, or process errors.

## Production and Release Policy

- Production-grade means correct terminal behavior, reliable process ownership, bounded resource use, reproducible source builds, and documented compatibility.
- A clean checkout must build without fonts, configuration, or resources unique to the development machine.
- macOS 15 support remains best-effort until a release build passes in a clean macOS 15 environment.
- Initial public releases are source builds and use no recurring paid developer services.
- Developer ID signing, notarization, DMG packaging, Homebrew distribution, automatic updates, Intel Macs, and the Mac App Store are outside the initial release.

## Target Architecture

Subsystem responsibilities should remain explicit:

- **Terminal state:** cells, cursor, modes, attributes, scrollback, and VT operations.
- **Pane:** PTY ownership, child lifecycle, terminal state, input, and resize propagation.
- **Layout:** maps panes to pixel rectangles and manages split topology.
- **Renderer:** converts immutable terminal snapshots into pixels.
- **Input:** translates platform events into Klara actions or terminal protocol input.
- **Application:** owns windows, GPU surfaces, configuration, and subsystem coordination.

## Phase 1: Establish a Safe Baseline

- [x] Run and record the existing test-suite baseline.
- [x] Add regression tests before fixing known behavior.
- [ ] Define error types for terminal, PTY, renderer, and configuration failures.
- [ ] Remove ambiguous ownership between pane, grid, layout, and renderer.
- [ ] Document currently supported terminal capabilities.

### Acceptance criteria

- Existing behavior is covered well enough to detect regressions during restructuring.
- Each mutable resource has a clear owner.
- Recoverable failures do not require `unwrap` or `expect` in application paths.

## Phase 2: PTY Ownership and Resize Propagation

- [ ] Retain the PTY master in `Pane`.
- [ ] Retain and reap the child process handle.
- [ ] Represent pane lifecycle state: running, exited, and failed.
- [ ] Report PTY read and write failures.
- [ ] Add managed reader-thread shutdown or cancellation.
- [ ] Implement `Pane::resize(rows, cols, pixel_width, pixel_height)`.
- [ ] Resize both primary and alternate terminal buffers.
- [ ] Call `MasterPty::resize` after pane dimensions change.
- [ ] Resize every existing pane after an OS-window resize.
- [ ] Resize the original and new panes after a split.
- [ ] Ignore or safely defer zero-sized window resize events.
- [ ] Mark terminal and renderer state dirty after resizing.

### Tests

- [ ] Growing a grid preserves existing cells.
- [ ] Shrinking a grid preserves visible cells and clamps the cursor.
- [ ] Primary and alternate buffers remain dimensionally consistent.
- [ ] Splitting resizes the original pane as well as the new pane.
- [ ] Window resizing propagates dimensions to every pane.
- [ ] Zero-sized windows do not produce invalid PTY sizes.
- [ ] Child exit is observable and the child is reaped.

### Acceptance criteria

- The application inside every pane always agrees with Klara about rows and columns.
- Resizing reliably produces the platform-equivalent terminal resize notification.
- Closing a pane or Klara does not leak child processes, threads, or file descriptors.

## Phase 3: Correct the Terminal Data Model

- [ ] Replace the one-`char` cell assumption with explicit cell content.
- [ ] Represent empty, narrow, wide-leading, and wide-continuation cells.
- [ ] Add terminal attributes: bold, dim, italic, underline, strikeout, reverse, hidden, and blink.
- [ ] Add underline color and style.
- [ ] Add hyperlink metadata.
- [ ] Store current rendition state separately from cells.
- [ ] Model primary and alternate screen buffers explicitly.
- [ ] Store cursor state and saved cursor states.
- [ ] Add scroll margins.
- [ ] Add terminal mode flags.
- [ ] Add tab stops.
- [ ] Add pending-autowrap state.
- [ ] Add bounded scrollback history and viewport offset.

### Acceptance criteria

- The grid can represent modern Unicode output and all required visual attributes.
- Editing or erasing a cell cannot leave an invalid wide-character fragment.
- Primary screen history and alternate-screen behavior are independent.

## Phase 4: Repair Core Terminal Semantics

Implement these operations as independently testable terminal-state methods. The `vte::Perform` implementation should decode and delegate rather than contain complex mutation logic.

- [ ] Pending autowrap.
- [ ] Line feed, carriage return, backspace, and horizontal tab.
- [ ] Save and restore cursor/state.
- [ ] Index, next line, and reverse index.
- [ ] Absolute and relative cursor positioning.
- [ ] Erase using the correct rendition/background.
- [ ] Insert and delete characters.
- [ ] Insert and delete lines.
- [ ] Scroll up and down.
- [ ] Top and bottom scroll margins.
- [ ] Origin mode.
- [ ] Insert mode.
- [ ] Primary and alternate screen transitions.
- [ ] Full SGR attributes and indexed/truecolor support.
- [ ] Multiple mode parameters in one sequence.
- [ ] Device status and capability replies.
- [ ] Terminal reset behavior.
- [ ] Required ESC sequences.
- [ ] Required OSC commands.
- [ ] Required DCS commands and synchronized updates.

### Acceptance criteria

- Common shells and TUIs do not corrupt their display during normal editing, scrolling, or resize.
- Multi-parameter sequences are applied completely.
- Repeated mode transitions are idempotent where the protocol requires it.

## Phase 5: Unicode Correctness

- [ ] Determine terminal display width using a maintained Unicode implementation.
- [ ] Attach combining marks without advancing the cursor.
- [ ] Handle double-width characters and continuation cells.
- [ ] Handle variation selectors and zero-width joiners.
- [ ] Handle regional-indicator flags and emoji sequences.
- [ ] Define an ambiguous-width policy.
- [ ] Make erase, insert, delete, wrap, resize, and selection wide-cell safe.
- [ ] Keep ligature shaping separate from terminal column width.

### Tests

- [ ] ASCII and control cases.
- [ ] CJK double-width characters.
- [ ] Combining accents.
- [ ] Emoji variation selectors.
- [ ] Zero-width-joiner emoji.
- [ ] Regional-indicator flags.
- [ ] Wide characters at the final column.
- [ ] Overwriting or erasing either half of a wide character.

### Acceptance criteria

- Grid columns remain stable regardless of the shaped glyph sequence.
- Unicode editing never leaves corrupted cells or an incorrect cursor position.

## Phase 6: Rebuild Rendering Around Shared Metrics

- [ ] Introduce one authoritative `CellMetrics` structure.
- [ ] Use shared metrics for grid sizing, layout, rendering, hit testing, and cursor drawing.
- [ ] Remove conflicting hardcoded cell dimensions.
- [ ] Wire configured font family and size into the renderer.
- [ ] Provide graceful monospace and emoji fallback fonts.
- [ ] Handle DPI and scale-factor changes.
- [ ] Position shaped runs against exact terminal columns.
- [ ] Enable advanced shaping and ligatures without breaking column alignment.
- [ ] Render wide characters and combining sequences correctly.
- [ ] Render all supported cell attributes.
- [ ] Render block, beam, and underline cursors.
- [ ] Add focus-aware cursor behavior and blinking.
- [ ] Invalidate caches after grid, layout, font, theme, or DPI changes.
- [ ] Allocate or grow GPU buffers based on actual demand.
- [ ] Recover from outdated or lost GPU surfaces.
- [ ] Handle out-of-memory failures explicitly.
- [ ] Configure transparency through a supported composite alpha mode.

### Acceptance criteria

- Terminal cells, backgrounds, glyphs, cursor, and hit testing share identical geometry.
- Resizing, splitting, and DPI changes cannot display stale cached positions.
- Missing configured fonts fall back cleanly instead of crashing.

## Phase 7: Complete Input and Interaction

- [ ] Implement complete control-character mappings.
- [ ] Implement Alt/Meta encoding.
- [ ] Encode modified navigation and function keys.
- [ ] Support application cursor and keypad modes.
- [ ] Handle IME preedit and commit events.
- [ ] Support bracketed paste.
- [ ] Implement focus reporting.
- [ ] Implement required mouse tracking protocols.
- [ ] Implement local selection and autoscroll.
- [ ] Implement native clipboard copy and paste.
- [ ] Add a configurable OSC 52 security policy.
- [ ] Consider a modern extended keyboard protocol after core compatibility.
- [ ] Make the Klara prefix and bindings configurable.

### Input routing

```text
platform event
    -> Klara shortcut or prefix handling
    -> selection or UI action
    -> terminal protocol encoding
    -> active pane
```

### Acceptance criteria

- Shell editing, Neovim, tmux, and mouse-aware TUIs receive correct input sequences.
- CJK and composed input work through the operating system IME.
- Pasting cannot accidentally bypass bracketed-paste behavior.

## Phase 8: Process and Error Resilience

- [ ] Monitor and reap child processes.
- [ ] Close panes when configured to do so after child exit.
- [ ] Implement graceful termination followed by bounded forced termination.
- [ ] Make reader tasks cancellable and observable.
- [ ] Surface pane failures to the user.
- [ ] Replace application-path panics with contextual error handling.
- [ ] Add structured logging and useful diagnostics.
- [ ] Recover from transient rendering and surface failures.
- [ ] Define deterministic application shutdown ordering.

### Acceptance criteria

- Klara exits without zombies, leaked threads, or stuck PTYs.
- A failed pane or recoverable GPU error does not crash unrelated panes.
- Errors contain enough context to diagnose the affected subsystem.

## Phase 9: Compatibility and Performance Validation

- [ ] Add table-driven CSI, ESC, OSC, and DCS fixtures.
- [ ] Add PTY integration tests using real child processes.
- [ ] Add Neovim, tmux, fzf, shell, and prompt smoke tests.
- [ ] Add differential or fixture-based compatibility tests.
- [ ] Fuzz arbitrary parser byte streams.
- [ ] Fuzz resize and mode-transition sequences.
- [ ] Add property tests for cursor, grid, and wide-cell invariants.
- [ ] Test rendering across fonts, fallback chains, and DPI scales.
- [ ] Test resize storms and rapid pane operations.
- [ ] Benchmark parser throughput, allocations, lock contention, and frame time.
- [ ] Profile before changing data structures for performance.
- [ ] Build a clean checkout for `aarch64-apple-darwin` with a macOS 15 deployment target.
- [ ] Run the release build in a clean macOS 15 environment before declaring macOS 15 verified.
- [ ] Test Metal rendering, DPI, input, PTY lifecycle, and shutdown on the primary physical Mac.

### Acceptance criteria

- Compatibility regressions are caught automatically.
- Arbitrary input cannot panic or violate grid invariants.
- Performance work is supported by repeatable benchmarks and profiles.

## Phase 10: Pane and Window Features

Begin this phase only after the terminal foundation is stable.

- [ ] Focus panes by direction and mouse selection.
- [ ] Resize pane split ratios.
- [ ] Close panes and rebalance the layout tree.
- [ ] Zoom and restore panes.
- [ ] Enforce sensible minimum pane dimensions.
- [ ] Implement numbered terminal windows.
- [ ] Implement the status line and pane borders.
- [ ] Add scrollback navigation and search.
- [ ] Add dynamic titles and shell integration.

## Phase 11: Configuration and UI Refinements

- [ ] Load configuration from platform-appropriate locations.
- [ ] Validate configuration and report invalid values.
- [ ] Reject unknown fields where appropriate.
- [ ] Support live configuration reload.
- [ ] Wire all declared theme, font, opacity, and blur settings.
- [ ] Add configurable themes and font fallback.
- [ ] Add background images and opacity overlays.
- [ ] Add accessibility and screen-reader considerations.

## Phase 12: AI Agent Overlay

Start the overlay after terminal correctness, interaction, and process lifecycle are dependable.

- [ ] Define the agent lifecycle separately from PTY pane lifecycle.
- [ ] Implement an input overlay with explicit focus routing.
- [ ] Stream API-agent output with cancellation and error reporting.
- [ ] Manage CLI-agent subprocesses safely.
- [ ] Render scrollable and collapsible output panels.
- [ ] Prevent overlays from corrupting terminal input, selection, or IME state.

## Immediate Milestone

The first coding milestone is **PTY ownership plus end-to-end resize propagation**:

1. Retain PTY master and child handles.
2. Add explicit pane lifecycle and errors.
3. Implement safe terminal-buffer resizing.
4. Resize the kernel PTY.
5. Propagate layout changes to every pane.
6. Add regression and lifecycle tests.

Completing this milestone gives every later terminal and renderer change trustworthy pane dimensions and process ownership.
