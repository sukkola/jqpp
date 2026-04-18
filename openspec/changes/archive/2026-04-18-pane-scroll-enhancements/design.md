## Context

`left_scroll` and `right_scroll` are `u16` offsets on `App` that are passed to ratatui's `Paragraph::scroll()`. The event loop already handles `j`/`k` and arrow keys for scroll. Mouse wheel events arrive as `MouseEventKind::ScrollUp/Down` and are already wired. Pane height is available inside the `terminal.draw()` closure (via the ratatui `Rect` from the layout) but is not currently surfaced to the event loop. ratatui ships a `Scrollbar` / `ScrollbarState` widget that renders a standard scrollbar without additional dependencies.

The event loop is a single `tokio::select!` on terminal events and compute results. Mouse click/drag events arrive as `MouseEventKind::Down`, `MouseEventKind::Drag`, and `MouseEventKind::Up` inside `Event::Mouse`. Pane boundaries (column/row of each pane's scrollbar column) need to be known at event-dispatch time to decide whether a click lands on a scrollbar.

## Goals / Non-Goals

**Goals:**
- Page Up / Page Down scroll by one viewport height in whichever pane has focus
- Home / End jump to scroll offset 0 / max in the focused pane
- Visual scrollbar on right edge of input and output panes using ratatui's `Scrollbar` widget
- Click on scrollbar track → jump to proportional position
- Drag on scrollbar thumb → continuous scroll
- Mouse wheel and trackpad scroll when pointer is inside a pane (extends existing wheel support to the pointer-position case, not just focused-pane)
- All four keyboard actions are remappable via `~/.config/jqpp/config.toml`
- README keybindings table updated

**Non-Goals:**
- Horizontal scrolling
- Scrollbar in the query bar or side menu
- Animated / smooth scrolling
- Scrollbar styling configuration

## Decisions

### D1: Pane height threading

The ratatui layout rects are only available inside `terminal.draw()`. Rather than restructuring the main loop, store `left_pane_height: u16` and `right_pane_height: u16` in `App`. The `ui::draw` function already receives `&mut App`, so it can write these fields during the draw pass. The event handler reads them on the next event cycle — a one-frame lag that is imperceptible.

Alternative considered: pass rects back via a channel. Rejected — adds complexity with no user-visible benefit for a single-threaded UI.

### D2: Scrollbar interaction model

ratatui `Scrollbar` renders a non-interactive widget. Click/drag handling must be implemented manually. Store the scrollbar column for each pane (rightmost column of the pane rect) in `App`. On `MouseEventKind::Down(MouseButton::Left)`, check if `column == scrollbar_col && row` is within the pane's row range. If so, compute the target scroll offset as `(row - pane_top) * content_len / pane_height` and set the scroll. On `MouseEventKind::Drag`, apply the same formula. This gives click-to-position and thumb-drag for free with the same logic.

Alternative considered: track whether the user clicked the thumb specifically and only drag if so. Rejected — click-anywhere-on-track is simpler and matches common UX.

### D3: Four new Action variants

Add `ScrollPageUp`, `ScrollPageDown`, `ScrollToTop`, `ScrollToBottom` to the `Action` enum. Default bindings: `PageUp`, `PageDown`, `Home`, `End`. These are added to `Action::all()` and `Action::name()` so they appear in `--print-config` output and are remappable via config.

### D4: Content line count for max scroll

The max scroll offset must be capped so the last line stays visible. Compute `content_lines` as `app.results.len()` (for right pane, after rendering) or the line count of the raw JSON for the left pane. Store `left_content_lines: usize` and `right_content_lines: usize` in `App`. `ui::draw` updates these during the draw pass alongside pane heights.

## Risks / Trade-offs

- [One-frame lag on pane height] Layout rects are written by draw and read by the next event. On the very first Page Down before any frame is rendered, height is 0. → Mitigation: default `left_pane_height` / `right_pane_height` to a sensible fallback (e.g. 20) in `App::new()`.
- [Scrollbar column overlap] The scrollbar takes the rightmost column of a pane. ratatui `Paragraph::wrap` will reflow content to avoid that column only if the inner area is shrunk by 1. → Mitigation: render `Paragraph` into `pane_rect.inner(Margin { horizontal: 1, vertical: 0 })` so text never obscures the scrollbar.
- [Drag on non-scrollbar columns] Mouse drag events fire for any drag, not just scrollbar drags. → Mitigation: track `drag_target: Option<DragTarget>` in `App`; only process drag as scroll if set on `MouseDown` on a scrollbar column.
