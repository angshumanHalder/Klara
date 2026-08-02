# Klara Architecture Boundaries

This document defines subsystem ownership and communication boundaries. It describes the intended architecture; some current code does not yet satisfy these rules.

## Application

`App` owns resources tied to the operating-system window and application lifetime:

- Configuration.
- OS windows.
- GPU surfaces, devices, and queues.
- Renderer instances.
- Input handler.
- Window and pane manager.

`App` coordinates subsystems but must not implement terminal protocol behavior.

## Window Manager

`WindowManager` owns terminal windows, pane layout trees, and active-pane selection.

Responsibilities:

- Create and remove terminal windows.
- Create, split, close, resize, and focus panes.
- Calculate each pane's pixel rectangle.
- Convert layout changes into pane resize requests.

It must not parse terminal output or render terminal cells.

## Layout

`LayoutNode` owns layout topology and split metadata.

Responsibilities:

- Represent leaf panes and internal splits.
- Calculate pane rectangles.
- Find, insert, remove, and rebalance pane nodes.
- Store adjustable split ratios.

It must not own PTYs, terminal state, GPU resources, or OS windows.

## Pane

`Pane` owns one terminal session:

- PTY master.
- PTY reader and writer.
- Child process handle.
- Terminal state.
- Pane lifecycle state.
- Reader-task/thread coordination.

Responsibilities:

- Spawn the configured shell or command.
- Feed PTY output into terminal parsing.
- Send input bytes to the PTY.
- Resize the terminal state and kernel PTY.
- Observe and reap child-process exit.
- Shut down resources deterministically.

A pane must not render cells or calculate its own pixel layout.

## Terminal State

Terminal state owns protocol-visible state:

- Primary and alternate screen buffers.
- Cursor and saved cursor state.
- Current rendition attributes.
- Terminal modes.
- Scroll regions and tab stops.
- Scrollback and viewport position.
- Dirty-state tracking.

Responsibilities:

- Implement terminal operations independently of rendering.
- Preserve grid invariants.
- Decode no platform input events.
- Hold no window, GPU, PTY, or child-process handles.

`vte::Perform` translates parsed actions into terminal-state operations.

## Renderer

The renderer owns GPU and text-rendering resources:

- Font system and fallback resolution.
- Text atlas and glyph cache.
- Render pipelines and GPU buffers.
- Per-pane render caches.
- Cursor animation timing.

Responsibilities:

- Render terminal snapshots.
- Respect exact cell geometry.
- Invalidate caches after relevant state changes.
- Recover from recoverable surface errors.

The renderer must not mutate terminal protocol state or block PTY processing while shaping text.

## Input

The input subsystem owns input routing and protocol encoding.

Routing order:

```text
platform event
    -> Klara shortcut or prefix handling
    -> local selection or UI action
    -> terminal protocol encoding
    -> active pane
```

Input encoding may inspect terminal modes such as application-cursor or bracketed-paste mode, but must not mutate the grid directly.

## Configuration

Configuration owns parsed and validated user settings.

Responsibilities:

- Deserialize configuration.
- Reject unknown or invalid fields.
- Provide typed, validated values.
- Report actionable errors.
- Eventually resolve platform-specific config locations.

Other subsystems must not parse configuration strings independently when a validated type can be provided.

## Locking Rules

These rules will be enforced during later phases:

- Never hold a terminal-state lock while performing blocking PTY I/O.
- Never hold a terminal-state lock during GPU submission.
- Snapshot dirty terminal data before expensive text shaping.
- Do not acquire pane and grid locks in inconsistent orders.
- Child-process waiting must not block the application event loop.
