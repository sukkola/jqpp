## 1. App State & Keymap

- [x] 1.1 Add `ScrollPageUp`, `ScrollPageDown`, `ScrollToTop`, `ScrollToBottom` variants to `Action` enum in `src/keymap.rs`
- [x] 1.2 Add the four new actions to `Action::all()` and `Action::name()` in `src/keymap.rs`
- [x] 1.3 Add the four new actions to `Action::from_toml_name()` in `src/keymap.rs`
- [x] 1.4 Set default bindings in `default_keymap()`: `PageUp`, `PageDown`, `Home`, `End`
- [x] 1.5 Add `left_pane_height: u16`, `right_pane_height: u16`, `left_content_lines: usize`, `right_content_lines: usize` fields to `App` with sensible defaults (height=20, lines=0)
- [x] 1.6 Add `drag_target: Option<DragTarget>` field to `App` where `DragTarget` is an enum `{ LeftScrollbar, RightScrollbar }` in `src/app.rs`
- [x] 1.7 Add `left_scrollbar_col: u16`, `right_scrollbar_col: u16`, `left_pane_top: u16`, `right_pane_top: u16` fields to `App` so the event handler can hit-test mouse clicks

## 2. UI — Scrollbar Rendering

- [x] 2.1 In `src/ui.rs`, shrink the input/output pane inner rect by 1 column on the right (`Margin { horizontal: 1, vertical: 0 }`) so text does not overlap the scrollbar column
- [x] 2.2 After rendering each `Paragraph`, render a ratatui `Scrollbar` (vertical, right side) into the full pane rect using `ScrollbarState` built from `content_lines` and `scroll_offset`
- [x] 2.3 Only render the scrollbar when `content_lines > pane_height` (hide when content fits)
- [x] 2.4 Write `app.left_pane_height`, `app.right_pane_height`, `app.left_content_lines`, `app.right_content_lines`, `app.left_scrollbar_col`, `app.right_scrollbar_col`, `app.left_pane_top`, `app.right_pane_top` from within `ui::draw` after computing the layout rects
- [x] 2.5 Compute `left_content_lines` as line count of the raw JSON string (split on `\n`); compute `right_content_lines` as the number of rendered result lines

## 3. Event Handling — Keyboard

- [x] 3.1 In the `_ =>` pane-branch of the keyboard handler in `src/main.rs`, add handling for `ScrollPageDown`: increment scroll by `pane_height`, clamped to `content_lines.saturating_sub(pane_height)`
- [x] 3.2 Add handling for `ScrollPageUp`: decrement scroll by `pane_height`, saturating at 0
- [x] 3.3 Add handling for `ScrollToTop`: set scroll to 0
- [x] 3.4 Add handling for `ScrollToBottom`: set scroll to `content_lines.saturating_sub(pane_height)`
- [x] 3.5 Apply the same max-clamp to existing single-line `ScrollDown` and arrow-down handlers so they cannot scroll past the last line

## 4. Event Handling — Mouse

- [x] 4.1 Extend `MouseEventKind::ScrollDown` / `ScrollUp` handlers to check pointer column/row and route to the pane under the cursor (not just the focused pane)
- [x] 4.2 Handle `MouseEventKind::Down(MouseButton::Left)`: if `column == app.left_scrollbar_col` and row within left pane range, compute and set `left_scroll`; set `app.drag_target = Some(DragTarget::LeftScrollbar)`
- [x] 4.3 Same for right pane: set `right_scroll` and `app.drag_target = Some(DragTarget::RightScrollbar)`
- [x] 4.4 Handle `MouseEventKind::Drag(MouseButton::Left)`: if `drag_target` is set, apply the same proportional formula to update the relevant scroll field
- [x] 4.5 Handle `MouseEventKind::Up(MouseButton::Left)`: clear `app.drag_target`
- [x] 4.6 Clamp scroll values set by mouse click/drag to valid range `[0, content_lines.saturating_sub(pane_height)]`

## 5. Documentation

- [x] 5.1 Add `Page Up`, `Page Down`, `Home`, `End` rows to the keybindings table in `README.md`
- [x] 5.2 Add a note in the README about scrollbar interaction (click/drag/wheel)

## 6. Tests

- [x] 6.1 Add unit tests in `src/app.rs` for `ScrollPageDown` clamping, `ScrollToTop`, and `ScrollToBottom` logic
- [x] 6.2 Add PTY keyboard tests in `tests/keyboard_tests.rs` for Page Down and Home in right pane (smoke: no crash, clean Ctrl+C exit)
- [x] 6.3 Add unit test verifying `right_content_lines` and `right_pane_height` produce correct max scroll offset
